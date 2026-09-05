export const maxStoredTabs = 64;
export const maxTabTextCharacters = 512;
export const maxTabStorageCharacters = 128 * 1024;

const maxTabCounter = 2 ** 31 - 1;

export const boundedTabText = (value) =>
  (typeof value === 'string' || typeof value === 'number'
    ? String(value)
    : '')
    .trim()
    .slice(0, maxTabTextCharacters);

export const readBoundedTabState = (getItem, storageKey) => {
  try {
    const raw = getItem(storageKey, '{}');
    if (typeof raw !== 'string' || raw.length > maxTabStorageCharacters) {
      return { tabCounter: 0, tabs: [] };
    }

    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      return { tabCounter: 0, tabs: [] };
    }

    return {
      tabCounter:
        Number.isSafeInteger(parsed.tabCounter) && parsed.tabCounter >= 0
          ? Math.min(parsed.tabCounter, maxTabCounter)
          : 0,
      tabs: Array.isArray(parsed.tabs) ? parsed.tabs.slice(-maxStoredTabs) : [],
    };
  } catch {
    return { tabCounter: 0, tabs: [] };
  }
};

export const writeBoundedTabState = (
  setItem,
  storageKey,
  tabCounter,
  tabs,
) => {
  const boundedCounter =
    Number.isSafeInteger(tabCounter) && tabCounter >= 0
      ? Math.min(tabCounter, maxTabCounter)
      : 0;
  const entries = (Array.isArray(tabs) ? tabs : []).slice(-maxStoredTabs);

  while (entries.length > 0) {
    const serialized = JSON.stringify({
      tabCounter: boundedCounter,
      tabs: entries,
    });
    if (serialized.length <= maxTabStorageCharacters) {
      setItem(storageKey, serialized);
      return entries;
    }
    entries.shift();
  }

  setItem(storageKey, JSON.stringify({ tabCounter: boundedCounter, tabs: [] }));
  return [];
};
