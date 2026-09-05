import { v4 as uuidv4 } from 'uuid';
import {
  maxPersistedJsonCharacters,
  readBoundedJson,
  writeBoundedList,
  writeBoundedObject,
} from './persistedJson';

export const communityQualitySignalStorageKey = 'slskr.communityQualitySignals';
export const communityQualityOverrideStorageKey =
  'slskr.communityQualityOverrides';

const maxQualitySignals = 500;
const maxQualityOverrides = 500;
const maxQualityTextCharacters = 2_048;

const positiveSignalTypes = new Set([
  'served-verified-content',
  'queue-reliable',
  'completed-album-consistent',
]);

const negativeSignalTypes = new Set([
  'failed-verification',
  'queue-unreliable',
  'suspicious-candidate',
]);

const normalizeText = (value, fallback = '') => {
  const text =
    typeof value === 'string' || typeof value === 'number'
      ? String(value).trim()
      : '';
  return (text || String(fallback)).slice(0, maxQualityTextCharacters);
};

const normalizeUsername = (username = '') => normalizeText(username);

const getStorage = () => {
  try {
    return window.localStorage;
  } catch (_error) {
    return null;
  }
};

const readSignals = () => {
  const storage = getStorage();
  if (!storage) return [];

  const parsed = readBoundedJson(
    (key, fallback) => storage.getItem(key) || fallback,
    communityQualitySignalStorageKey,
    [],
    maxPersistedJsonCharacters,
  );
  return Array.isArray(parsed)
    ? parsed
        .filter(
          (signal) =>
            signal && typeof signal === 'object' && !Array.isArray(signal),
        )
        .slice(-maxQualitySignals)
        .map(normalizeSignal)
        .filter((signal) => signal.username)
    : [];
};

const readOverrides = () => {
  const storage = getStorage();
  if (!storage) return {};

  const parsed = readBoundedJson(
    (key, fallback) => storage.getItem(key) || fallback,
    communityQualityOverrideStorageKey,
    {},
    maxPersistedJsonCharacters,
  );
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {};

  return Object.fromEntries(
    Object.entries(parsed)
      .map(([username, override]) => [normalizeUsername(username), normalizeOverride(override)])
      .filter(([username]) => username)
      .slice(-maxQualityOverrides),
  );
};

const writeSignals = (signals) => {
  const storage = getStorage();
  if (!storage) return signals;

  return writeBoundedList(
    (key, value) => storage.setItem(key, value),
    communityQualitySignalStorageKey,
    signals,
    {
      maxCharacters: maxPersistedJsonCharacters,
      maxItems: maxQualitySignals,
    },
  );
};

const writeOverrides = (overrides) => {
  const storage = getStorage();
  if (!storage) return overrides;

  return writeBoundedObject(
    (key, value) => storage.setItem(key, value),
    communityQualityOverrideStorageKey,
    overrides,
    {
      maxCharacters: maxPersistedJsonCharacters,
      maxEntries: maxQualityOverrides,
    },
  );
};

const normalizeSignal = (signal = {}) => {
  const sourceSignal = signal && typeof signal === 'object' && !Array.isArray(signal)
    ? signal
    : {};
  const username = normalizeUsername(sourceSignal.username);
  const type = normalizeText(sourceSignal.type) || 'suspicious-candidate';
  const category = positiveSignalTypes.has(type)
    ? 'positive'
    : negativeSignalTypes.has(type)
      ? 'negative'
      : 'neutral';

  return {
    category,
    createdAt: normalizeText(sourceSignal.createdAt, new Date().toISOString()),
    id: normalizeText(sourceSignal.id, `quality-${uuidv4()}`),
    reason: normalizeText(sourceSignal.reason),
    source: normalizeText(sourceSignal.source, 'local-review'),
    type,
    username,
  };
};

export const getCommunityQualitySignals = () => readSignals();

export const saveCommunityQualitySignals = (signals) =>
  writeSignals(
    signals
      .map(normalizeSignal)
      .filter((signal) => signal.username)
      .slice(-500),
  );

