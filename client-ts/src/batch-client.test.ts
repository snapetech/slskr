import { BatchBuilder, BatchClient, maxBatchOperations } from './batch-client';
import { SlskrClient } from './client';
import { BatchOperation } from './types';

describe('Batch operation limits', () => {
  function mockClient(): { client: SlskrClient; postAuth: jest.Mock } {
    const postAuth = jest.fn();
    return { client: { postAuth } as unknown as SlskrClient, postAuth };
  }

  it('rejects oversized direct batches before sending', async () => {
    const { client, postAuth } = mockClient();
    const batch = new BatchClient(client);
    const operations: BatchOperation[] = Array.from(
      { length: maxBatchOperations + 1 },
      (_, index) => ({ id: `op-${index}`, method: 'GET', path: '/api/health' })
    );

    await expect(batch.execute(operations)).rejects.toThrow('more than 100 operations');
    expect(postAuth).not.toHaveBeenCalled();
  });

  it('enforces the same limit for builder bulk additions', async () => {
    const { client, postAuth } = mockClient();
    const builder = new BatchBuilder(client);
    builder.addOperations(Array.from(
      { length: maxBatchOperations + 1 },
      (_, index) => ({ id: `op-${index}`, method: 'GET', path: '/api/health' })
    ));

    await expect(builder.execute()).rejects.toThrow('more than 100 operations');
    expect(postAuth).not.toHaveBeenCalled();
  });
});
