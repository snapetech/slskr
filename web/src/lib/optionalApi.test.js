import { describe, expect, it } from 'vitest';
import { readOptionalApiResponse } from './optionalApi';

describe('readOptionalApiResponse', () => {
  it('uses the compatibility fallback only for a missing endpoint', async () => {
    await expect(
      readOptionalApiResponse(() =>
        Promise.reject({ response: { status: 404 } }),
      ),
    ).resolves.toEqual({ data: [] });
  });

  it('propagates authentication and server failures', async () => {
    const error = { response: { status: 401 } };

    await expect(
      readOptionalApiResponse(() => Promise.reject(error)),
    ).rejects.toBe(error);
  });

  it('rejects malformed successful list responses', async () => {
    await expect(
      readOptionalApiResponse(() => Promise.resolve({ data: {} })),
    ).rejects.toThrow('Optional API returned an invalid list response');
  });
});
