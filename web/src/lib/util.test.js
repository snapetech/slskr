import * as utils from './util';

describe('formatBytesAsUnit', () => {
  it('converts bytes to specified unit', () => {
    expect(utils.formatBytesAsUnit(1_234_567, 'MB', 2)).toBe(1.18);
  });
});

describe('dashboard formatting helpers', () => {
  it('formats speeds and queue waits like slskd', () => {
    expect(utils.formatSpeed(1_024)).toBe('1 KB/s');
    expect(utils.formatWait(30)).toBe('30s');
    expect(utils.formatWait(90)).toBe('1.5m');
  });

  it('truncates long diagnostic text with an ellipsis', () => {
    expect(utils.truncate('abcdefgh', 5)).toBe('abcde...');
  });
});
