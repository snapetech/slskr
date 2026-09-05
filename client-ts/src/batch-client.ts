/**
 * Batch operations client for efficient bulk requests
 */

import { BatchOperation, BatchRequest, BatchResponse, BatchResult } from './types';
import { SlskrClient } from './client';
import { ResponseContractError } from './errors';

type BatchCapableClient = {
  postAuth<T>(path: string, body: unknown): Promise<T>;
};

export const maxBatchOperations = 100;

function cloneJson<T>(value: T): T {
  if (Array.isArray(value)) {
    return value.map((item) => cloneJson(item)) as T;
  }
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, cloneJson(item)])
    ) as T;
  }
  return value;
}

function cloneOperation(operation: BatchOperation): BatchOperation {
  return {
    ...operation,
    ...(operation.body === undefined ? {} : { body: cloneJson(operation.body) }),
  };
}

function validateBatchOperationIds(operations: BatchOperation[]): void {
  const seen = new Set<string>();
  for (const operation of operations) {
    if (seen.has(operation.id)) {
      throw new Error(`Batch contains duplicate operation ID: ${operation.id}`);
    }
    seen.add(operation.id);
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function parseBatchResponse(response: unknown): BatchResponse {
  if (!isRecord(response) || !Array.isArray(response.results)) {
    throw new ResponseContractError('batch');
  }
  if (
    typeof response.total_time_ms !== 'number'
    || !Number.isSafeInteger(response.total_time_ms)
    || response.total_time_ms < 0
  ) {
    throw new ResponseContractError('batch');
  }
  for (const result of response.results) {
    if (
      !isRecord(result)
      || typeof result.id !== 'string'
      || result.id.trim() === ''
      || typeof result.status !== 'number'
      || !Number.isSafeInteger(result.status)
      || result.status < 100
      || result.status > 599
      || !Object.prototype.hasOwnProperty.call(result, 'body')
    ) {
      throw new ResponseContractError('batch');
    }
  }
  return response as unknown as BatchResponse;
}

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

    if (operations.length > maxBatchOperations) {
      throw new Error(`Batch cannot contain more than ${maxBatchOperations} operations`);
    }

    validateBatchOperationIds(operations);

    const request: BatchRequest = { operations: operations.map(cloneOperation) };
    
    // Use internal client method to make the request
    return parseBatchResponse(
      await (this.client as unknown as BatchCapableClient).postAuth<unknown>('/api/batch', request),
    );
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
    return response.results.filter((r) => !(r.status >= 200 && r.status < 300));
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

  private nextId(id?: string): string {
    if (id) return id;

    let candidate: string;
    do {
      candidate = `op-${++this.idCounter}`;
    } while (this.operations.some((operation) => operation.id === candidate));
    return candidate;
  }

  /**
   * Add GET operation
   */
  get(path: string, id?: string): this {
    this.operations.push({
      id: this.nextId(id),
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
      id: this.nextId(id),
      method: 'POST',
      path,
      body: cloneJson(body),
    });
    return this;
  }

  /**
   * Add PUT operation
   */
  put(path: string, body: any, id?: string): this {
    this.operations.push({
      id: this.nextId(id),
      method: 'PUT',
      path,
      body: cloneJson(body),
    });
    return this;
  }

  /**
   * Add DELETE operation
   */
  delete(path: string, id?: string): this {
    this.operations.push({
      id: this.nextId(id),
      method: 'DELETE',
      path,
    });
    return this;
  }

  /**
   * Add multiple operations at once
   */
  addOperations(ops: BatchOperation[]): this {
    this.operations.push(...ops.map(cloneOperation));
    return this;
  }

  /**
   * Get current operations
   */
  getOperations(): BatchOperation[] {
    return this.operations.map(cloneOperation);
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
    if (this.operations.length > maxBatchOperations) {
      throw new Error(`Batch cannot contain more than ${maxBatchOperations} operations`);
    }

    validateBatchOperationIds(this.operations);

    const request: BatchRequest = { operations: this.operations.map(cloneOperation) };
    return parseBatchResponse(
      await (this.client as unknown as BatchCapableClient).postAuth<unknown>('/api/batch', request),
    );
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
