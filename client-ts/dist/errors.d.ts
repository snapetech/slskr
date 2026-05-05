/**
 * Error classes for slskr API client
 */
export declare class ApiError extends Error {
    status: number;
    code: string;
    details?: string | undefined;
    constructor(status: number, code: string, message?: string, details?: string | undefined);
    isClientError(): boolean;
    isServerError(): boolean;
    isNotFound(): boolean;
    isUnauthorized(): boolean;
    isForbidden(): boolean;
    isConflict(): boolean;
}
export declare class NetworkError extends Error {
    cause?: Error | undefined;
    constructor(message: string, cause?: Error | undefined);
}
export declare class TimeoutError extends Error {
    constructor(message?: string);
}
export declare class ValidationError extends Error {
    field: string;
    constructor(field: string, message: string);
}
export declare class NotImplementedError extends Error {
    constructor(feature: string);
}
//# sourceMappingURL=errors.d.ts.map