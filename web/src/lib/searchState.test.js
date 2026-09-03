import {
  formatSearchTime,
  getSearchStateKind,
  isSearchComplete,
  parseSearchDate,
  searchDateValue,
} from './searchState';

describe('search state helpers', () => {
  it('recognizes native and universal terminal state spellings', () => {
    expect(isSearchComplete({ state: 'Completed, TimedOut' })).toBe(true);
    expect(isSearchComplete({ state: 'Cancelled' })).toBe(true);
    expect(isSearchComplete({ status: 'failed' })).toBe(true);
    expect(isSearchComplete({ state: 'InProgress' })).toBe(false);
    expect(getSearchStateKind({ state: 'Completed, Errored' })).toBe('failed');
  });

  it('parses Unix seconds and ISO timestamps without producing Invalid Date', () => {
    const unixSeconds = parseSearchDate('1780000000');
    const iso = parseSearchDate('2026-05-15T00:00:00Z');

    expect(unixSeconds?.getTime()).toBe(1_780_000_000_000);
    expect(iso?.toISOString()).toBe('2026-05-15T00:00:00.000Z');
    expect(searchDateValue('not-a-date')).toBe(0);
    expect(formatSearchTime('not-a-date')).toBe('-');
  });
});
