"use strict";
/**
 * Main HTTP API client for slskr
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SlskrClient = void 0;
const errors_1 = require("./errors");
const MAX_HTTP_RESPONSE_BYTES = 8 * 1024 * 1024;
const MAX_HTTP_ERROR_BYTES = 64 * 1024;
class SlskrClient {
    constructor(config) {
        this.baseUrl = config.baseUrl.replace(/\/$/, '');
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
        return this.get('/api/health', {});
    }
    async version() {
        return this.get('/api/version', {});
    }
    // =========================================================================
    // Configuration
    // =========================================================================
    async getConfig() {
        return this.getAuth('/api/config');
    }
    async getStats() {
        return this.getAuth('/api/stats');
    }
    // =========================================================================
    // Capabilities
    // =========================================================================
    async getCapabilities() {
        return this.get('/api/capabilities', {});
    }
    // =========================================================================
    // Sessions
    // =========================================================================
    async getSessions() {
        const response = await this.getAuth('/api/sessions');
        return response.sessions || [];
    }
    async createSession(kind, parameters) {
        return this.postAuth('/api/sessions', { kind, parameters });
    }
    async pingSession(id) {
        return this.postAuth(`/api/sessions/${this.pathSegment(id)}/ping`, {});
    }
    async disconnectSession(id) {
        await this.deleteAuth(`/api/sessions/${this.pathSegment(id)}`);
    }
    async getSessionPrivileges(id) {
        return this.getAuth(`/api/sessions/${this.pathSegment(id)}/privileges`);
    }
    // =========================================================================
    // Search
    // =========================================================================
    async listSearches(params) {
        const response = await this.getAuth('/api/searches', params);
        return response.searches || [];
    }
    async createSearch(request) {
        return this.postAuth('/api/searches', request);
    }
    async getSearchDetails(id, params) {
        return this.getAuth(`/api/searches/${this.pathSegment(id)}`, params);
    }
    // =========================================================================
    // Messages
    // =========================================================================
    async listMessages(params) {
        const response = await this.getAuth('/api/messages', params);
        return response.messages || [];
    }
    async getUserMessages(username, params) {
        const response = await this.getAuth(`/api/messages/${this.pathSegment(username)}`, params);
        return response.messages || [];
    }
    async sendMessage(request) {
        return this.postAuth('/api/messages', request);
    }
    async acknowledgeMessage(id) {
        await this.putAuth(`/api/messages/${this.pathSegment(id)}/acknowledge`, {});
    }
    // =========================================================================
    // Transfers
    // =========================================================================
    async listTransfers(params) {
        const response = await this.getAuth('/api/transfers', params);
        return response.transfers || [];
    }
    async createTransfer(request) {
        return this.postAuth('/api/transfers', request);
    }
    async getTransfer(id) {
        return this.getAuth(`/api/transfers/${this.pathSegment(id)}`);
    }
    async cancelTransfer(id) {
        await this.deleteAuth(`/api/transfers/${this.pathSegment(id)}`);
    }
    // =========================================================================
    // Rooms
    // =========================================================================
    async listRooms(params) {
        const response = await this.getAuth('/api/rooms', params);
        return response.rooms || [];
    }
    async getRoom(name) {
        return this.getAuth(`/api/rooms/${this.pathSegment(name)}`);
    }
    async joinRoom(name) {
        return this.postAuth(`/api/rooms/${this.pathSegment(name)}`, {});
    }
    async leaveRoom(name) {
        await this.deleteAuth(`/api/rooms/${this.pathSegment(name)}`);
    }
    // =========================================================================
    // Browse
    // =========================================================================
    async browseUser(username, params) {
        return this.getAuth(`/api/browse/${this.pathSegment(username)}`, params);
    }
    async requestBrowse(username, folder) {
        return this.postAuth(`/api/browse/${this.pathSegment(username)}`, { folder });
    }
    async getBrowseRequests(params) {
        const response = await this.getAuth('/api/browse/requests', params);
        return response.requests || [];
    }
    async respondToBrowseRequest(id, action, folder) {
        return this.postAuth(`/api/browse/requests/${this.pathSegment(id)}`, { action, folder });
    }
    // =========================================================================
    // Events
    // =========================================================================
    async getEvents(params) {
        const response = await this.getAuth('/api/events', params);
        return response.events || [];
    }
    // =========================================================================
    // Cache
    // =========================================================================
    async getCacheStats() {
        return this.getAuth('/api/cache/stats');
    }
    async invalidateCache(keys) {
        await this.postAuth('/api/cache/invalidate', { keys });
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
        return this.request('GET', url, {}, true);
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
            const timeoutId = setTimeout(() => controller.abort(), this.timeout);
            let response;
            try {
                response = await fetch(url, {
                    method,
                    headers,
                    body: body ? JSON.stringify(body) : undefined,
                    signal: controller.signal,
                });
            }
            finally {
                clearTimeout(timeoutId);
            }
            if (!response.ok) {
                const errorData = await this.readJson(response, MAX_HTTP_ERROR_BYTES).catch(() => ({}));
                throw new errors_1.ApiError(response.status, errorData.error || `HTTP ${response.status}`, errorData.details);
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
        return JSON.parse(new TextDecoder().decode(body));
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