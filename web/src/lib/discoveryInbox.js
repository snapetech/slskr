import { getLocalStorageItem, setLocalStorageItem } from './storage';
import {
  maxPersistedJsonCharacters,
  readBoundedJson,
  writeBoundedList,
} from './persistedJson';
import { v4 as uuidv4 } from 'uuid';

export const discoveryInboxStorageKey = 'slskr.discoveryInbox.items';

export const discoveryInboxStates = [
  'Suggested',
  'Approved',
  'Downloading',
  'Staged',
  'Imported',
  'Rejected',
  'Snoozed',
  'Failed',
];

export const defaultDiscoveryInboxState = 'Suggested';

const maxDiscoveryInboxItems = 500;
const maxDiscoveryInboxTextCharacters = 2_048;

const now = () => new Date().toISOString();
const daysFromNow = (days, timestamp = Date.now()) =>
  new Date(timestamp + days * 24 * 60 * 60 * 1_000).toISOString();

const normalizeState = (state) =>
  discoveryInboxStates.includes(state) ? state : defaultDiscoveryInboxState;

const normalizeText = (value, fallback = '') =>
  (typeof value === 'string' || typeof value === 'number'
    ? String(value)
    : fallback)
    .trim()
    .slice(0, maxDiscoveryInboxTextCharacters);

const normalizeItem = (item = {}) => {
  const sourceItem = item && typeof item === 'object' && !Array.isArray(item)
    ? item
    : {};
  const timestamp = now();
  const title = normalizeText(
    sourceItem.title || sourceItem.searchText,
    'Untitled discovery',
  );

  return {
    acquisitionProfile: normalizeText(sourceItem.acquisitionProfile, 'lossless-exact'),
    createdAt: normalizeText(sourceItem.createdAt, timestamp),
    evidenceKey: normalizeText(
      sourceItem.evidenceKey || title || sourceItem.searchText,
      uuidv4(),
    ),
    id: normalizeText(sourceItem.id, uuidv4()),
    networkImpact: normalizeText(
      sourceItem.networkImpact,
      'Manual review; no network request until approved.',
    ),
    reason: normalizeText(sourceItem.reason, 'Manual discovery suggestion.'),
    searchText: normalizeText(sourceItem.searchText || title),
    source: normalizeText(sourceItem.source, 'Manual'),
    sourceId: normalizeText(sourceItem.sourceId),
    state: normalizeState(sourceItem.state),
    snoozedUntil: normalizeText(sourceItem.snoozedUntil),
    title,
    updatedAt: normalizeText(sourceItem.updatedAt, timestamp),
  };
};

export const getDiscoveryInboxItems = (getItem = getLocalStorageItem) => {
  const parsed = readBoundedJson(
    getItem,
    discoveryInboxStorageKey,
    [],
    maxPersistedJsonCharacters,
  );
  return Array.isArray(parsed)
    ? parsed
        .filter(
          (item) => item && typeof item === 'object' && !Array.isArray(item),
        )
        .slice(0, maxDiscoveryInboxItems)
        .map(normalizeItem)
    : [];
};

export const saveDiscoveryInboxItems = (
  items,
  setItem = setLocalStorageItem,
) => {
  const normalized = (Array.isArray(items) ? items : [])
    .map(normalizeItem)
    .slice(0, maxDiscoveryInboxItems);
  return writeBoundedList(setItem, discoveryInboxStorageKey, normalized, {
    maxCharacters: maxPersistedJsonCharacters,
    maxItems: maxDiscoveryInboxItems,
  });
};

export const addDiscoveryInboxItem = (
  item,
  {
    getItem = getLocalStorageItem,
    setItem = setLocalStorageItem,
  } = {},
) => {
  const items = getDiscoveryInboxItems(getItem);
  const nextItem = normalizeItem(item);
  const duplicate = items.find(
    (existing) =>
      existing.evidenceKey === nextItem.evidenceKey &&
      existing.source === nextItem.source,
  );

  if (duplicate) {
    return duplicate;
  }

  saveDiscoveryInboxItems([nextItem, ...items], setItem);
  return nextItem;
};

export const updateDiscoveryInboxItemState = (
  id,
  state,
  {
    getItem = getLocalStorageItem,
    setItem = setLocalStorageItem,
  } = {},
) => {
  const nextState = normalizeState(state);
  const updated = getDiscoveryInboxItems(getItem).map((item) =>
    item.id === id
      ? {
          ...item,
          snoozedUntil: nextState === 'Snoozed' ? item.snoozedUntil : '',
          state: nextState,
          updatedAt: now(),
        }
      : item,
  );

  return saveDiscoveryInboxItems(updated, setItem);
};

export const snoozeDiscoveryInboxItem = (
  id,
  days = 7,
  {
    getItem = getLocalStorageItem,
    setItem = setLocalStorageItem,
    timestamp = Date.now(),
  } = {},
) => {
  const updated = getDiscoveryInboxItems(getItem).map((item) =>
    item.id === id
      ? {
          ...item,
          snoozedUntil: daysFromNow(days, timestamp),
          state: 'Snoozed',
          updatedAt: now(),
        }
      : item,
  );

  return saveDiscoveryInboxItems(updated, setItem);
};

export const getDiscoveryInboxSnoozeStatus = (
  item,
  timestamp = Date.now(),
) => {
  if (item?.state !== 'Snoozed') {
    return null;
  }

  const dueAt = Date.parse(item.snoozedUntil || '');
  if (Number.isNaN(dueAt)) {
    return {
      color: 'grey',
      isDue: false,
      label: 'Snoozed',
    };
  }

  return {
    color: dueAt <= timestamp ? 'orange' : 'grey',
    isDue: dueAt <= timestamp,
    label: dueAt <= timestamp ? 'Snooze due' : 'Snoozed until',
  };
};

export const bulkUpdateDiscoveryInboxItems = (
  ids,
  state,
  {
    getItem = getLocalStorageItem,
    setItem = setLocalStorageItem,
  } = {},
) => {
  const idSet = new Set(Array.isArray(ids) ? ids : []);
  const nextState = normalizeState(state);
  const updated = getDiscoveryInboxItems(getItem).map((item) =>
    idSet.has(item.id)
      ? { ...item, state: nextState, updatedAt: now() }
      : item,
  );

  return saveDiscoveryInboxItems(updated, setItem);
};
