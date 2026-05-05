"use strict";
/**
 * Error classes for slskr API client
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.NotImplementedError = exports.ValidationError = exports.TimeoutError = exports.NetworkError = exports.ApiError = void 0;
class ApiError extends Error {
    constructor(status, code, message, details) {
        super(message || `API Error: ${code}`);
        this.status = status;
        this.code = code;
        this.details = details;
        this.name = 'ApiError';
        Object.setPrototypeOf(this, ApiError.prototype);
    }
    isClientError() {
        return this.status >= 400 && this.status < 500;
    }
    isServerError() {
        return this.status >= 500;
    }
    isNotFound() {
        return this.status === 404;
    }
    isUnauthorized() {
        return this.status === 401;
    }
    isForbidden() {
        return this.status === 403;
    }
    isConflict() {
        return this.status === 409;
    }
}
exports.ApiError = ApiError;
class NetworkError extends Error {
    constructor(message, cause) {
        super(message);
        this.cause = cause;
        this.name = 'NetworkError';
        Object.setPrototypeOf(this, NetworkError.prototype);
    }
}
exports.NetworkError = NetworkError;
class TimeoutError extends Error {
    constructor(message = 'Request timeout') {
        super(message);
        this.name = 'TimeoutError';
        Object.setPrototypeOf(this, TimeoutError.prototype);
    }
}
exports.TimeoutError = TimeoutError;
class ValidationError extends Error {
    constructor(field, message) {
        super(message);
        this.field = field;
        this.name = 'ValidationError';
        Object.setPrototypeOf(this, ValidationError.prototype);
    }
}
exports.ValidationError = ValidationError;
class NotImplementedError extends Error {
    constructor(feature) {
        super(`${feature} is not implemented`);
        this.name = 'NotImplementedError';
        Object.setPrototypeOf(this, NotImplementedError.prototype);
    }
}
exports.NotImplementedError = NotImplementedError;
//# sourceMappingURL=errors.js.map