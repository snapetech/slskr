import { copyToClipboard } from './clipboard';
import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('copyToClipboard', () => {
  beforeEach(() => {
    Object.assign(navigator, { clipboard: undefined });
  });

  it('reports unavailable clipboard access without throwing', async () => {
    await expect(copyToClipboard('report')).resolves.toBe(false);
  });

  it('returns success only after the browser accepts the write', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    await expect(copyToClipboard('report')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('report');
  });

  it('preserves clipboard write failures for the caller to report', async () => {
    const error = new Error('permission denied');
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockRejectedValue(error) },
    });

    await expect(copyToClipboard('report')).rejects.toBe(error);
  });
});
