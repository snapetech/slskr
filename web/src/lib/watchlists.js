// <copyright file="watchlists.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import { getLocalStorageItem, setLocalStorageItem } from './storage';
import {
  maxPersistedJsonCharacters,
  readBoundedJson,
  writeBoundedList,
} from './persistedJson';
import {
  acquisitionProfiles,
  defaultAcquisitionProfileId,
  getAcquisitionProfile,
} from './acquisitionProfiles';
import { v4 as uuidv4 } from 'uuid';

export const watchlistStorageKey = 'slskr.watchlists.items';

const allowedKinds = ['Artist', 'Label', 'Playlist', 'Collection'];
const allowedReleaseTypes = [
  'Album',
  'EP',
  'Single',
  'Compilation',
  'Live',
  'Remix',
  'Deluxe',
];
const allowedCountries = ['Any', 'US', 'GB', 'CA', 'JP', 'DE', 'FR', 'BR', 'AU'];
const allowedFormats = ['Any', 'Digital', 'CD', 'Vinyl', 'Cassette'];
const allowedSchedules = ['Manual only', 'Daily', 'Weekly', 'Monthly'];
const maxWatchlists = 500;
const maxWatchlistTextCharacters = 2_048;
const maxExpansionCandidates = 100;

const toDropdownOptions = (values) =>
  values.map((value) => ({
    key: value.toLowerCase(),
    text: value,
    value,
  }));

export const watchlistKindOptions = toDropdownOptions(allowedKinds);
export const watchlistReleaseTypeOptions = toDropdownOptions(allowedReleaseTypes);
export const watchlistCountryOptions = toDropdownOptions(allowedCountries);
export const watchlistFormatOptions = toDropdownOptions(allowedFormats);
export const watchlistScheduleOptions = toDropdownOptions(allowedSchedules);
export const watchlistAcquisitionProfileOptions = acquisitionProfiles.map((profile) => ({
  key: profile.id,
  text: profile.label,
  value: profile.id,
}));

const now = () => new Date().toISOString();

const normalizeText = (value, fallback = '') =>
  (typeof value === 'string' || typeof value === 'number'
    ? String(value)
    : fallback)
    .trim()
    .slice(0, maxWatchlistTextCharacters);

const isPlainObject = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

const normalizeReleaseTypes = (releaseTypes = []) =>
  (Array.isArray(releaseTypes) ? releaseTypes : [])
    .filter((releaseType) => allowedReleaseTypes.includes(releaseType));

const normalizeCountry = (country) =>
  allowedCountries.includes(country) ? country : 'Any';

const normalizeFormat = (format) => (allowedFormats.includes(format) ? format : 'Any');

const normalizeSchedule = (schedule) =>
  allowedSchedules.includes(schedule) ? schedule : 'Manual only';

const normalizeCooldownDays = (cooldownDays) => {
  const parsed = Number(cooldownDays);

  if (!Number.isFinite(parsed)) {
    return 7;
  }

  return Math.min(Math.max(Math.round(parsed), 1), 30);
};

const normalizeExpansionCandidates = (candidates = []) => {
  const names = Array.isArray(candidates) ? candidates : [];
  const seen = new Set();

  return names
    .map((candidate) =>
      typeof candidate === 'string' ? { name: candidate } : candidate,
    )
    .filter(isPlainObject)
    .map((candidate) => ({
      createdAt: normalizeText(candidate.createdAt, now()),
      decidedAt: normalizeText(candidate.decidedAt),
      name: normalizeText(candidate.name),
      status: ['Approved', 'Rejected'].includes(candidate.status)
        ? candidate.status
        : 'Pending',
    }))
    .filter((candidate) => {
      const key = candidate.name.toLowerCase();
      if (!candidate.name || seen.has(key)) {
        return false;
      }

      seen.add(key);
      return true;
    })
    .slice(0, maxExpansionCandidates);
};

const normalizeWatchlist = (item = {}) => {
  const sourceItem = isPlainObject(item) ? item : {};
  const timestamp = now();

  return {
    acquisitionProfile:
      getAcquisitionProfile(sourceItem.acquisitionProfile)?.id ?? defaultAcquisitionProfileId,
    cooldownDays: normalizeCooldownDays(sourceItem.cooldownDays),
    country: normalizeCountry(sourceItem.country),
    createdAt: normalizeText(sourceItem.createdAt, timestamp),
    destination: normalizeText(sourceItem.destination, 'Discovery Inbox'),
    expansionCandidates: normalizeExpansionCandidates(sourceItem.expansionCandidates),
    expansionSource: normalizeText(sourceItem.expansionSource),
    format: normalizeFormat(sourceItem.format),
    id: normalizeText(sourceItem.id, uuidv4()),
    kind: allowedKinds.includes(sourceItem.kind) ? sourceItem.kind : 'Artist',
    lastScannedAt: normalizeText(sourceItem.lastScannedAt),
    lastScanPreview: normalizeText(sourceItem.lastScanPreview),
    releaseTypes:
      normalizeReleaseTypes(sourceItem.releaseTypes).length > 0
        ? normalizeReleaseTypes(sourceItem.releaseTypes)
        : ['Album', 'EP', 'Single'],
    schedule: normalizeSchedule(sourceItem.schedule),
    target: normalizeText(sourceItem.target, 'Untitled watch'),
    updatedAt: normalizeText(sourceItem.updatedAt, timestamp),
  };
};

