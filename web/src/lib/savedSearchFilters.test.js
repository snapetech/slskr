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
});