export const recordCommunityQualitySignal = (signal) => {
  const normalized = normalizeSignal(signal);
  if (!normalized.username) {
    return getCommunityQualitySignals();
  }

  return saveCommunityQualitySignals([...getCommunityQualitySignals(), normalized]);
};

export const clearCommunityQualitySignalsForUser = (username) => {
  const normalizedUsername = normalizeUsername(username);
  return saveCommunityQualitySignals(
    getCommunityQualitySignals().filter(
      (signal) => signal.username !== normalizedUsername,
    ),
  );
};

const normalizeOverrideMode = (mode) =>
  ['ignore', 'trust', 'caution'].includes(mode) ? mode : 'ignore';

const normalizeOverride = (override = {}) => {
  const sourceOverride =
    override && typeof override === 'object' && !Array.isArray(override)
      ? override
      : {};
  return {
    createdAt: normalizeText(sourceOverride.createdAt, new Date().toISOString()),
    mode: normalizeOverrideMode(sourceOverride.mode),
    note: normalizeText(sourceOverride.note),
    source: normalizeText(sourceOverride.source, 'local-review'),
  };
};

export const getCommunityQualityOverrides = () => readOverrides();

export const setCommunityQualityOverride = (username, override = {}) => {
  const normalizedUsername = normalizeUsername(username);
  if (!normalizedUsername) {
    return getCommunityQualityOverrides();
  }

  return writeOverrides({
    ...getCommunityQualityOverrides(),
    [normalizedUsername]: normalizeOverride(override),
  });
};

export const clearCommunityQualityOverride = (username) => {
  const normalizedUsername = normalizeUsername(username);
  const overrides = { ...getCommunityQualityOverrides() };
  delete overrides[normalizedUsername];
  return writeOverrides(overrides);
};

export const getCommunityQualitySummary = (username) => {
  const normalizedUsername = normalizeUsername(username);
  const signals = getCommunityQualitySignals().filter(
    (signal) => signal.username === normalizedUsername,
  );
  const positive = signals.filter((signal) => signal.category === 'positive').length;
  const negative = signals.filter((signal) => signal.category === 'negative').length;
  const override = getCommunityQualityOverrides()[normalizedUsername] || null;
  const rawScore = Math.min(Math.max((positive * 4) - (negative * 6), -18), 18);
  const score =
    override?.mode === 'ignore'
      ? 0
      : override?.mode === 'trust'
        ? Math.max(rawScore, 8)
        : override?.mode === 'caution'
          ? Math.min(rawScore, -6)
          : rawScore;

  return {
    latestReason: signals[signals.length - 1]?.reason || '',
    negative,
    override,
    positive,
    rawScore,
    score,
    signals,
    username: normalizedUsername,
  };
};

export const getCommunityQualityLabel = (summary) => {
  if (!summary || summary.signals.length === 0) {
    if (!summary?.override) return null;

    return {
      color: summary.override.mode === 'trust' ? 'green' : 'grey',
      icon: summary.override.mode === 'trust' ? 'shield alternate' : 'eye slash',
      text:
        summary.override.mode === 'trust'
          ? 'Local trust override'
          : 'Signals ignored',
    };
  }

  if (summary.override?.mode === 'ignore') {
    return {
      color: 'grey',
      icon: 'eye slash',
      text: 'Signals ignored',
    };
  }

  if (summary.override?.mode === 'trust') {
    return {
      color: 'green',
      icon: 'shield alternate',
      text: 'Local trust override',
    };
  }

  if (summary.override?.mode === 'caution') {
    return {
      color: 'orange',
      icon: 'exclamation triangle',
      text: 'Local caution override',
    };
  }

  if (summary.score >= 8) {
    return {
      color: 'green',
      icon: 'shield alternate',
      text: 'Local trust',
    };
  }

  if (summary.score <= -6) {
    return {
      color: 'orange',
      icon: 'exclamation triangle',
      text: 'Local caution',
    };
  }

  return {
    color: 'violet',
    icon: 'balance scale',
    text: 'Local signals',
  };
};
