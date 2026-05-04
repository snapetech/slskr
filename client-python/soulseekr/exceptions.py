"""
Exception classes for soulseekR API client
"""


class ApiError(Exception):
    """API error response"""

    def __init__(self, status: int, code: str, message: str = None, details: str = None):
        self.status = status
        self.code = code
        self.details = details
        super().__init__(message or f"API Error: {code}")

    def is_client_error(self) -> bool:
        """Check if error is client error (4xx)"""
        return 400 <= self.status < 500

    def is_server_error(self) -> bool:
        """Check if error is server error (5xx)"""
        return self.status >= 500

    def is_not_found(self) -> bool:
        """Check if error is 404 Not Found"""
        return self.status == 404

    def is_unauthorized(self) -> bool:
        """Check if error is 401 Unauthorized"""
        return self.status == 401

    def is_forbidden(self) -> bool:
        """Check if error is 403 Forbidden"""
        return self.status == 403

    def is_conflict(self) -> bool:
        """Check if error is 409 Conflict"""
        return self.status == 409


class NetworkError(Exception):
    """Network connection error"""

    def __init__(self, message: str, cause: Exception = None):
        self.cause = cause
        super().__init__(message)


class TimeoutError(Exception):
    """Request timeout error"""

    pass


class ValidationError(Exception):
    """Input validation error"""

    def __init__(self, field: str, message: str):
        self.field = field
        super().__init__(f"{field}: {message}")
