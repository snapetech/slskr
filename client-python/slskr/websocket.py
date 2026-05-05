"""
WebSocket client for real-time events
"""

import asyncio
import json
from typing import Callable, Dict, Set, Optional
import aiohttp


class WebSocketClient:
    """WebSocket client for real-time events"""

    def __init__(self, base_url: str, token: str, debug: bool = False):
        self.url = base_url.replace("http", "ws").rstrip("/") + "/api/events/ws"
        self.token = token
        self.debug = debug
        self.session: Optional[aiohttp.ClientSession] = None
        self.ws: Optional[aiohttp.ClientWebSocketResponse] = None
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
        try:
            headers = {"Authorization": f"Bearer {self.token}"}
            self.session = aiohttp.ClientSession()

            self.ws = await self.session.ws_connect(
                self.url,
                headers=headers,
                autoclose=False,
            )

            self.reconnect_attempts = 0
            self._notify_connection_listeners(True)

            # Start message handler
            asyncio.create_task(self._handle_messages())

        except Exception as e:
            if self.session:
                await self.session.close()
                self.session = None
            self._notify_error_listeners(e)
            raise

    async def disconnect(self):
        """Disconnect from WebSocket"""
        if self.ws:
            await self.ws.close()
            self.ws = None
        if self.session:
            await self.session.close()
            self.session = None
        self._notify_connection_listeners(False)

    async def _handle_messages(self):
        """Handle incoming messages"""
        try:
            async for msg in self.ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    await self._process_message(msg.data)
                elif msg.type in (
                    aiohttp.WSMsgType.ERROR,
                    aiohttp.WSMsgType.CLOSED,
                ):
                    break
        except Exception as e:
            self._notify_error_listeners(e)

    async def _process_message(self, data: str):
        """Process incoming message"""
        try:
            event = json.loads(data)

            # Emit to listeners
            event_type = event.get("type")
            if event_type in self.event_listeners:
                for listener in self.event_listeners[event_type]:
                    try:
                        await listener(event)
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

        if self.ws and not self.ws.closed:
            message = {"type": "subscribe", "data": {"topics": list(new_topics)}}
            asyncio.create_task(self.ws.send_json(message))

    def unsubscribe(self, *topics: str):
        """Unsubscribe from event types"""
        for topic in topics:
            self.subscribed_topics.discard(topic)

        if self.ws and not self.ws.closed:
            message = {"type": "unsubscribe", "data": {"topics": list(topics)}}
            asyncio.create_task(self.ws.send_json(message))

    def on(self, event_type: str, listener: Callable) -> Callable:
        """Listen to event type"""
        if event_type not in self.event_listeners:
            self.event_listeners[event_type] = set()

        self.event_listeners[event_type].add(listener)

        # Return unsubscribe function
        def unsubscribe():
            self.event_listeners[event_type].discard(listener)

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
        for listener in self.connection_listeners:
            try:
                asyncio.create_task(self._call_listener(listener, connected))
            except Exception as e:
                print(f"Error in connection listener: {e}")

    def _notify_error_listeners(self, error: Exception):
        """Notify error listeners"""
        for listener in self.error_listeners:
            try:
                asyncio.create_task(self._call_listener(listener, error))
            except Exception as e:
                print(f"Error in error listener: {e}")

    async def _call_listener(self, listener: Callable, *args):
        """Call listener function"""
        if asyncio.iscoroutinefunction(listener):
            await listener(*args)
        else:
            listener(*args)
