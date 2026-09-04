"""
Main HTTP API client for slskr
"""

import asyncio
import json
import logging
from typing import Any, Dict, List, Optional
from urllib.parse import quote, urlencode, urlsplit, urlunsplit
import aiohttp

from .exceptions import ApiError, NetworkError, TimeoutError
from .batch import BatchClient
from .websocket import WebSocketClient

logger = logging.getLogger(__name__)

MAX_HTTP_RESPONSE_BYTES = 8 * 1024 * 1024
MAX_HTTP_ERROR_BYTES = 64 * 1024


class SlskrClient:
    """Main HTTP client for slskr API"""

    def __init__(
        self,
        base_url: str,
        token: str,
        timeout: int = 30,
        retries: int = 3,
        retry_delay: int = 1,
        debug: bool = False,
    ):
        """Initialize client"""
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
        self.base_url = urlunsplit(
            (parsed_url.scheme, parsed_url.netloc, parsed_url.path.rstrip("/"), "", "")
        )
        self.token = token
        self.timeout = timeout
        self.retries = retries
        self.retry_delay = retry_delay
        self.debug = debug
        self.session: Optional[aiohttp.ClientSession] = None
        
        # Initialize batch and websocket clients
        self.batch = BatchClient(self)
        self.ws = None  # WebSocket client created on demand
        self._ws_lock = asyncio.Lock()

    async def __aenter__(self):
        """Context manager entry"""
        await self._ensure_session()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit"""
        await self.close()

    async def _ensure_session(self):
        """Ensure session is open"""
        if self.session is None or self.session.closed is True:
            self.session = aiohttp.ClientSession()

    async def close(self):
        """Close session"""
        session = self.session
        self.session = None
        if session:
            await session.close()
        async with self._ws_lock:
            ws = self.ws
            self.ws = None
            if ws:
                await ws.disconnect()

    # =========================================================================
    # WebSocket Connection
    # =========================================================================

    async def connect_ws(self) -> WebSocketClient:
        """Connect to WebSocket for real-time events"""
        async with self._ws_lock:
            if self.ws is None:
                self.ws = WebSocketClient(
                    self.base_url,
                    self.token,
                    debug=self.debug,
                    connect_timeout=self.timeout,
                )

            if not self.ws.is_connected():
                await self.ws.connect()

            return self.ws

    async def disconnect_ws(self):
        """Disconnect WebSocket"""
        async with self._ws_lock:
            if self.ws:
                await self.ws.disconnect()

    def get_ws(self) -> Optional[WebSocketClient]:
        """Get WebSocket client (must be connected first)"""
        return self.ws if self.ws and self.ws.is_connected() else None

    # =========================================================================
    # Health & Version
    # =========================================================================

    async def health(self) -> Dict[str, Any]:
        """Get server health status"""
        return await self._get("/api/health", authenticated=False)

    async def version(self) -> Dict[str, Any]:
        """Get version information"""
        return await self._get("/api/version", authenticated=False)

    # =========================================================================
    # Configuration
    # =========================================================================

    async def get_config(self) -> Dict[str, Any]:
        """Get current configuration"""
        return await self._get("/api/config")

    async def get_stats(self) -> Dict[str, Any]:
        """Get server statistics"""
        return await self._get("/api/stats")

    # =========================================================================
    # Capabilities
    # =========================================================================

    async def get_capabilities(self) -> Dict[str, Any]:
        """Get API capabilities"""
        return await self._get("/api/capabilities", authenticated=False)

    # =========================================================================
    # Sessions
    # =========================================================================

    async def get_sessions(self) -> List[Dict[str, Any]]:
        """Get the current server session snapshot as a list."""
        result = await self._get("/api/session")
        return [result] if isinstance(result, dict) else []

    async def create_session(
        self, kind: str = "server", parameters: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """Connect the server session and return its refreshed snapshot."""
        if kind != "server":
            raise ValueError(f"Unsupported session type: {kind}")
        if parameters and ("username" in parameters or "password" in parameters):
            await self._put("/api/server", parameters)
        else:
            await self._post("/api/session/connect", {})
        sessions = await self.get_sessions()
        return sessions[0] if sessions else {}

    async def ping_session(self, session_id: str = "server") -> Dict[str, Any]:
        """Keep the server session alive."""
        del session_id
        return await self._post("/api/session/ping", {})

    async def disconnect_session(self, session_id: str = "server") -> None:
        """Disconnect the server session."""
        del session_id
        await self._post("/api/session/disconnect", {})

    async def get_session_privileges(
        self, session_id: str = "server"
    ) -> Dict[str, Any]:
        """Request and return the current session privilege projection."""
        del session_id
        await self._post("/api/session/privileges/check", {})
        snapshot = await self._get("/api/session")
        if not isinstance(snapshot, dict):
            return {"user_id": "", "privileges": []}
        seconds = snapshot.get("privileges_seconds", 0)
        privileged = isinstance(seconds, (int, float)) and seconds > 0
        return {
            "user_id": str(snapshot.get("username", "")),
            "privileges": ["privileged"] if privileged else [],
        }

    # =========================================================================
    # Users
    # =========================================================================

    async def list_users(self, limit: int = 50, offset: int = 0) -> List[Dict]:
        """List watched users."""
        result = await self._get("/api/users", params={"limit": limit, "offset": offset})
        return self._response_list(result, "users", "entries")

    async def get_user(self, username: str) -> Dict[str, Any]:
        """Get a watched user's information."""
        return await self._get(
            f"/api/users/{self._path_segment(username)}/info"
        )

    # =========================================================================
    # Search
    # =========================================================================

    async def list_searches(self, limit: int = 50, offset: int = 0) -> List[Dict]:
        """List searches"""
        result = await self._get("/api/searches", params={"limit": limit, "offset": offset})
        return self._response_list(result, "searches", "entries")

    async def create_search(self, query: str, room: str = None, target: str = None) -> Dict:
        """Create new search"""
        body = {"query": query, "room": room, "target": target}
        result = await self._post("/api/searches", body)
        if isinstance(result, dict) and "id" not in result:
            search_id = result.get("searchId")
            if search_id is not None:
                result = {**result, "id": search_id}
        return result

    async def get_search_details(
        self, search_id: str, limit: int = 50, offset: int = 0
    ) -> Dict:
        """Get search details and results"""
        return await self._get(
            f"/api/searches/{self._path_segment(search_id)}",
            params={"limit": limit, "offset": offset},
        )

    # =========================================================================
    # Messages
    # =========================================================================

    async def list_messages(self, limit: int = 50, offset: int = 0) -> List[Dict]:
        """List messages"""
        result = await self._get("/api/messages", params={"limit": limit, "offset": offset})
        return self._response_list(result, "messages", "entries")

    async def get_user_messages(
        self, username: str, limit: int = 50, offset: int = 0
    ) -> List[Dict]:
        """Get messages from user"""
        result = await self._get(
            f"/api/messages/{self._path_segment(username)}",
            params={"limit": limit, "offset": offset},
        )
        return self._response_list(result, "messages", "entries")

    async def send_message(self, recipient: str, content: str) -> Dict:
        """Send message to user"""
        body = {"username": recipient, "body": content}
        return await self._post("/api/messages", body)

    async def acknowledge_message(self, message_id: str) -> None:
        """Mark message as acknowledged"""
        await self._post(
            f"/api/messages/{self._path_segment(message_id)}/ack",
            {},
        )

    # =========================================================================
    # Transfers
    # =========================================================================

    async def list_transfers(
        self,
        direction: str = None,
        status: str = None,
        limit: int = 50,
        offset: int = 0,
    ) -> List[Dict]:
        """List transfers"""
        params = {"limit": limit, "offset": offset}
        if direction:
            params["direction"] = self._transfer_direction_value(direction)
        if status:
            params["status"] = status

        result = await self._get("/api/transfers", params=params)
        return self._response_list(result, "transfers", "entries")

    async def create_transfer(
        self, direction: str, peer_username: str, filename: str
    ) -> Dict:
        """Create transfer"""
        body = {
            "direction": self._transfer_direction_value(direction),
            "peer_username": peer_username,
            "filename": filename,
        }
        return await self._post("/api/transfers", body)

    async def get_transfer(self, transfer_id: str) -> Dict:
        """Get transfer details"""
        return await self._get(f"/api/transfers/{self._path_segment(transfer_id)}")

    async def cancel_transfer(self, transfer_id: str) -> None:
        """Cancel transfer"""
        await self._delete(f"/api/transfers/{self._path_segment(transfer_id)}")

    # =========================================================================
    # Rooms
    # =========================================================================

    async def list_rooms(self, limit: int = 50, offset: int = 0) -> List[Dict]:
        """List rooms."""
        result = await self._get("/api/rooms", params={"limit": limit, "offset": offset})
        return self._response_list(result, "rooms", "entries")

    async def get_room(self, name: str) -> Dict[str, Any]:
        """Get room details by name."""
        return await self._get(f"/api/rooms/{self._path_segment(name)}")

    async def join_room(self, name: str) -> Dict[str, Any]:
        """Join a room."""
        return await self._post(
            f"/api/rooms/{self._path_segment(name)}/join", {"name": name}
        )

    async def leave_room(self, name: str) -> None:
        """Leave a room."""
        await self._delete(f"/api/rooms/{self._path_segment(name)}/join")

    # =========================================================================
    # Browse
    # =========================================================================

    async def browse_user(
        self,
        username: str,
        folder: Optional[str] = None,
        limit: int = 50,
        offset: int = 0,
    ) -> Dict[str, Any]:
        """Get a user's shared files, optionally filtered to a folder."""
        params: Dict[str, Any] = {"limit": limit, "offset": offset}
        if folder is not None:
            params["folder"] = folder
        return await self._get(
            f"/api/users/{self._path_segment(username)}/browse", params=params
        )

    async def request_browse(
        self, username: str, folder: Optional[str] = None
    ) -> Dict[str, Any]:
        """Request a fresh browse listing from a user."""
        segment = self._path_segment(username)
        if folder is not None:
            return await self._post(
                f"/api/users/{segment}/browse/folder", {"folder": folder}
            )
        return await self._post(
            f"/api/users/{segment}/browse/request", {}
        )

    async def get_browse_requests(
        self,
        status: Optional[str] = None,
        limit: int = 50,
        offset: int = 0,
    ) -> List[Dict]:
        """List browse requests."""
        params: Dict[str, Any] = {"limit": limit, "offset": offset}
        if status:
            params["status"] = status
        result = await self._get("/api/browse/requests", params=params)
        return self._response_list(result, "requests", "entries")

    async def respond_to_browse_request(
        self, username: str, action: str, folder: Optional[str] = None
    ) -> Dict[str, Any]:
        """Accept or reject a browse request."""
        if action not in ("accept", "reject"):
            raise ValueError("action must be 'accept' or 'reject'")
        segment = self._path_segment(username)
        if action == "reject":
            path = f"/api/users/{segment}/browse/cancel"
            body = {"reason": "rejected by client"}
        else:
            path = f"/api/users/{segment}/browse/folder"
            body = {"folder": folder or ""}
        result = await self._post(path, body)
        return result if isinstance(result, dict) else {}

    # =========================================================================
    # Events
    # =========================================================================

    async def get_events(
        self,
        event_type: Optional[str] = None,
        limit: int = 50,
        offset: int = 0,
    ) -> List[Dict]:
        """List recorded events."""
        params: Dict[str, Any] = {"limit": limit, "offset": offset}
        if event_type:
            params["kind"] = event_type
        result = await self._get("/api/events", params=params)
        return self._response_list(result, "events", "entries")

    # =========================================================================
    # Shares and filters
    # =========================================================================

    async def list_shares(self, limit: int = 50, offset: int = 0) -> List[Dict]:
        """List shared files and directories."""
        result = await self._get("/api/shares", params={"limit": limit, "offset": offset})
        return self._response_list(result, "shares", "local", "entries")

    async def refresh_shares(self) -> Dict[str, Any]:
        """Rescan configured shared directories."""
        return await self._post("/api/shares/rescan", {})

    async def get_filters(self) -> Dict[str, Any]:
        """Get download/search filter settings."""
        return await self._get("/api/config/download-filter")

    async def update_filters(self, filters: Dict[str, Any]) -> Dict[str, Any]:
        """Update download/search filter settings."""
        return await self._put("/api/config/download-filter", filters)

    # =========================================================================
    # MediaCore cache
    # =========================================================================

    async def get_cache_stats(self) -> Dict[str, Any]:
        """Get MediaCore retrieval cache statistics."""
        return await self._get("/api/mediacore/retrieve/stats")

    async def invalidate_cache(self, keys: Optional[List[str]] = None) -> Dict[str, Any]:
        """Invalidate selected MediaCore cache keys, or the complete cache."""
        return await self._post(
            "/api/mediacore/retrieve/cache/clear", {"keys": keys or []}
        )

    # =========================================================================
    # HTTP Methods
    # =========================================================================

    async def _get(
        self,
        path: str,
        params: Dict = None,
        authenticated: bool = True,
    ) -> Any:
        """Make GET request"""
        return await self._request("GET", path, params=params, authenticated=authenticated)

    async def _post(
        self,
        path: str,
        body: Dict,
        authenticated: bool = True,
    ) -> Dict:
        """Make POST request"""
        return await self._request("POST", path, body=body, authenticated=authenticated)

    async def _put(
        self,
        path: str,
        body: Dict,
        authenticated: bool = True,
    ) -> Dict:
        """Make PUT request"""
        return await self._request("PUT", path, body=body, authenticated=authenticated)

    async def _delete(
        self,
        path: str,
        authenticated: bool = True,
    ) -> None:
        """Make DELETE request"""
        await self._request("DELETE", path, authenticated=authenticated)

    # =========================================================================
    # Core Request Handler
    # =========================================================================

    async def _request(
        self,
        method: str,
        path: str,
        params: Dict = None,
        body: Dict = None,
        authenticated: bool = True,
        attempt: int = 0,
    ) -> Any:
        """Make HTTP request"""
        await self._ensure_session()

        url = self._build_url(path)
        if params:
            url += "?" + urlencode(params)

        headers = {"Content-Type": "application/json"}
        if authenticated:
            headers["Authorization"] = f"Bearer {self.token}"

        try:
            timeout = aiohttp.ClientTimeout(total=self.timeout)

            async with self.session.request(
                method,
                url,
                json=body,
                headers=headers,
                timeout=timeout,
            ) as response:
                if self.debug:
                    logger.debug("[slskr] %s %s %s", method, url, response.status)

                if response.status >= 400:
                    error_data = {}
                    try:
                        parsed_error = await self._read_json(response, MAX_HTTP_ERROR_BYTES)
                        if isinstance(parsed_error, dict):
                            error_data = parsed_error
                    except Exception:
                        pass

                    error_code = next(
                        (
                            error_data.get(key)
                            for key in ("code", "error")
                            if isinstance(error_data.get(key), str)
                            and error_data.get(key)
                        ),
                        f"HTTP {response.status}",
                    )
                    message = next(
                        (
                            error_data.get(key)
                            for key in ("message", "detail", "error")
                            if isinstance(error_data.get(key), str)
                            and error_data.get(key)
                        ),
                        f"HTTP {response.status}",
                    )
                    details = error_data.get("details")
                    if not isinstance(details, str):
                        details = None

                    raise ApiError(
                        response.status,
                        error_code,
                        message=message,
                        details=details,
                    )

                if response.status == 204:
                    return None

                return await self._read_json(response, MAX_HTTP_RESPONSE_BYTES)

        except asyncio.TimeoutError:
            raise TimeoutError(f"Request timeout after {self.timeout}s")
        except (ApiError, NetworkError):
            raise
        except Exception as e:
            if method == "GET" and attempt < self.retries:
                await asyncio.sleep(self.retry_delay)
                return await self._request(
                    method,
                    path,
                    params=params,
                    body=body,
                    authenticated=authenticated,
                    attempt=attempt + 1,
                )

            raise NetworkError(f"Failed to {method} {url}", cause=e)

    async def _read_json(self, response, maximum: int) -> Any:
        content_length = response.content_length
        if content_length is not None and content_length > maximum:
            raise NetworkError(f"HTTP response body exceeds {maximum} bytes")

        chunks = []
        length = 0
        async for chunk in response.content.iter_chunked(64 * 1024):
            length += len(chunk)
            if length > maximum:
                raise NetworkError(f"HTTP response body exceeds {maximum} bytes")
            chunks.append(chunk)
        body = b"".join(chunks)
        if not body.strip():
            return None
        return json.loads(body)

    def _path_segment(self, value: str) -> str:
        return quote(str(value), safe="")

    @staticmethod
    def _response_list(result: Any, *keys: str) -> List[Dict]:
        if isinstance(result, list):
            return result
        if isinstance(result, dict):
            for key in keys:
                value = result.get(key)
                if isinstance(value, list):
                    return value
        return []

    @staticmethod
    def _transfer_direction_value(direction: str) -> Any:
        normalized = str(direction).strip().lower()
        if normalized == "download":
            return 0
        if normalized == "upload":
            return 1
        return direction

    def _build_url(self, path: str) -> str:
        return self.base_url + (path if path.startswith("/") else f"/{path}")
