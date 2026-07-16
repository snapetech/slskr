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

  it('owns nested bodies added directly and returned in snapshots', () => {
    const { client } = mockClient();
    const filters = ['lossless'];
    const body = { query: 'ambient', options: { filters } };
    const builder = new BatchBuilder(client).post('/api/searches', body);

    filters[0] = 'mutated input';
    body.query = 'mutated input';
    const snapshot = builder.getOperations();
    snapshot[0].body.query = 'mutated snapshot';
    snapshot[0].body.options.filters[0] = 'mutated snapshot';

    expect(builder.getOperations()[0].body).toEqual({
      query: 'ambient',
      options: { filters: ['lossless'] },
    });
  });

  it('owns nested bodies from bulk additions and direct execution', async () => {
    const { client, postAuth } = mockClient();
    postAuth.mockResolvedValue({ results: [], total_time_ms: 0 });
    const directBody = { filters: ['direct'] };
    const bulkBody = { filters: ['bulk'] };
    const direct = [{ id: 'direct', method: 'POST', path: '/api/searches', body: directBody }] as BatchOperation[];
    const builder = new BatchBuilder(client).addOperations([
      { id: 'bulk', method: 'POST', path: '/api/searches', body: bulkBody },
    ]);

    await new BatchClient(client).execute(direct);
    await builder.execute();
    directBody.filters[0] = 'mutated';
    bulkBody.filters[0] = 'mutated';

    expect(postAuth.mock.calls[0][1].operations[0].body.filters).toEqual(['direct']);
    expect(postAuth.mock.calls[1][1].operations[0].body.filters).toEqual(['bulk']);
  });
});
