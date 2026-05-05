/**
 * Batch operations client for efficient bulk requests
 */
import { BatchOperation, BatchResponse, BatchResult } from './types';
import { SlskrClient } from './client';
export declare class BatchClient {
    private client;
    constructor(client: SlskrClient);
    /**
     * Create a new batch builder
     */
    builder(): BatchBuilder;
    /**
     * Execute batch operations
     */
    execute(operations: BatchOperation[]): Promise<BatchResponse>;
    /**
     * Check if all results were successful
     */
    allSuccessful(response: BatchResponse): boolean;
    /**
     * Get all failed results
     */
    getFailed(response: BatchResponse): BatchResult[];
    /**
     * Get all successful results
     */
    getSuccessful(response: BatchResponse): BatchResult[];
}
export declare class BatchBuilder {
    private client;
    private operations;
    private idCounter;
    constructor(client: SlskrClient);
    /**
     * Add GET operation
     */
    get(path: string, id?: string): this;
    /**
     * Add POST operation
     */
    post(path: string, body: any, id?: string): this;
    /**
     * Add PUT operation
     */
    put(path: string, body: any, id?: string): this;
    /**
     * Add DELETE operation
     */
    delete(path: string, id?: string): this;
    /**
     * Add multiple operations at once
     */
    addOperations(ops: BatchOperation[]): this;
    /**
     * Get current operations
     */
    getOperations(): BatchOperation[];
    /**
     * Clear all operations
     */
    clear(): this;
    /**
     * Get operation count
     */
    size(): number;
    /**
     * Execute the batch
     */
    execute(): Promise<BatchResponse>;
    /**
     * Create shorthand for common operations
     */
    static getStats(id?: string): BatchOperation;
    static getTransfers(id?: string): BatchOperation;
    static getMessages(id?: string): BatchOperation;
    static getSearches(id?: string): BatchOperation;
}
//# sourceMappingURL=batch-client.d.ts.map