import { describe, expect, it } from 'vitest';
import { toDisplayError } from './errors';

describe('toDisplayError', () => {
  it('prefers a structured API error message', () => {
    expect(toDisplayError({ response: { data: { error: 'validation failed' } } })).toBe(
      'validation failed',
    );
  });

  it('serializes structured errors without exposing an object to React', () => {
    expect(toDisplayError({ response: { data: { code: 'invalid' } } })).toBe(
      '{"code":"invalid"}',
    );
  });

  it('uses a fallback for empty or non-displayable errors', () => {
    expect(toDisplayError(null, 'Unavailable')).toBe('Unavailable');
  });
});
