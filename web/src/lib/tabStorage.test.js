import {
  maxStoredTabs,
  maxTabStorageCharacters,
  readBoundedTabState,
  writeBoundedTabState,
} from './tabStorage';

describe('tabStorage', () => {
  it('rejects oversized persisted state before parsing', () => {
    const getItem = () => 'x'.repeat(maxTabStorageCharacters + 1);

    expect(readBoundedTabState(getItem, 'tabs')).toEqual({
      tabCounter: 0,
      tabs: [],
    });
  });

  it('retains only the newest bounded tab set when persisting', () => {
    let saved = '';
    const tabs = Array.from({ length: maxStoredTabs + 4 }, (_, index) => ({
      key: `tab-${index}`,
      label: `Tab ${index}`,
    }));

    writeBoundedTabState(
      (_key, value) => {
        saved = value;
      },
      'tabs',
      Number.MAX_SAFE_INTEGER,
      tabs,
    );

    const parsed = JSON.parse(saved);
    expect(parsed.tabCounter).toBe(2 ** 31 - 1);
    expect(parsed.tabs).toHaveLength(maxStoredTabs);
    expect(parsed.tabs[0].key).toBe('tab-4');
  });
});
