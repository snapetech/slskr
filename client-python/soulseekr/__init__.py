"""
soulseekR HTTP API Python Client

Official Python client library for the soulseekR HTTP API.
"""

from .client import SoulseekrClient
from .exceptions import (
    ApiError,
    NetworkError,
    TimeoutError,
    ValidationError,
)
from .batch import BatchClient, BatchBuilder
from .websocket import WebSocketClient

__version__ = "1.0.0"
__all__ = [
    "SoulseekrClient",
    "BatchClient",
    "BatchBuilder",
    "WebSocketClient",
    "ApiError",
    "NetworkError",
    "TimeoutError",
    "ValidationError",
]