const getWatchlistsWith = (getItem = getLocalStorageItem) => {
  const parsed = readBoundedJson(
    getItem,
    watchlistStorageKey,
    [],
    maxPersistedJsonCharacters,
  );
  return Array.isArray(parsed)
    ? parsed.filter(isPlainObject).slice(0, maxWatchlists).map(normalizeWatchlist)
    : [];
};

const saveWatchlistsWith = (items, setItem = setLocalStorageItem) => {
  const normalized = (Array.isArray(items) ? items : [])
    .filter(isPlainObject)
    .map(normalizeWatchlist)
    .slice(0, maxWatchlists);
  return writeBoundedList(setItem, watchlistStorageKey, normalized, {
    maxCharacters: maxPersistedJsonCharacters,
    maxItems: maxWatchlists,
  });
};

export const getWatchlists = () => getWatchlistsWith();

export const saveWatchlist = (
  item,
  {
    getItem = getLocalStorageItem,
    setItem = setLocalStorageItem,
  } = {},
) => {
  const next = normalizeWatchlist(item);
  const existing = getWatchlistsWith(getItem).filter(
    (watch) =>
      !(
        watch.kind === next.kind &&
        watch.target.toLowerCase() === next.target.toLowerCase()
      ),
  );

  return saveWatchlistsWith([next, ...existing], setItem);
};

export const recordWatchlistManualScan = (
  id,
  {
    getItem = getLocalStorageItem,
    setItem = setLocalStorageItem,
    timestamp = now(),
  } = {},
) => {
  const updated = getWatchlistsWith(getItem).map((item) =>
    item.id === id
      ? {
          ...item,
          lastScannedAt: timestamp,
          lastScanPreview:
            'Manual scan preview only; no provider lookup or peer search was started.',
          updatedAt: timestamp,
        }
      : item,
  );

  return saveWatchlistsWith(updated, setItem);
};

export const recordWatchlistExpansionDecision = (
  id,
  candidateName,
  decision,
  {
    getItem = getLocalStorageItem,
    setItem = setLocalStorageItem,
    timestamp = now(),
  } = {},
) => {
  const normalizedName = candidateName.trim();
  const status = decision === 'Approved' ? 'Approved' : 'Rejected';
  const items = getWatchlistsWith(getItem);
  const parent = items.find((item) => item.id === id);

  const updated = items.map((item) =>
    item.id === id
      ? {
          ...item,
          expansionCandidates: item.expansionCandidates.map((candidate) =>
            candidate.name.toLowerCase() === normalizedName.toLowerCase()
              ? {
                  ...candidate,
                  decidedAt: timestamp,
                  status,
                }
              : candidate,
          ),
          updatedAt: timestamp,
        }
      : item,
  );

  const candidateExists = updated.some(
    (item) =>
      item.kind === 'Artist' &&
      item.target.toLowerCase() === normalizedName.toLowerCase(),
  );

  if (status === 'Approved' && parent && normalizedName && !candidateExists) {
    updated.unshift(
      normalizeWatchlist({
        acquisitionProfile: parent.acquisitionProfile,
        cooldownDays: parent.cooldownDays,
        country: parent.country,
        expansionSource: parent.target,
        format: parent.format,
        kind: 'Artist',
        releaseTypes: parent.releaseTypes,
        schedule: 'Manual only',
        target: normalizedName,
      }),
    );
  }

  return saveWatchlistsWith(updated, setItem);
};

export const buildWatchlistSummary = (items = []) =>
  items.reduce(
    (summary, item) => ({
      ...summary,
      [item.kind]: (summary[item.kind] || 0) + 1,
      scheduled:
        item.schedule === 'Manual only' ? summary.scheduled : summary.scheduled + 1,
      total: summary.total + 1,
    }),
    {
      Artist: 0,
      Collection: 0,
      Label: 0,
      Playlist: 0,
      scheduled: 0,
      total: 0,
    },
  );

export const buildWatchlistDiscoverySeed = (item) => ({
  acquisitionProfile: item.acquisitionProfile,
  evidenceKey: `watchlist:${item.kind}:${item.target}`.toLowerCase(),
  networkImpact:
    'Watchlist review seed only; no provider lookup, Soulseek search, peer browse, or download has started.',
  reason: `${item.kind} watchlist target using ${item.releaseTypes.join(', ')} releases, ${item.country} country, and ${item.format} format filters.`,
  searchText: item.target,
  source: 'Watchlist',
  sourceId: item.id,
  title: item.target,
});

export const buildWatchlistSchedulePreview = (item) => {
  const profile = getAcquisitionProfile(item.acquisitionProfile);
  const enabled = item.schedule !== 'Manual only';

  return {
    cooldown: `${item.cooldownDays} day${item.cooldownDays === 1 ? '' : 's'}`,
    enabled,
    label: enabled ? `${item.schedule} schedule visible` : 'Manual scans only',
    networkImpact: enabled
      ? 'Scheduled watchlist scans are enabled in the plan, but this local preview does not execute provider lookups or peer searches.'
      : 'Manual scan previews only; no scheduled provider lookup, peer search, or download is enabled.',
    profileLabel: profile.label,
  };
};

export const buildWatchlistExpansionSummary = (item) =>
  normalizeExpansionCandidates(item.expansionCandidates).reduce(
    (summary, candidate) => ({
      ...summary,
      [candidate.status]: (summary[candidate.status] || 0) + 1,
      total: summary.total + 1,
    }),
    {
      Approved: 0,
      Pending: 0,
      Rejected: 0,
      total: 0,
    },
  );
