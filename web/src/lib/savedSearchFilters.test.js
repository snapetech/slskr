import {
  getSavedSearchFilters,
  removeSavedSearchFilter,
  saveSearchFilter,
} from './savedSearchFilters';
import * as storage from './storage';

vi.mock('./storage', () => ({
  getLocalStorageItem: vi.fn(),
  setLocalStorageItem: vi.fn(),
}));

describe('saved search filters', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    storage.getLocalStorageItem.mockReturnValue('[]');
  });

  it('normalizes, sorts, and replaces filters by name', () => {
    storage.getLocalStorageItem.mockReturnValue(
      JSON.stringify([{ name: 'Existing', value: 'old' }]),
    );

    expect(saveSearchFilter({ name: ' existing ', value: '  new  ' })).toEqual([
      { name: 'existing', value: 'new' },
    ]);
    expect(storage.setLocalStorageItem).toHaveBeenCalledWith(
      'slskr-saved-search-filters',
      JSON.stringify([{ name: 'existing', value: 'new' }]),
    );
  });

  it('removes a filter case-insensitively', () => {
    storage.getLocalStorageItem.mockReturnValueOnce(
      JSON.stringify([{ name: 'Lossless', value: 'flac' }]),
    );

    expect(removeSavedSearchFilter(' lossLESS ')).toEqual([]);
    expect(getSavedSearchFilters()).toEqual([]);
  });

  it('rejects oversized and malformed persisted filters', () => {
    storage.getLocalStorageItem.mockReturnValue(
      `[${JSON.stringify({ name: 'valid', value: 'ok' })},${JSON.stringify({ name: 'x'.repeat(3_000), value: 'ignored' })}]`,
    );

    expect(getSavedSearchFilters()).toHaveLength(2);
    expect(getSavedSearchFilters()[1].name).toHaveLength(2_048);

    storage.getLocalStorageItem.mockReturnValue('x'.repeat(512 * 1024 + 1));
    expect(getSavedSearchFilters()).toEqual([]);
  });
});
