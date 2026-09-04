import { describe, expect, it } from 'vitest';
import { isAbortError } from './api';

describe('isAbortError', () => {
  it('recognizes browser and fetch-style abort errors', () => {
    expect(isAbortError(new DOMException('aborted', 'AbortError'))).toBe(true);
    expect(isAbortError(Object.assign(new Error('aborted'), { name: 'AbortError' }))).toBe(true);
  });

  it('does not classify ordinary errors as aborts', () => {
    expect(isAbortError(new Error('failed'))).toBe(false);
    expect(isAbortError({ name: 'NetworkError' })).toBe(false);
    expect(isAbortError(null)).toBe(false);
  });
});
