"use strict";
/**
 * Main HTTP API client for slskr
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SlskrClient = void 0;
const errors_1 = require("./errors");
const MAX_HTTP_RESPONSE_BYTES = 8 * 1024 * 1024;
const MAX_HTTP_ERROR_BYTES = 64 * 1024;
const MAX_DATE_MILLISECONDS = 8640000000000000;
function isRecord(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function responseList(response, resource, ...keys) {
    let entries;
    if (Array.isArray(response)) {
        entries = response;
    }
    else if (isRecord(response)) {
        for (const key of keys) {
            if (Array.isArray(response[key])) {
                entries = response[key];
                break;
            }
        }
    }
    if (!entries || entries.some((entry) => !isRecord(entry))) {
        throw new errors_1.ResponseContractError(resource);
    }
    return entries;
}
function responseObject(response, resource = 'object') {
    if (!isRecord(response)) {
        throw new errors_1.ResponseContractError(resource);
    }
    return response;
}
function requiredObject(response, resource) {
    return responseObject(response, resource);
}
function requiredIdentifier(object, resource, ...keys) {
    for (const key of keys) {
        const value = object[key];
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
        if (typeof value === 'number' && Number.isSafeInteger(value)) {
            return String(value);
        }
    }
    throw new errors_1.ResponseContractError(resource);
}
function requiredText(value, resource) {
    if (typeof value !== 'string' || value.trim() === '') {
        throw new errors_1.ResponseContractError(resource);
    }
    return value;
}
function numberValue(value, fallback = 0) {
    return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}
function normalizeTimestamp(value) {
    if (typeof value === 'number' && Number.isFinite(value)) {
        const milliseconds = value > 10000000000 ? value : value * 1000;
        return isoTimestamp(milliseconds);
    }
    return typeof value === 'string' ? value : '';
}
function normalizeEpochSeconds(value) {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return isoTimestamp(value * 1000);
    }
    return typeof value === 'string' ? value : '';
}
function isoTimestamp(milliseconds) {
    if (!Number.isFinite(milliseconds) || Math.abs(milliseconds) > MAX_DATE_MILLISECONDS) {
        return '';
    }
    const date = new Date(milliseconds);
    return Number.isNaN(date.getTime()) ? '' : date.toISOString();
}
function normalizeSearchStatus(value) {
    switch (String(value ?? '').toLowerCase()) {
        case 'failed':
            return 'failed';
        case 'cancelled':
        case 'canceled':
            return 'cancelled';
        case 'completed':
        case 'complete':
        case 'expired':
            return 'completed';
        default:
            return 'active';
    }
}
function normalizeSearchResult(response) {
    const object = responseObject(response, 'search result');
    return {
        ...object,
        username: String(object.username ?? object.peer_username ?? ''),
        filename: String(object.filename ?? ''),
        size: numberValue(object.size),
        ...(object.bitRate !== undefined || object.bit_rate !== undefined || object.bitrate !== undefined
            ? { bitrate: numberValue(object.bitRate ?? object.bit_rate ?? object.bitrate) }
            : {}),
        ...(object.length !== undefined || object.length_seconds !== undefined
            ? { length: numberValue(object.length ?? object.length_seconds) }
            : {}),
    };
}
function normalizeSearch(response) {
    const object = responseObject(response, 'search');
    const results = Array.isArray(object.results) ? object.results : [];
    return {
        ...object,
        id: requiredIdentifier(object, 'search', 'id', 'searchId', 'token'),
        query: String(object.query ?? object.searchText ?? ''),
        status: normalizeSearchStatus(object.status ?? object.state),
        results_count: numberValue(object.results_count ?? object.result_count ?? object.resultsCount, results.length),
        started_at: normalizeTimestamp(object.started_at ?? object.startedAt ?? object.created_at),
    };
}
function normalizeSearchDetails(response) {
    const object = responseObject(response, 'search details');
    return {
        ...normalizeSearch(response),
        results: Array.isArray(object.results)
            ? object.results.map(normalizeSearchResult)
            : [],
    };
}
function normalizeMessage(response) {
    const object = responseObject(response, 'message');
    return {
        ...object,
        id: requiredIdentifier(object, 'message', 'id'),
        sender: String(object.sender ?? object.username ?? ''),
        content: String(object.content ?? object.body ?? object.message ?? ''),
        timestamp: normalizeTimestamp(object.timestamp ?? object.created_at ?? object.createdAtMs),
    };
}
function normalizeTransferStatus(value) {
    switch (String(value ?? '').toLowerCase()) {
        case 'succeeded':
        case 'completed':
        case 'complete':
            return 'completed';
        case 'failed':
        case 'error':
        case 'errored':
        case 'rejected':
            return 'failed';
        case 'cancelled':
        case 'canceled':
            return 'cancelled';
        default:
            return 'active';
    }
}
function normalizeTransfer(response) {
    const object = responseObject(response, 'transfer');
    const direction = object.direction === 1
        || String(object.direction ?? '').toLowerCase() === 'upload'
        ? 'upload'
        : 'download';
    return {
        ...object,
        id: requiredIdentifier(object, 'transfer', 'id'),
        direction,
        status: normalizeTransferStatus(object.status),
        peer_username: String(object.peer_username ?? object.username ?? ''),
        filename: String(object.filename ?? ''),
        bytes_transferred: numberValue(object.bytes_transferred ?? object.bytesTransferred),
        started_at: normalizeTimestamp(object.started_at ?? object.startedAt),
    };
}
function normalizeEvent(response) {
    const object = responseObject(response, 'event');
    const type = requiredText(object.type ?? object.kind, 'event');
    let data = object.data;
    if (typeof data === 'string') {
        try {
            data = JSON.parse(data);
        }
        catch {
            data = undefined;
        }
    }
    if (data === null || typeof data !== 'object' || Array.isArray(data)) {
        data = object.payload && typeof object.payload === 'object' && !Array.isArray(object.payload)
            ? object.payload
            : {};
    }
    return {
        ...object,
        id: requiredIdentifier(object, 'event', 'id'),
        type: type,
        data: data,
        timestamp: normalizeTimestamp(object.timestamp ?? object.created_at),
    };
}
function sessionFromSnapshot(response) {
    const snapshot = responseObject(response, 'session');
    const rawState = snapshot.state ?? snapshot.status;
    const statuses = [
        'connecting',
        'connected',
        'disconnecting',
        'disconnected',
    ];
    if (typeof rawState !== 'string' || !statuses.includes(rawState)) {
        throw new errors_1.ResponseContractError('session');
    }
    const status = rawState;
    const connectedAt = snapshot.connected_at;
    const normalizedConnectedAt = normalizeEpochSeconds(connectedAt);
    return {
        id: 'server',
        type: 'server',
        status,
        ...(normalizedConnectedAt ? { connected_at: normalizedConnectedAt } : {}),
    };
}
function normalizeBrowseStatus(value) {
    switch (String(value ?? '').toLowerCase()) {
        case 'ready':
        case 'partial':
        case 'accepted':
            return 'accepted';
        case 'failed':
        case 'cancelled':
        case 'rejected':
            return 'rejected';
        default:
            return 'pending';
    }
}
function normalizeRoom(response) {
    const object = responseObject(response, 'room');
    const name = requiredText(object.name ?? object.room, 'room');
    const rawUsers = Array.isArray(object.users)
        ? object.users
        : Array.isArray(object.members)
            ? object.members
            : undefined;
    const users = rawUsers?.filter((user) => typeof user === 'string');
    return {
        ...object,
        name,
        user_count: numberValue(object.user_count ?? object.userCount ?? object.memberCount, users?.length ?? 0),
        ...(users ? { users } : {}),
    };
}
function nullableNumber(value) {
    return typeof value === 'number' && Number.isFinite(value) ? value : null;
}
function normalizeUser(response) {
    const object = responseObject(response, 'user');
    return {
        ...object,
        username: requiredText(object.username, 'user'),
        watched: object.watched === true,
        status: typeof object.status === 'string' ? object.status : null,
        average_speed: nullableNumber(object.average_speed ?? object.averageSpeed),
        upload_count: nullableNumber(object.upload_count ?? object.uploadCount),
        file_count: nullableNumber(object.file_count ?? object.fileCount),
        directory_count: nullableNumber(object.directory_count ?? object.directoryCount),
        updated_at: numberValue(object.updated_at ?? object.updatedAt),
    };
}
function normalizeUserInfo(response, fallbackUsername) {
    const object = responseObject(response, 'user info');
    const picture = object.picture;
    return {
        ...object,
        username: String(object.username ?? fallbackUsername),
        description: typeof object.description === 'string' ? object.description : '',
        hasFreeUploadSlot: object.hasFreeUploadSlot === true || object.has_free_upload_slot === true,
        hasPicture: object.hasPicture === true || object.has_picture === true,
        picture: typeof picture === 'string' || picture === null ? picture : null,
        queueLength: numberValue(object.queueLength ?? object.queue_length),
        uploadSlots: numberValue(object.uploadSlots ?? object.upload_slots),
        uploadSpeed: numberValue(object.uploadSpeed ?? object.upload_speed),
        uploadCount: numberValue(object.uploadCount ?? object.upload_count),
        fileCount: numberValue(object.fileCount ?? object.file_count),
        directoryCount: numberValue(object.directoryCount ?? object.directory_count),
    };
}
function browseRequestFromResponse(response, fallbackUsername = '') {
    const object = responseObject(response, 'browse request');
    const username = requiredText(object.username ?? object.from ?? object.id ?? fallbackUsername, 'browse request');
    const requestedAt = object.requested_at ?? object.requestedAt;
    const requested_at = normalizeEpochSeconds(requestedAt);
    return {
        id: String(object.id ?? username),
        from: String(object.from ?? username),
        status: normalizeBrowseStatus(object.status),
        requested_at,
    };
}
function normalizeBrowseResult(response) {
    const object = responseObject(response, 'browse result');
    const hasEntries = Array.isArray(object.entries);
    const hasDirectories = Array.isArray(object.directories);
    if (!hasEntries && !hasDirectories) {
        throw new errors_1.ResponseContractError('browse result');
    }
    const rawEntries = hasEntries ? object.entries : [];
    const directEntries = rawEntries.filter((entry) => isRecord(entry));
    if (hasEntries && directEntries.length !== rawEntries.length) {
        throw new errors_1.ResponseContractError('browse result');
    }
    if (directEntries.length > 0 || Array.isArray(object.entries)) {
        return {
            entries: directEntries,
            ...(typeof object.folder === 'string' ? { folder: object.folder } : {}),
        };
    }
    const directories = hasDirectories ? object.directories : [];
    if (directories.some((directory) => !isRecord(directory))) {
        throw new errors_1.ResponseContractError('browse result');
    }
    const entries = directories.flatMap((directory) => {
        const files = directory.files;
        if (!Array.isArray(files))
            return [];
        if (files.some((entry) => !isRecord(entry))) {
            throw new errors_1.ResponseContractError('browse result');
        }
        return files;
    });
    return {
        entries,
        ...(typeof object.folder === 'string' ? { folder: object.folder } : {}),
    };
}
function normalizeShare(response) {
    return responseObject(response, 'share');
}
function normalizeDownloadFilter(response) {
    const object = responseObject(response, 'download filter');
    const rawTerms = Array.isArray(object.exclude)
        ? object.exclude
        : Array.isArray(object.terms)
            ? object.terms
            : undefined;
    if (!rawTerms || rawTerms.some((term) => typeof term !== 'string')) {
        throw new errors_1.ResponseContractError('download filter');
    }
    const terms = rawTerms;
    return {
        ...object,
        exclude: terms,
        ...(typeof object.maxTerms === 'number' ? { maxTerms: object.maxTerms } : {}),
        ...(typeof object.maxTermLength === 'number' ? { maxTermLength: object.maxTermLength } : {}),
    };
}
function normalizeCacheStats(response) {
    const object = responseObject(response, 'cache stats');
    const numericFields = [
        'totalRetrievals',
        'cacheHits',
        'cacheMisses',
        'cacheHitRatio',
        'expiredEntriesCleaned',
    ];
    if (numericFields.some((field) => (typeof object[field] !== 'number' || !Number.isFinite(object[field])))) {
        throw new errors_1.ResponseContractError('cache stats');
    }
    const hits = typeof object.cacheHits === 'number' ? object.cacheHits : 0;
    const misses = typeof object.cacheMisses === 'number' ? object.cacheMisses : 0;
    const total_requests = typeof object.totalRetrievals === 'number'
        ? object.totalRetrievals
        : hits + misses;
    const hit_rate = typeof object.cacheHitRatio === 'number'
        ? object.cacheHitRatio
        : total_requests === 0 ? 0 : hits / total_requests;
    return {
        hits,
        misses,
        evictions: typeof object.expiredEntriesCleaned === 'number' ? object.expiredEntriesCleaned : 0,
        total_requests,
        hit_rate,
    };
}
class SlskrClient {
    constructor(config) {
        const parsedUrl = new URL(config.baseUrl);
        if (!['http:', 'https:'].includes(parsedUrl.protocol) || parsedUrl.username || parsedUrl.password) {
            throw new Error('baseUrl must be an absolute HTTP or HTTPS URL without credentials');
        }
        parsedUrl.pathname = parsedUrl.pathname.replace(/\/+$/, '');
        parsedUrl.search = '';
        parsedUrl.hash = '';
        this.baseUrl = parsedUrl.toString().replace(/\/$/, '');
        this.token = config.token;
        this.timeout = config.timeout ?? 30000;
        this.retries = config.retries ?? 3;
        this.retryDelay = config.retryDelay ?? 1000;
        this.debug = config.debug ?? false;
    }
    debugBody(body) {
        if (body === null || body === undefined) {
            return body;
        }
        if (Array.isArray(body)) {
            return body.map((item) => this.debugBody(item));
        }
        if (typeof body !== 'object') {
            return body;
        }
        const redacted = {};
        for (const [key, value] of Object.entries(body)) {
            if (/(api[-_]?key|authorization|credential|pass(word)?|secret|session|token)/i.test(key)) {
                redacted[key] = '[REDACTED]';
            }
            else {
                redacted[key] = this.debugBody(value);
            }
        }
        return redacted;
    }
    // =========================================================================
    // Health & Version
    // =========================================================================
    async health() {
        return requiredObject(await this.get('/api/health', {}), 'health');
    }
    async version() {
        return requiredObject(await this.get('/api/version', {}), 'version');
    }
    // =========================================================================
    // Configuration
    // =========================================================================
    async getConfig() {
        return requiredObject(await this.getAuth('/api/config'), 'configuration');
    }
    async getStats() {
        return requiredObject(await this.getAuth('/api/stats'), 'statistics');
    }
    // =========================================================================
    // Capabilities
    // =========================================================================
    async getCapabilities() {
        return requiredObject(await this.get('/api/capabilities', {}), 'capabilities');
    }
    // =========================================================================
    // Sessions
    // =========================================================================
    async getSessions() {
        const response = await this.getAuth('/api/session');
        return [sessionFromSnapshot(response)];
    }
    async createSession(kind, parameters) {
        if (kind !== 'server') {
            throw new Error(`Unsupported session type: ${kind}`);
        }
        if (parameters && ('username' in parameters || 'password' in parameters)) {
            await this.putAuth('/api/server', parameters);
        }
        else {
            await this.postAuth('/api/session/connect', {});
        }
        return (await this.getSessions())[0];
    }
    async pingSession(_id) {
        const started = Date.now();
        const response = requiredObject(await this.postAuth('/api/session/ping', {}), 'session ping');
        if (typeof response.accepted !== 'boolean') {
            throw new errors_1.ResponseContractError('session ping');
        }
        return {
            status: response.accepted ? 'accepted' : 'unknown',
            latency_ms: Date.now() - started,
        };
    }
    async disconnectSession(_id) {
        await this.postAuth('/api/session/disconnect', {});
    }
    async getSessionPrivileges(_id) {
        await this.postAuth('/api/session/privileges/check', {});
        const snapshot = responseObject(await this.getAuth('/api/session'), 'session');
        const seconds = typeof snapshot.privileges_seconds === 'number'
            ? snapshot.privileges_seconds
            : 0;
        return {
            user_id: String(snapshot.username ?? ''),
            privileges: seconds > 0 ? ['privileged'] : [],
        };
    }
    // =========================================================================
    // Users
    // =========================================================================
    async listUsers(params) {
        const response = await this.getAuth('/api/users', params);
        return responseList(response, 'users', 'users', 'entries').map(normalizeUser);
    }
    async getUser(username) {
        return normalizeUserInfo(await this.getAuth(`/api/users/${this.pathSegment(username)}/info`), username);
    }
    // =========================================================================
    // Search
    // =========================================================================
    async listSearches(params) {
        const response = await this.getAuth('/api/searches', params);
        return responseList(response, 'searches', 'searches', 'entries').map(normalizeSearch);
    }
    async createSearch(request) {
        return normalizeSearch(await this.postAuth('/api/searches', request));
    }
    async getSearchDetails(id, params) {
        return normalizeSearchDetails(await this.getAuth(`/api/searches/${this.pathSegment(id)}`, params));
    }
    // =========================================================================
    // Messages
    // =========================================================================
    async listMessages(params) {
        const response = await this.getAuth('/api/messages', params);
        return responseList(response, 'messages', 'messages', 'entries').map(normalizeMessage);
    }
    async getUserMessages(username, params) {
        const response = await this.getAuth(`/api/messages/${this.pathSegment(username)}`, params);
        return responseList(response, 'messages', 'messages', 'entries').map(normalizeMessage);
    }
    async sendMessage(request) {
        return normalizeMessage(await this.postAuth('/api/messages', {
            username: request.recipient,
            body: request.content,
        }));
    }
    async acknowledgeMessage(id) {
        await this.postAuth(`/api/messages/${this.pathSegment(id)}/ack`, {});
    }
    // =========================================================================
    // Transfers
    // =========================================================================
    async listTransfers(params) {
        const query = params && {
            ...params,
            ...(params.direction
                ? { direction: params.direction === 'upload' ? 1 : 0 }
                : {}),
        };
        const response = await this.getAuth('/api/transfers', query);
        return responseList(response, 'transfers', 'transfers', 'entries').map(normalizeTransfer);
    }
    async createTransfer(request) {
        return normalizeTransfer(await this.postAuth('/api/transfers', {
            ...request,
            direction: request.direction === 'upload' ? 1 : 0,
        }));
    }
    async getTransfer(id) {
        return normalizeTransfer(await this.getAuth(`/api/transfers/${this.pathSegment(id)}`));
    }
    async cancelTransfer(id) {
        await this.deleteAuth(`/api/transfers/${this.pathSegment(id)}`);
    }
    // =========================================================================
    // Rooms
    // =========================================================================
    async listRooms(params) {
        const response = await this.getAuth('/api/rooms', params);
        return responseList(response, 'rooms', 'rooms', 'entries').map(normalizeRoom);
    }
    async getRoom(name) {
        return normalizeRoom(await this.getAuth(`/api/rooms/${this.pathSegment(name)}`));
    }
    async joinRoom(name) {
        return normalizeRoom(await this.postAuth(`/api/rooms/${this.pathSegment(name)}/join`, {}));
    }
    async leaveRoom(name) {
        await this.deleteAuth(`/api/rooms/${this.pathSegment(name)}/join`);
    }
    // =========================================================================
    // Shares & Filters
    // =========================================================================
    async listShares(params) {
        const response = await this.getAuth('/api/shares', params);
        return responseList(response, 'shares', 'shares', 'local', 'entries').map(normalizeShare);
    }
    async refreshShares() {
        return requiredObject(await this.postAuth('/api/shares/rescan', {}), 'share rescan');
    }
    async getFilters() {
        return normalizeDownloadFilter(await this.getAuth('/api/config/download-filter'));
    }
    async updateFilters(filters) {
        return normalizeDownloadFilter(await this.putAuth('/api/config/download-filter', filters));
    }
    // =========================================================================
    // Browse
    // =========================================================================
    async browseUser(username, params) {
        const response = await this.getAuth(`/api/users/${this.pathSegment(username)}/browse`, params);
        return normalizeBrowseResult(response);
    }
    async requestBrowse(username, folder) {
        const path = folder === undefined
            ? `/api/users/${this.pathSegment(username)}/browse/request`
            : `/api/users/${this.pathSegment(username)}/browse/folder`;
        const response = await this.postAuth(path, folder === undefined ? {} : { folder });
        return browseRequestFromResponse(response, username);
    }
    async getBrowseRequests(params) {
        const response = await this.getAuth('/api/browse/requests', params);
        return responseList(response, 'browse requests', 'requests', 'entries')
            .map((request) => browseRequestFromResponse(request));
    }
    async respondToBrowseRequest(id, action, folder) {
        const username = this.pathSegment(id);
        const path = action === 'reject'
            ? `/api/users/${username}/browse/cancel`
            : `/api/users/${username}/browse/folder`;
        const body = action === 'reject'
            ? { reason: 'rejected by client' }
            : { folder: folder ?? '' };
        return normalizeBrowseResult(await this.postAuth(path, body));
    }
    // =========================================================================
    // Events
    // =========================================================================
    async getEvents(params) {
        const query = params && {
            limit: params.limit,
            offset: params.offset,
            ...(params.type ? { kind: params.type } : {}),
            ...(params.topic ? { topic: params.topic } : {}),
            ...(params.query ? { q: params.query } : {}),
        };
        const response = await this.getAuth('/api/events', query);
        return responseList(response, 'events', 'events', 'entries').map(normalizeEvent);
    }
    // =========================================================================
    // Cache
    // =========================================================================
    async getCacheStats() {
        return normalizeCacheStats(await this.getAuth('/api/mediacore/retrieve/stats'));
    }
    async invalidateCache(keys = []) {
        return requiredObject(await this.postAuth('/api/mediacore/retrieve/cache/clear', { keys }), 'cache invalidation');
    }
    // =========================================================================
    // HTTP Methods
    // =========================================================================
    async get(path, query) {
        const url = this.buildUrl(path, query);
        return this.request('GET', url);
    }
    async getAuth(path, query) {
        const url = this.buildUrl(path, query);
        return this.request('GET', url, undefined, true);
    }
    async post(path, body) {
        return this.request('POST', this.baseUrl + path, body, false);
    }
    async postAuth(path, body) {
        return this.request('POST', this.baseUrl + path, body, true);
    }
    async put(path, body) {
        return this.request('PUT', this.baseUrl + path, body, false);
    }
    async putAuth(path, body) {
        return this.request('PUT', this.baseUrl + path, body, true);
    }
    async deleteAuth(path) {
        await this.request('DELETE', this.baseUrl + path, {}, true);
    }
    // =========================================================================
    // Core Request Handler
    // =========================================================================
    async request(method, url, body, authenticated = false, attempt = 0) {
        try {
            if (this.debug) {
                console.debug('[slskr] request', method, url, this.debugBody(body));
            }
            const headers = {
                'Content-Type': 'application/json',
            };
            if (authenticated) {
                headers['Authorization'] = `Bearer ${this.token}`;
            }
            const controller = new AbortController();
            const timeoutId = this.timeout > 0
                ? setTimeout(() => controller.abort(), this.timeout)
                : undefined;
            let response;
            try {
                response = await fetch(url, {
                    method,
                    headers,
                    body: body === undefined ? undefined : JSON.stringify(body),
                    signal: controller.signal,
                    redirect: 'error',
                });
            }
            finally {
                if (timeoutId !== undefined) {
                    clearTimeout(timeoutId);
                }
            }
            if (!response.ok) {
                const parsedError = await this.readJson(response, MAX_HTTP_ERROR_BYTES).catch(() => ({}));
                const errorData = parsedError !== null &&
                    typeof parsedError === 'object' &&
                    !Array.isArray(parsedError)
                    ? parsedError
                    : {};
                const errorCode = typeof errorData.code === 'string'
                    ? errorData.code
                    : typeof errorData.error === 'string'
                        ? errorData.error
                        : `HTTP ${response.status}`;
                const message = typeof errorData.message === 'string'
                    ? errorData.message
                    : typeof errorData.detail === 'string'
                        ? errorData.detail
                        : typeof errorData.error === 'string'
                            ? errorData.error
                            : `HTTP ${response.status}`;
                const details = typeof errorData.details === 'string' ? errorData.details : undefined;
                throw new errors_1.ApiError(response.status, errorCode, message, details);
            }
            if (response.status === 204) {
                return undefined;
            }
            const data = await this.readJson(response, MAX_HTTP_RESPONSE_BYTES);
            return data;
        }
        catch (error) {
            if (error instanceof errors_1.ApiError || error instanceof errors_1.NetworkError) {
                throw error;
            }
            if (error instanceof Error && error.name === 'AbortError') {
                throw new errors_1.TimeoutError(`Request timeout after ${this.timeout}ms`);
            }
            if (method === 'GET' && attempt < this.retries) {
                await new Promise((resolve) => setTimeout(resolve, this.retryDelay));
                return this.request(method, url, body, authenticated, attempt + 1);
            }
            throw new errors_1.NetworkError(`Failed to ${method} ${url}`, error instanceof Error ? error : undefined);
        }
    }
    async readJson(response, maximum) {
        const declaredLength = response.headers.get('content-length');
        if (declaredLength !== null && Number(declaredLength) > maximum) {
            throw new errors_1.NetworkError(`HTTP response body exceeds ${maximum} bytes`);
        }
        const reader = response.body?.getReader();
        if (!reader) {
            const text = await response.text();
            if (new TextEncoder().encode(text).byteLength > maximum) {
                throw new errors_1.NetworkError(`HTTP response body exceeds ${maximum} bytes`);
            }
            if (!text.trim()) {
                return undefined;
            }
            return JSON.parse(text);
        }
        const chunks = [];
        let length = 0;
        while (true) {
            const { done, value } = await reader.read();
            if (done)
                break;
            length += value.byteLength;
            if (length > maximum) {
                await reader.cancel();
                throw new errors_1.NetworkError(`HTTP response body exceeds ${maximum} bytes`);
            }
            chunks.push(value);
        }
        const body = new Uint8Array(length);
        let offset = 0;
        for (const chunk of chunks) {
            body.set(chunk, offset);
            offset += chunk.byteLength;
        }
        const text = new TextDecoder().decode(body);
        if (!text.trim()) {
            return undefined;
        }
        return JSON.parse(text);
    }
    // =========================================================================
    // Utilities
    // =========================================================================
    buildUrl(path, query) {
        let url = this.baseUrl + path;
        if (query && Object.keys(query).length > 0) {
            const params = new URLSearchParams();
            Object.entries(query).forEach(([key, value]) => {
                if (value !== undefined && value !== null) {
                    params.append(key, String(value));
                }
            });
            const queryString = params.toString();
            if (queryString) {
                url += '?' + queryString;
            }
        }
        return url;
    }
    pathSegment(value) {
        return encodeURIComponent(value);
    }
}
exports.SlskrClient = SlskrClient;
exports.default = SlskrClient;
//# sourceMappingURL=client.js.map