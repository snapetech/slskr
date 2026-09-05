"""
WebSocket client for real-time events
"""

import asyncio
import inspect
import json
import logging
from typing import Any, Callable, Dict, Set, Optional
from urllib.parse import urlsplit, urlunsplit
import aiohttp

MAX_WEBSOCKET_MESSAGE_BYTES = 64 * 1024
DEFAULT_WEBSOCKET_CONNECT_TIMEOUT = 30.0
logger = logging.getLogger(__name__)


class WebSocketClient:
    """WebSocket client for real-time events"""

    def __init__(
        self,
        base_url: str,
        token: str,
        debug: bool = False,
        connect_timeout: float = DEFAULT_WEBSOCKET_CONNECT_TIMEOUT,
    ):
        parsed_url = urlsplit(base_url)
        if (
            parsed_url.scheme not in ("http", "https")
            or not parsed_url.netloc
            or parsed_url.username is not None
            or parsed_url.password is not None
        ):
            raise ValueError(
                "base_url must be an absolute HTTP or HTTPS URL without credentials"
            )
        websocket_scheme = "wss" if parsed_url.scheme == "https" else "ws"
        websocket_path = parsed_url.path.rstrip("/") + "/api/events/ws"
        self.url = urlunsplit(
            (websocket_scheme, parsed_url.netloc, websocket_path, "", "")
        )
        self.token = token
        self.debug = debug
        if connect_timeout <= 0:
            raise ValueError("connect_timeout must be greater than zero")
        self.connect_timeout = connect_timeout
        self.session: Optional[aiohttp.ClientSession] = None
        self.ws: Optional[aiohttp.ClientWebSocketResponse] = None
        self._connect_lock = asyncio.Lock()
        self._message_task: Optional[asyncio.Task] = None
        self._reconnect_task: Optional[asyncio.Task] = None
        self._outbound_tasks: Set[asyncio.Task] = set()
        self._intentional_disconnect = False
        self.subscribed_topics: Set[str] = set()

        # Listeners
        self.event_listeners: Dict[str, Set[Callable]] = {}
        self.connection_listeners: Set[Callable] = set()
        self.error_listeners: Set[Callable] = set()

        self.reconnect_attempts = 0
        self.max_reconnect_attempts = 5
        self.reconnect_delay = 1

    async def connect(self):
        """Connect to WebSocket"""
        current_task = asyncio.current_task()
        reconnect_task = self._reconnect_task
        if reconnect_task is not None and reconnect_task is not current_task:
            reconnect_task.cancel()
            self._reconnect_task = None
        async with self._connect_lock:
            if self.is_connected():
                raise RuntimeError("WebSocket is already connected")
            self._intentional_disconnect = True
            await self._close_resources()
            self._intentional_disconnect = False
            try:
                websocket_options = {
                    "autoclose": False,
                    "max_msg_size": MAX_WEBSOCKET_MESSAGE_BYTES,
                }
                if self.token:
                    websocket_options["headers"] = {
                        "Authorization": f"Bearer {self.token}"
                    }
                session = aiohttp.ClientSession()
                self.session = session

                self.ws = await asyncio.wait_for(
                    session.ws_connect(
                        self.url,
                        **websocket_options,
                    ),
                    timeout=self.connect_timeout,
                )

                if self.subscribed_topics:
                    await asyncio.wait_for(
                        self.ws.send_json(
                            {
                                "type": "subscribe",
                                "data": {"topics": sorted(self.subscribed_topics)},
                            }
                        ),
                        timeout=self.connect_timeout,
                    )

                self.reconnect_attempts = 0
                self._notify_connection_listeners(True)

                # Start message handler
                self._message_task = asyncio.create_task(self._handle_messages(self.ws))

            except asyncio.CancelledError:
                await self._close_resources()
                raise
            except Exception as e:
                await self._close_resources()
                self._notify_error_listeners(e)
                raise

    async def disconnect(self):
        """Disconnect from WebSocket"""
        self._intentional_disconnect = True
        current_task = asyncio.current_task()
        reconnect_task = self._reconnect_task
        if reconnect_task is not None and reconnect_task is not current_task:
            reconnect_task.cancel()
            self._reconnect_task = None
        async with self._connect_lock:
            was_connected = self.is_connected()
            await self._close_resources()
            if was_connected:
                self._notify_connection_listeners(False)

    async def _handle_messages(self, ws: aiohttp.ClientWebSocketResponse):
        """Handle incoming messages"""
        try:
            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    await self._process_message(msg.data)
                elif msg.type in (
                    aiohttp.WSMsgType.ERROR,
                    aiohttp.WSMsgType.CLOSED,
                ):
                    break
        except Exception as e:
            if not self._intentional_disconnect:
                self._notify_error_listeners(e)
        finally:
            if self.ws is ws:
                self.ws = None
                if self.session:
                    await self.session.close()
                    self.session = None
                self._message_task = None
                if not self._intentional_disconnect:
                    self._notify_connection_listeners(False)
                    self._schedule_reconnect()

    def _schedule_reconnect(self):
        """Start a bounded reconnect loop after an unexpected close."""
        if (
            self._intentional_disconnect
            or self.max_reconnect_attempts <= 0
            or (
                self._reconnect_task is not None
                and not self._reconnect_task.done()
            )
        ):
            return
        self._reconnect_task = asyncio.create_task(self._reconnect_loop())

    async def _reconnect_loop(self):
        """Retry a dropped connection with exponential backoff."""
        current_task = asyncio.current_task()
        try:
            attempt_limit = min(max(0, self.max_reconnect_attempts), 32)
            for attempt in range(1, attempt_limit + 1):
                if self._intentional_disconnect:
                    return
                self.reconnect_attempts = attempt
                delay = min(
                    max(0, self.reconnect_delay) * (2 ** min(attempt - 1, 16)),
                    30,
                )
                if delay:
                    await asyncio.sleep(delay)
                if self._intentional_disconnect:
                    return
                try:
                    await self.connect()
                    return
                except asyncio.CancelledError:
                    raise
                except Exception:
                    # connect() notifies the registered error listeners; keep
                    # retrying without duplicating that notification.
                    continue
        finally:
            if self._reconnect_task is current_task:
                self._reconnect_task = None

    async def _close_resources(self):
        outbound_tasks = list(self._outbound_tasks)
        self._outbound_tasks.clear()
        for outbound in outbound_tasks:
            outbound.cancel()
        if outbound_tasks:
            await asyncio.gather(*outbound_tasks, return_exceptions=True)
        task = self._message_task
        self._message_task = None
        if task and task is not asyncio.current_task():
            task.cancel()
            try:
                await task
            except asyncio.CancelledError:
                pass
        if self.ws:
            await self.ws.close()
            self.ws = None
        if self.session:
            await self.session.close()
            self.session = None

    async def _process_message(self, data: str):
        """Process incoming message"""
        try:
            event = json.loads(data)

            # Emit to listeners
            event_type = event.get("type")
            if event_type in self.event_listeners:
                for listener in tuple(self.event_listeners[event_type]):
                    try:
                        await self._call_listener(listener, event)
                    except Exception as e:
                        self._notify_error_listeners(e)
        except Exception as e:
            self._notify_error_listeners(e)

    def subscribe(self, *topics: str):
        """Subscribe to event types"""
        new_topics = set(topics) - self.subscribed_topics
        if not new_topics:
            return

        for topic in new_topics:
            self.subscribed_topics.add(topic)

        ws = self.ws
        if ws and not ws.closed:
            message = {"type": "subscribe", "data": {"topics": list(new_topics)}}
            self._schedule_send(message, "subscribe", new_topics, ws)

    def unsubscribe(self, *topics: str):
        """Unsubscribe from event types"""
        removed_topics = set(topics) & self.subscribed_topics
        if not removed_topics:
            return
        self.subscribed_topics.difference_update(removed_topics)

        ws = self.ws
        if ws and not ws.closed:
            message = {"type": "unsubscribe", "data": {"topics": list(removed_topics)}}
            self._schedule_send(message, "unsubscribe", removed_topics, ws)

    def _schedule_send(
        self,
        message: Dict,
        transition: str,
        topics: Set[str],
        ws: aiohttp.ClientWebSocketResponse,
    ):
        task = asyncio.create_task(ws.send_json(message))
        self._outbound_tasks.add(task)

        def finished(completed: asyncio.Task):
            self._outbound_tasks.discard(completed)
            if completed.cancelled():
                return
            error = completed.exception()
            if error is not None and not self._intentional_disconnect:
                # A send can finish after this socket has been replaced or
                # closed.  In that case the desired topic set is replayed by
                # the next handshake and must not be rolled back based on a
                # stale connection's failure.
                if self.ws is ws:
                    if transition == "subscribe":
                        self.subscribed_topics.difference_update(topics)
                    else:
                        self.subscribed_topics.update(topics)
                self._notify_error_listeners(error)

        task.add_done_callback(finished)

    def on(self, event_type: str, listener: Callable) -> Callable:
        """Listen to event type"""
        if event_type not in self.event_listeners:
            self.event_listeners[event_type] = set()

        self.event_listeners[event_type].add(listener)

        # Return unsubscribe function
        def unsubscribe():
            self.event_listeners.get(event_type, set()).discard(listener)

        return unsubscribe

    def on_connection_change(self, listener: Callable) -> Callable:
        """Listen to connection state changes"""
        self.connection_listeners.add(listener)

        def unsubscribe():
            self.connection_listeners.discard(listener)

        return unsubscribe

    def on_error(self, listener: Callable) -> Callable:
        """Listen to errors"""
        self.error_listeners.add(listener)

        def unsubscribe():
            self.error_listeners.discard(listener)

        return unsubscribe

    def is_connected(self) -> bool:
        """Check if connected"""
        return self.ws is not None and not self.ws.closed

    def get_subscribed_topics(self) -> list:
        """Get subscribed topics"""
        return list(self.subscribed_topics)

    def remove_all_listeners(self):
        """Remove all listeners"""
        self.event_listeners.clear()
        self.connection_listeners.clear()
        self.error_listeners.clear()

    # =========================================================================
    # Private Methods
    # =========================================================================

    def _notify_connection_listeners(self, connected: bool):
        """Notify connection listeners"""
        for listener in tuple(self.connection_listeners):
            self._schedule_listener(listener, connected, report_errors=True)

    def _notify_error_listeners(self, error: Exception):
        """Notify error listeners"""
        for listener in tuple(self.error_listeners):
            self._schedule_listener(listener, error, report_errors=False)

    def _schedule_listener(self, listener: Callable, argument: Any, report_errors: bool):
        try:
            task = asyncio.create_task(self._call_listener(listener, argument))
        except Exception as error:
            self._handle_listener_error(error, report_errors)
            return
        task.add_done_callback(
            lambda completed: self._finish_listener(completed, report_errors)
        )

    def _finish_listener(self, task: asyncio.Task, report_errors: bool):
        if task.cancelled():
            return
        try:
            error = task.exception()
        except Exception as exception:
            error = exception
        if error is not None:
            self._handle_listener_error(error, report_errors)

    def _handle_listener_error(self, error: Exception, report_errors: bool):
        if report_errors:
            self._notify_error_listeners(error)
        else:
            logger.error("WebSocket error listener failed: %s", error)

    async def _call_listener(self, listener: Callable, *args):
        """Call listener function"""
        if inspect.iscoroutinefunction(listener):
            await listener(*args)
        else:
            listener(*args)
