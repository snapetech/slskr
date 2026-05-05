/**
 * Batch operations client for efficient bulk requests
 */

import { BatchOperation, BatchRequest, BatchResponse, BatchResult } from './types';
import { SlskrClient } from './client';

type BatchCapableClient = {
  postAuth<T>(path: string, body: unknown): Promise<T>;
};

export class BatchClient {
  constructor(private client: SlskrClient) {}

  /**
   * Create a new batch builder
   */
  builder(): BatchBuilder {
    return new BatchBuilder(this.client);
  }

  /**
   * Execute batch operations
   */
  async execute(operations: BatchOperation[]): Promise<BatchResponse> {
    if (operations.length === 0) {
      throw new Error('Batch must contain at least one operation');
    }

    if (operations.length > 100) {
      throw new Error('Batch cannot contain more than 100 operations');
    }

    const request: BatchRequest = { operations };
    
    // Use internal client method to make the request
    return (this.client as unknown as BatchCapableClient).postAuth<BatchResponse>('/api/batch', request);
  }

  /**
   * Check if all results were successful
   */
  allSuccessful(response: BatchResponse): boolean {
    return response.results.every((r) => r.status >= 200 && r.status < 300);
  }

  /**
   * Get all failed results
   */
  getFailed(response: BatchResponse): BatchResult[] {
    return response.results.filter((r) => r.status >= 400);
  }

  /**
   * Get all successful results
   */
  getSuccessful(response: BatchResponse): BatchResult[] {
    return response.results.filter((r) => r.status >= 200 && r.status < 300);
  }
}

export class BatchBuilder {
  private operations: BatchOperation[] = [];
  private idCounter = 0;

  constructor(private client: SlskrClient) {}

  /**
   * Add GET operation
   */
  get(path: string, id?: string): this {
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
  post(path: string, body: any, id?: string): this {
    this.operations.push({
      id: id || `op-${++this.idCounter}`,
      method: 'POST',
      path,
      body,
    });
    return this;
  }

  /**
   * Add PUT operation
   */
  put(path: string, body: any, id?: string): this {
    this.operations.push({
      id: id || `op-${++this.idCounter}`,
      method: 'PUT',
      path,
      body,
    });
    return this;
  }

  /**
   * Add DELETE operation
   */
  delete(path: string, id?: string): this {
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
  addOperations(ops: BatchOperation[]): this {
    this.operations.push(...ops);
    return this;
  }

  /**
   * Get current operations
   */
  getOperations(): BatchOperation[] {
    return [...this.operations];
  }

  /**
   * Clear all operations
   */
  clear(): this {
    this.operations = [];
    this.idCounter = 0;
    return this;
  }

  /**
   * Get operation count
   */
  size(): number {
    return this.operations.length;
  }

  /**
   * Execute the batch
   */
  async execute(): Promise<BatchResponse> {
    if (this.operations.length === 0) {
      throw new Error('Batch is empty');
    }

    const request: BatchRequest = { operations: this.operations };
    return (this.client as unknown as BatchCapableClient).postAuth<BatchResponse>('/api/batch', request);
  }

  /**
   * Create shorthand for common operations
   */
  static getStats(id?: string): BatchOperation {
    return {
      id: id || 'stats',
      method: 'GET',
      path: '/api/stats',
    };
  }

  static getTransfers(id?: string): BatchOperation {
    return {
      id: id || 'transfers',
      method: 'GET',
      path: '/api/transfers',
    };
  }

  static getMessages(id?: string): BatchOperation {
    return {
      id: id || 'messages',
      method: 'GET',
      path: '/api/messages',
    };
  }

  static getSearches(id?: string): BatchOperation {
    return {
      id: id || 'searches',
      method: 'GET',
      path: '/api/searches',
    };
  }
}
