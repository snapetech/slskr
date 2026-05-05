"""
slskr HTTP API Python Client

Python client library for the slskr HTTP API.
"""

from .client import SlskrClient
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
    "SlskrClient",
    "BatchClient",
    "BatchBuilder",
    "WebSocketClient",
    "ApiError",
    "NetworkError",
    "TimeoutError",
    "ValidationError",
]
