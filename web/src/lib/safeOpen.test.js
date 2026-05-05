import { describe, expect, it, vi } from 'vitest';
import { safeOpenBlank } from './safeOpen';

describe('safeOpenBlank', () => {
  it('opens links with opener isolation', () => {
    const opened = { opener: {} };
    const open = vi.spyOn(window, 'open').mockReturnValue(opened);

    expect(safeOpenBlank('/api/v0/streams/item')).toBe(opened);
    expect(open).toHaveBeenCalledWith(
      '/api/v0/streams/item',
      '_blank',
      'noopener,noreferrer',
    );
    expect(opened.opener).toBeNull();

    open.mockRestore();
  });
});
