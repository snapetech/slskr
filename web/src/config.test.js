import { normalizeUrlBase } from './config';

describe('normalizeUrlBase', () => {
  it('normalizes the root and subpath forms used by the daemon', () => {
    expect(normalizeUrlBase('/')).toBe('');
    expect(normalizeUrlBase('//')).toBe('');
    expect(normalizeUrlBase('/slsk/')).toBe('/slsk');
    expect(normalizeUrlBase(' /nested/slsk/// ')).toBe('/nested/slsk');
  });

  it('rejects URL-like and traversing values', () => {
    expect(normalizeUrlBase('https://example.test/slsk')).toBe('');
    expect(normalizeUrlBase('/slsk?next=/admin')).toBe('');
    expect(normalizeUrlBase('/slsk/../admin')).toBe('');
  });
});
