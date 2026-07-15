import { getBrowseErrorMessage } from './BrowseSession';

describe('BrowseSession errors', () => {
  it('uses bounded text returned by the daemon', () => {
    expect(
      getBrowseErrorMessage({
        response: { data: 'Unable to browse user; the remote peer is unavailable' },
      }),
    ).toBe('Unable to browse user; the remote peer is unavailable');
  });

  it('uses structured API details and ordinary errors', () => {
    expect(
      getBrowseErrorMessage({ response: { data: { detail: 'Reconnect first' } } }),
    ).toBe('Reconnect first');
    expect(getBrowseErrorMessage(new Error('Browse timed out'))).toBe(
      'Browse timed out',
    );
  });

  it('does not pass structured error objects into React children', () => {
    expect(
      getBrowseErrorMessage({
        response: { data: { detail: { reason: 'nested' }, message: 'Safe text' } },
      }),
    ).toBe('Safe text');
    expect(
      getBrowseErrorMessage({ response: { data: { detail: { reason: 'nested' } } } }),
    ).toBe('Browse failed');
  });
});
