import {
  getLocalStorageItem,
  setLocalStorageItem,
} from './storage';

const savedSearchFiltersKey = 'slskr-saved-search-filters';
const legacySavedSearchFiltersKey = 'slskd-saved-search-filters';

const parseSavedFilters = () => {
  try {
    const stored = getLocalStorageItem(savedSearchFiltersKey);
    const legacyStored =
      stored === null
        ? getLocalStorageItem(legacySavedSearchFiltersKey)
        : null;
    const value = stored ?? legacyStored ?? '[]';
    if (stored === null && legacyStored !== null) {
      setLocalStorageItem(savedSearchFiltersKey, legacyStored);
    }
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
};

export const getSavedSearchFilters = () =>
  parseSavedFilters()
    .filter((filter) => filter?.name && filter?.value)
    .sort((left, right) => left.name.localeCompare(right.name));

export const saveSearchFilter = ({ name, value }) => {
  const trimmedName = (name || '').trim();
  const trimmedValue = (value || '').trim();

  if (!trimmedName || !trimmedValue) {
    return getSavedSearchFilters();
  }

  const next = [
    ...getSavedSearchFilters().filter(
      (filter) => filter.name.toLowerCase() !== trimmedName.toLowerCase(),
    ),
    { name: trimmedName, value: trimmedValue },
  ].sort((left, right) => left.name.localeCompare(right.name));

  setLocalStorageItem(savedSearchFiltersKey, JSON.stringify(next));
  return next;
};

export const removeSavedSearchFilter = (name) => {
  const trimmedName = (name || '').trim();
  const next = getSavedSearchFilters().filter(
    (filter) => filter.name.toLowerCase() !== trimmedName.toLowerCase(),
  );

  setLocalStorageItem(savedSearchFiltersKey, JSON.stringify(next));
  return next;
};
