import { describe, expect, it, vi } from 'vitest';
import { isSafeBlankUrl, safeOpenBlank } from './safeOpen';

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

  it.each(['javascript:alert(1)', 'data:text/html,<script>alert(1)</script>'])(
    'rejects unsafe URL schemes: %s',
    (url) => {
      const open = vi.spyOn(window, 'open');

      expect(isSafeBlankUrl(url)).toBe(false);
      expect(safeOpenBlank(url)).toBeNull();
      expect(open).not.toHaveBeenCalled();

      open.mockRestore();
    },
  );
});
