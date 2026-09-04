import {
  formatSearchTime,
  getSearchStateKind,
  isSearchComplete,
  mergeSearchRecords,
  parseSearchDate,
  searchDateValue,
} from './searchState';

describe('search state helpers', () => {
  it('recognizes native and universal terminal state spellings', () => {
    expect(isSearchComplete({ state: 'Completed, TimedOut' })).toBe(true);
    expect(isSearchComplete({ state: 'Cancelled' })).toBe(true);
    expect(getSearchStateKind({ status: 'expired' })).toBe('completed');
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

  it('does not let a stale active hub update reopen a terminal search', () => {
    expect(
      mergeSearchRecords(
        {
          endedAt: '2026-09-04T00:00:00Z',
          isComplete: true,
          state: 'Cancelled',
          status: 'cancelled',
        },
        { isComplete: false, state: 'InProgress', status: 'active' },
      ),
    ).toMatchObject({
      endedAt: '2026-09-04T00:00:00Z',
      isComplete: true,
      state: 'Cancelled',
      status: 'cancelled',
    });
  });

  it('fills missing terminal fields from the preserved state kind', () => {
    expect(
      mergeSearchRecords(
        { isComplete: true, state: 'Cancelled' },
        { isComplete: false, status: 'active', state: 'InProgress' },
      ),
    ).toMatchObject({
      isComplete: true,
      state: 'Cancelled',
      status: 'cancelled',
    });
  });
});
