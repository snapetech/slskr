export const maxPersistedJsonCharacters = 512 * 1024;

export const readBoundedJson = (
  getItem,
  storageKey,
  fallback,
  maxCharacters = maxPersistedJsonCharacters,
) => {
  try {
    const raw = getItem(storageKey, JSON.stringify(fallback));
    if (typeof raw !== 'string' || raw.length > maxCharacters) return fallback;

    const parsed = JSON.parse(raw);
    return parsed === null || parsed === undefined ? fallback : parsed;
  } catch {
    return fallback;
  }
};

export const writeBoundedList = (
  setItem,
  storageKey,
  values,
  {
    maxItems = Number.POSITIVE_INFINITY,
    maxCharacters = maxPersistedJsonCharacters,
  } = {},
) => {
  const itemLimit = Number.isFinite(maxItems)
    ? Math.max(0, Math.floor(maxItems))
    : Number.POSITIVE_INFINITY;
  const entries = (Array.isArray(values) ? values : []).slice(0, itemLimit);

  while (entries.length > 0) {
    const serialized = JSON.stringify(entries);
    if (serialized.length <= maxCharacters) {
      try {
        setItem(storageKey, serialized);
      } catch {
        // Browser persistence is optional and must not break the caller.
      }
      return entries;
    }
    entries.pop();
  }

  try {
    setItem(storageKey, '[]');
  } catch {
    // Browser persistence is optional.
  }
  return [];
};

export const writeBoundedObject = (
  setItem,
  storageKey,
  value,
  {
    maxEntries = Number.POSITIVE_INFINITY,
    maxCharacters = maxPersistedJsonCharacters,
  } = {},
) => {
  const allEntries = Object.entries(
    value && typeof value === 'object' ? value : {},
  );
  const entryLimit = Number.isFinite(maxEntries)
    ? Math.max(0, Math.floor(maxEntries))
    : allEntries.length;
  const entries = allEntries.slice(Math.max(0, allEntries.length - entryLimit));

  while (entries.length > 0) {
    const serialized = JSON.stringify(Object.fromEntries(entries));
    if (serialized.length <= maxCharacters) {
      try {
        setItem(storageKey, serialized);
      } catch {
        // Browser persistence is optional.
      }
      return Object.fromEntries(entries);
    }
    entries.shift();
  }

  try {
    setItem(storageKey, '{}');
  } catch {
    // Browser persistence is optional.
  }
  return {};
};
