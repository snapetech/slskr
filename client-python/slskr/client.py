"""
Main HTTP API client for slskr
"""

import asyncio
import json
import logging
from typing import Any, Dict, List, Optional
from urllib.parse import quote, urlencode
import aiohttp

from .exceptions import ApiError, NetworkError, TimeoutError
from .batch import BatchClient
from .websocket import WebSocketClient

logger = logging.getLogger(__name__)


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
        self.base_url = base_url.rstrip("/")
        self.token = token
        self.timeout = timeout
        self.retries = retries
        self.retry_delay = retry_delay
        self.debug = debug
        self.session: Optional[aiohttp.ClientSession] = None
        
        # Initialize batch and websocket clients
        self.batch = BatchClient(self)
        self.ws = None  # WebSocket client created on demand

    async def __aenter__(self):
        """Context manager entry"""
        self.session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit"""
        if self.session:
            await self.session.close()

    async def _ensure_session(self):
        """Ensure session is open"""
        if self.session is None:
            self.session = aiohttp.ClientSession()

    async def close(self):
        """Close session"""
        if self.session:
            await self.session.close()
            self.session = None
        if self.ws:
            await self.ws.disconnect()
            self.ws = None

    # =========================================================================
    # WebSocket Connection
    # =========================================================================

    async def connect_ws(self) -> WebSocketClient:
        """Connect to WebSocket for real-time events"""
        if self.ws is None:
            self.ws = WebSocketClient(self.base_url, self.token, debug=self.debug)
        
        if not self.ws.is_connected():
            await self.ws.connect()
        
        return self.ws

    async def disconnect_ws(self):
        """Disconnect WebSocket"""
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
    # Search
    # =========================================================================

    async def list_searches(self, limit: int = 50, offset: int = 0) -> List[Dict]:
        """List searches"""
        result = await self._get("/api/searches", params={"limit": limit, "offset": offset})
        return result.get("searches", [])

    async def create_search(self, query: str, room: str = None, target: str = None) -> Dict:
        """Create new search"""
        body = {"query": query, "room": room, "target": target}
        return await self._post("/api/searches", body)

    async def get_search_details(self, search_id: str, limit: int = 50) -> Dict:
        """Get search details and results"""
        return await self._get(
            f"/api/searches/{self._path_segment(search_id)}",
            params={"limit": limit},
        )

    # =========================================================================
    # Messages
    # =========================================================================

    async def list_messages(self, limit: int = 50, offset: int = 0) -> List[Dict]:
        """List messages"""
        result = await self._get("/api/messages", params={"limit": limit, "offset": offset})
        return result.get("messages", [])

    async def get_user_messages(self, username: str, limit: int = 50) -> List[Dict]:
        """Get messages from user"""
        result = await self._get(
            f"/api/messages/{self._path_segment(username)}",
            params={"limit": limit},
        )
        return result.get("messages", [])

    async def send_message(self, recipient: str, content: str) -> Dict:
        """Send message to user"""
        body = {"recipient": recipient, "content": content}
        return await self._post("/api/messages", body)

    async def acknowledge_message(self, message_id: str) -> None:
        """Mark message as acknowledged"""
        await self._put(
            f"/api/messages/{self._path_segment(message_id)}/acknowledge",
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
            params["direction"] = direction
        if status:
            params["status"] = status

        result = await self._get("/api/transfers", params=params)
        return result.get("transfers", [])

    async def create_transfer(
        self, direction: str, peer_username: str, filename: str
    ) -> Dict:
        """Create transfer"""
        body = {
            "direction": direction,
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
    # HTTP Methods
    # =========================================================================

    async def _get(
        self,
        path: str,
        params: Dict = None,
        authenticated: bool = True,
    ) -> Dict:
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
    ) -> Dict:
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
                        error_data = await response.json()
                    except Exception:
                        pass

                    raise ApiError(
                        response.status,
                        error_data.get("error", f"HTTP {response.status}"),
                        details=error_data.get("details"),
                    )

                if response.status == 204:
                    return None

                return await response.json()

        except asyncio.TimeoutError:
            raise TimeoutError(f"Request timeout after {self.timeout}s")
        except ApiError:
            raise
        except Exception as e:
            if attempt < self.retries:
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

    def _path_segment(self, value: str) -> str:
        return quote(str(value), safe="")

    def _build_url(self, path: str) -> str:
        return self.base_url + (path if path.startswith("/") else f"/{path}")
