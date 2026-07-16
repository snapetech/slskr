"use strict";
/**
 * Batch operations client for efficient bulk requests
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.BatchBuilder = exports.BatchClient = exports.maxBatchOperations = void 0;
exports.maxBatchOperations = 100;
function cloneJson(value) {
    if (Array.isArray(value)) {
        return value.map((item) => cloneJson(item));
    }
    if (value !== null && typeof value === 'object') {
        return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, cloneJson(item)]));
    }
    return value;
}
function cloneOperation(operation) {
    return {
        ...operation,
        ...(operation.body === undefined ? {} : { body: cloneJson(operation.body) }),
    };
}
class BatchClient {
    constructor(client) {
        this.client = client;
    }
    /**
     * Create a new batch builder
     */
    builder() {
        return new BatchBuilder(this.client);
    }
    /**
     * Execute batch operations
     */
    async execute(operations) {
        if (operations.length === 0) {
            throw new Error('Batch must contain at least one operation');
        }
        if (operations.length > exports.maxBatchOperations) {
            throw new Error(`Batch cannot contain more than ${exports.maxBatchOperations} operations`);
        }
        const request = { operations: operations.map(cloneOperation) };
        // Use internal client method to make the request
        return this.client.postAuth('/api/batch', request);
    }
    /**
     * Check if all results were successful
     */
    allSuccessful(response) {
        return response.results.every((r) => r.status >= 200 && r.status < 300);
    }
    /**
     * Get all failed results
     */
    getFailed(response) {
        return response.results.filter((r) => r.status >= 400);
    }
    /**
     * Get all successful results
     */
    getSuccessful(response) {
        return response.results.filter((r) => r.status >= 200 && r.status < 300);
    }
}
exports.BatchClient = BatchClient;
class BatchBuilder {
    constructor(client) {
        this.client = client;
        this.operations = [];
        this.idCounter = 0;
    }
    /**
     * Add GET operation
     */
    get(path, id) {
        this.operations.push({
            id: id || `op-${++this.idCounter}`,
            method: 'GET',
            path,
        });
        return this;
    }
    /**
     * Add POST operation
     */
    post(path, body, id) {
        this.operations.push({
            id: id || `op-${++this.idCounter}`,
            method: 'POST',
            path,
            body: cloneJson(body),
        });
        return this;
    }
    /**
     * Add PUT operation
     */
    put(path, body, id) {
        this.operations.push({
            id: id || `op-${++this.idCounter}`,
            method: 'PUT',
            path,
            body: cloneJson(body),
        });
        return this;
    }
    /**
     * Add DELETE operation
     */
    delete(path, id) {
        this.operations.push({
            id: id || `op-${++this.idCounter}`,
            method: 'DELETE',
            path,
        });
        return this;
    }
    /**
     * Add multiple operations at once
     */
    addOperations(ops) {
        this.operations.push(...ops.map(cloneOperation));
        return this;
    }
    /**
     * Get current operations
     */
    getOperations() {
        return this.operations.map(cloneOperation);
    }
    /**
     * Clear all operations
     */
    clear() {
        this.operations = [];
        this.idCounter = 0;
        return this;
    }
    /**
     * Get operation count
     */
    size() {
        return this.operations.length;
    }
    /**
     * Execute the batch
     */
    async execute() {
        if (this.operations.length === 0) {
            throw new Error('Batch is empty');
        }
        if (this.operations.length > exports.maxBatchOperations) {
            throw new Error(`Batch cannot contain more than ${exports.maxBatchOperations} operations`);
        }
        const request = { operations: this.operations.map(cloneOperation) };
        return this.client.postAuth('/api/batch', request);
    }
    /**
     * Create shorthand for common operations
     */
    static getStats(id) {
        return {
            id: id || 'stats',
            method: 'GET',
            path: '/api/stats',
        };
    }
    static getTransfers(id) {
        return {
            id: id || 'transfers',
            method: 'GET',
            path: '/api/transfers',
        };
    }
    static getMessages(id) {
        return {
            id: id || 'messages',
            method: 'GET',
            path: '/api/messages',
        };
    }
    static getSearches(id) {
        return {
            id: id || 'searches',
            method: 'GET',
            path: '/api/searches',
        };
    }
}
exports.BatchBuilder = BatchBuilder;
//# sourceMappingURL=batch-client.js.map