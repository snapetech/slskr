import {
  getLocalStorageItem,
  setLocalStorageItem,
} from './storage';
import {
  maxPersistedJsonCharacters,
  readBoundedJson,
  writeBoundedList,
} from './persistedJson';

const savedSearchFiltersKey = 'slskr-saved-search-filters';
const legacySavedSearchFiltersKey = 'slskd-saved-search-filters';
const maxSavedSearchFilters = 100;
const maxSavedSearchFilterTextCharacters = 2_048;

const normalizeText = (value) =>
  typeof value === 'string' || typeof value === 'number'
    ? String(value).trim().slice(0, maxSavedSearchFilterTextCharacters)
    : '';

const normalizeFilter = (filter) => {
  if (!filter || typeof filter !== 'object' || Array.isArray(filter)) return null;

  const name = normalizeText(filter.name);
  const value = normalizeText(filter.value);
  return name && value ? { name, value } : null;
};

const parseFilterValue = (raw) => {
  const parsed = readBoundedJson(
    (_key, fallback) => (typeof raw === 'string' ? raw : fallback),
    savedSearchFiltersKey,
    [],
    maxPersistedJsonCharacters,
  );

  return Array.isArray(parsed)
    ? parsed
        .slice(0, maxSavedSearchFilters)
        .map(normalizeFilter)
        .filter(Boolean)
    : [];
};

const parseSavedFilters = () => {
  const stored = getLocalStorageItem(savedSearchFiltersKey);
  const legacyStored =
    stored === null
      ? getLocalStorageItem(legacySavedSearchFiltersKey)
      : null;
  const filters = parseFilterValue(stored ?? legacyStored);

  if (stored === null && legacyStored !== null) {
    writeBoundedList(
      setLocalStorageItem,
      savedSearchFiltersKey,
      filters,
      {
        maxCharacters: maxPersistedJsonCharacters,
        maxItems: maxSavedSearchFilters,
      },
    );
  }

  return filters;
};

export const getSavedSearchFilters = () =>
  parseSavedFilters()
    .filter((filter) => filter?.name && filter?.value)
    .sort((left, right) => left.name.localeCompare(right.name));

export const saveSearchFilter = ({ name, value } = {}) => {
  const trimmedName = normalizeText(name);
  const trimmedValue = normalizeText(value);

  if (!trimmedName || !trimmedValue) {
    return getSavedSearchFilters();
  }

  const next = [
    ...getSavedSearchFilters().filter(
      (filter) => filter.name.toLowerCase() !== trimmedName.toLowerCase(),
    ),
    { name: trimmedName, value: trimmedValue },
  ].sort((left, right) => left.name.localeCompare(right.name));

  writeBoundedList(setLocalStorageItem, savedSearchFiltersKey, next, {
    maxCharacters: maxPersistedJsonCharacters,
    maxItems: maxSavedSearchFilters,
  });
  return next;
};

export const removeSavedSearchFilter = (name) => {
  const trimmedName = normalizeText(name);
  const next = getSavedSearchFilters().filter(
    (filter) => filter.name.toLowerCase() !== trimmedName.toLowerCase(),
  );

  writeBoundedList(setLocalStorageItem, savedSearchFiltersKey, next, {
    maxCharacters: maxPersistedJsonCharacters,
    maxItems: maxSavedSearchFilters,
  });
  return next;
};
