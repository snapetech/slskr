import api from './api';
import { getLocalStorageItem, setLocalStorageItem } from './storage';
import { v4 as uuidv4 } from 'uuid';

const USER_DOWNLOAD_STATS_CACHE_TTL_MS = 30_000;
let userDownloadStatsCache = null;
let userDownloadStatsCacheExpiresAt = 0;
let userDownloadStatsInflight = null;

export const getAll = async (limit = 500, source = null) => {
  const params = new URLSearchParams({ limit: String(limit) });
  if (source && source !== 'all') {
    params.set('source', source);
  }
  return (await api.get(`/searches?${params.toString()}`)).data;
};

export const cleanupSearches = () => {
  return api.post('/searches/cleanup');
};

export const get = async ({ id }) => {
  return (await api.get(`/searches/${encodeURIComponent(id)}`)).data;
};

export const stop = ({ id }) => {
  return api.put(`/searches/${encodeURIComponent(id)}`);
};

export const remove = ({ id }) => {
  return api.delete(`/searches/${encodeURIComponent(id)}`);
};

export const removeAll = () => {
  return api.delete('/searches');
};

// User download stats for badges
export const getUserDownloadStats = () => {
  if (
    userDownloadStatsCache &&
    userDownloadStatsCacheExpiresAt > Date.now()
  ) {
    return Promise.resolve(userDownloadStatsCache);
  }

  if (userDownloadStatsInflight) {
    return userDownloadStatsInflight;
  }

  userDownloadStatsInflight = api
    .get('/transfers/downloads/user-stats')
    .then((response) => {
      const stats =
        response.data &&
        typeof response.data === 'object' &&
        !Array.isArray(response.data)
          ? response.data
          : {};
      userDownloadStatsCache = stats;
      userDownloadStatsCacheExpiresAt =
        Date.now() + USER_DOWNLOAD_STATS_CACHE_TTL_MS;
      return stats;
    })
    .finally(() => {
      userDownloadStatsInflight = null;
    });

  return userDownloadStatsInflight;
};

// Blocked users management (localStorage-based)
const BLOCKED_USERS_KEY = 'slskr_blocked_users';
const LEGACY_BLOCKED_USERS_KEY = 'slskdn_blocked_users';

export const getBlockedUsers = () => {
  try {
    const current = getLocalStorageItem(BLOCKED_USERS_KEY);
    const blocked =
      current ?? getLocalStorageItem(LEGACY_BLOCKED_USERS_KEY);
    if (current === null && blocked !== null) {
      setLocalStorageItem(BLOCKED_USERS_KEY, blocked);
    }
    const parsed = blocked ? JSON.parse(blocked) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
};

export const blockUser = (username) => {
  const blocked = getBlockedUsers();
  if (!blocked.includes(username)) {
    blocked.push(username);
    setLocalStorageItem(BLOCKED_USERS_KEY, JSON.stringify(blocked));
  }

  return blocked;
};

export const unblockUser = (username) => {
  let blocked = getBlockedUsers();
  blocked = blocked.filter((u) => u !== username);
  setLocalStorageItem(BLOCKED_USERS_KEY, JSON.stringify(blocked));
  return blocked;
};

export const isUserBlocked = (username) => {
  return getBlockedUsers().includes(username);
};

export const create = ({
  acquisitionProfile = null,
  id,
  searchText,
  providers = null,
}) => {
  const body = { id, searchText };

  if (acquisitionProfile) {
    body.acquisitionProfile = acquisitionProfile;
  }

  // Include providers if specified (for Scene ↔ Pod Bridging)
  if (providers && Array.isArray(providers)) {
    body.providers = providers;
  }

  return api.post('/searches', body);
};

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const isSerializedSearchCreateError = (error) =>
  error?.response?.status === 429 &&
  /only one concurrent operation is permitted/i.test(
    error?.response?.data || error?.message || '',
  );

const createWithRetry = async (
  request,
  { maxAttempts = 4, retryDelayMs = 300 } = {},
) => {
  let attempt = 0;

  while (attempt < maxAttempts) {
    attempt += 1;

    try {
      return await create(request);
    } catch (error) {
      if (!isSerializedSearchCreateError(error) || attempt >= maxAttempts) {
        throw error;
      }

      await sleep(retryDelayMs * attempt);
    }
  }

  throw new Error('Search batch retry loop exhausted unexpectedly.');
};

export const createBatch = async ({ queries = [], providers = null } = {}) => {
  const normalizedQueries = Array.isArray(queries)
    ? queries
        .map((query) => (typeof query === 'string' ? query.trim() : ''))
        .filter(Boolean)
    : [];

  await normalizedQueries.reduce(
    (chain, searchText) =>
      chain.then(() =>
        createWithRetry({
          id: uuidv4(),
          providers,
          searchText,
        }),
      ),
    Promise.resolve(),
  );

  return normalizedQueries.length;
};

export const getStatus = async ({ id, includeResponses = false }) => {
  return (
    await api.get(
      `/searches/${encodeURIComponent(id)}?includeResponses=${includeResponses}`,
    )
  ).data;
};

export const getResponses = async ({ id }) => {
  const response = (
    await api.get(`/searches/${encodeURIComponent(id)}/responses`)
  ).data;

  if (!Array.isArray(response)) {
    console.warn('got non-array response from searches API', response);
    return [];
  }

  return response
    .filter(
      (entry) => entry && typeof entry === 'object' && !Array.isArray(entry),
    )
    .map((entry) => ({
      ...entry,
      files: Array.isArray(entry.files)
        ? entry.files.filter(
            (file) =>
              file &&
              typeof file === 'object' &&
              !Array.isArray(file) &&
              typeof file.filename === 'string',
          )
        : [],
      lockedFiles: Array.isArray(entry.lockedFiles)
        ? entry.lockedFiles.filter(
            (file) =>
              file &&
              typeof file === 'object' &&
              !Array.isArray(file) &&
              typeof file.filename === 'string',
          )
        : [],
    }));
};

const getNthMatch = (string, regex, n) => {
  const match = string.match(regex);

  if (match) {
    return Number.parseInt(match[n], 10);
  }

  return undefined;
};

// Parse size with unit (kb, mb, gb). Without unit, defaults to bytes.
const parseSize = (value, unit) => {
  const parsedNumber = Number.parseInt(value, 10);
  switch (unit?.toLowerCase()) {
    case 'gb':
      return parsedNumber * 1_024 * 1_024 * 1_024;
    case 'mb':
      return parsedNumber * 1_024 * 1_024;
    case 'kb':
      return parsedNumber * 1_024;
    case 'b':
    default:
      // Without unit, treat as bytes (most intuitive for raw numbers)
      return parsedNumber;
  }
};

const getSizeFromRegex = (string, regex) => {
  const match = string.match(regex);
  if (match) {
    const value = match[2];
    const unit = match[3];
    if (unit) {
      return parseSize(value, unit);
    }

    return Number.parseInt(value, 10);
  }

  return undefined;
};

export const parseFiltersFromString = (string) => {
  const query = typeof string === 'string' ? string : '';
  const filters = {
    exclude: [],
    extensions: [],
    include: [],
    isCBR: false,
    isLossless: false,
    isLossy: false,
    isVBR: false,
    maxFileSize: Number.MAX_SAFE_INTEGER,
    minBitDepth: 0,
    minBitRate: 0,
    minFilesInFolder: 0,
    minFileSize: 0,
    minLength: 0,
    minSampleRate: 0,
    preferExtensions: [],
    preferLossless: false,
    preferMinBitRate: 0,
  };

  filters.minBitRate =
    getNthMatch(query, /(minbr|minbitrate):(\d+)/iu, 2) || filters.minBitRate;
  filters.minBitDepth =
    getNthMatch(query, /(minbd|minbitdepth):(\d+)/iu, 2) ||
    filters.minBitDepth;
  filters.minSampleRate =
    getNthMatch(query, /(minsr|minsamplerate):(\d+)/iu, 2) ||
    filters.minSampleRate;

  filters.minFileSize =
    getSizeFromRegex(query, /(minfs|minfilesize):(\d+)(kb|mb|gb)?/iu) ||
    filters.minFileSize;

  filters.maxFileSize =
    getSizeFromRegex(query, /(maxfs|maxfilesize):(\d+)(kb|mb|gb)?/iu) ||
    filters.maxFileSize;

  filters.minLength =
    getNthMatch(query, /(minlen|minlength):(\d+)/iu, 2) || filters.minLength;
  filters.minFilesInFolder =
    getNthMatch(query, /(minfif|minfilesinfolder):(\d+)/iu, 2) ||
    filters.minFilesInFolder;

  filters.isVBR = Boolean(/isvbr/iu.test(query));
  filters.isCBR = Boolean(/iscbr/iu.test(query));
  filters.isLossless = Boolean(/islossless/iu.test(query));
  filters.isLossy = Boolean(/islossy/iu.test(query));
  filters.preferLossless = Boolean(/preferlossless/iu.test(query));
  filters.preferMinBitRate =
    getNthMatch(query, /(prefbr|preferbr|preferbitrate):(\d+)/iu, 2) ||
    filters.preferMinBitRate;

  // Parse extensions: ext:flac,mp3 or ext:flac mp3
  const extensionMatch = query.match(/ext:(\S+)/iu);
  if (extensionMatch) {
    filters.extensions = extensionMatch[1]
      .split(/[ ,]/)
      .map((e) => e.toLowerCase().trim())
      .filter((e) => e.length > 0);
  }

  const preferredExtensionMatch = query.match(/prefext:(\S+)/iu);
  if (preferredExtensionMatch) {
    filters.preferExtensions = preferredExtensionMatch[1]
      .split(/[ ,]/)
      .map((e) => e.toLowerCase().trim())
      .filter((e) => e.length > 0);
  }

  const terms = (query.toLowerCase().match(/-?"[^"]+"|\S+/gu) || [])
    .map((term) => {
      const excluded = term.startsWith('-');
      const value = (excluded ? term.slice(1) : term).replace(/^"|"$/gu, '');
      return excluded ? `-${value}` : value;
    })
    .filter(
      (term) =>
        !term.includes(':') &&
        term !== 'isvbr' &&
        term !== 'iscbr' &&
        term !== 'islossless' &&
        term !== 'islossy' &&
        term !== 'preferlossless' &&
        !term.startsWith('ext:') &&
        !term.startsWith('prefext:'),
    );

  filters.include = terms.filter((term) => !term.startsWith('-'));
  filters.exclude = terms
    .filter((term) => term.startsWith('-'))
    .map((term) => term.slice(1));

  return filters;
};

// eslint-disable-next-line complexity
const filterFile = (file, filters) => {
  const input =
    file && typeof file === 'object' && !Array.isArray(file) ? file : {};
  const activeFilters =
    filters && typeof filters === 'object' && !Array.isArray(filters)
      ? filters
      : {};
  const {
    bitRate,
    size,
    length,
    filename: rawFilename,
    sampleRate,
    bitDepth,
    isVariableBitRate,
  } = input;
  const {
    isCBR = false,
    isVBR = false,
    isLossless = false,
    isLossy = false,
  } = activeFilters;
  const rawInclude = Array.isArray(activeFilters.include)
    ? activeFilters.include
    : [];
  const rawExclude = Array.isArray(activeFilters.exclude)
    ? activeFilters.exclude
    : [];
  const rawExtensions = Array.isArray(activeFilters.extensions)
    ? activeFilters.extensions
    : [];
  const include = rawInclude
    .filter((term) => typeof term === 'string')
    .map((term) => term.toLowerCase());
  const exclude = rawExclude
    .filter((term) => typeof term === 'string')
    .map((term) => term.toLowerCase());
  const extensions = rawExtensions
    .filter((extension) => typeof extension === 'string')
    .map((extension) => extension.toLowerCase().replace(/^\./u, ''));
  const numeric = (value, fallback = 0) => {
    const parsed = typeof value === 'number' ? value : Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  };
  const normalizedBitRate = numeric(bitRate);
  const normalizedSize = numeric(size);
  const normalizedLength = numeric(length);
  const normalizedSampleRate = numeric(sampleRate);
  const normalizedBitDepth = numeric(bitDepth);
  const minBitRate = numeric(activeFilters.minBitRate);
  const minBitDepth = numeric(activeFilters.minBitDepth);
  const minSampleRate = numeric(activeFilters.minSampleRate);
  const maxFileSize = numeric(
    activeFilters.maxFileSize,
    Number.MAX_SAFE_INTEGER,
  );
  const minFileSize = numeric(activeFilters.minFileSize);
  const minLength = numeric(activeFilters.minLength);
  const filename = typeof rawFilename === 'string' ? rawFilename : '';

  if (isCBR && (typeof isVariableBitRate !== 'boolean' || isVariableBitRate))
    return false;
  if (isVBR && (typeof isVariableBitRate !== 'boolean' || !isVariableBitRate))
    return false;
  if (isLossless && (!normalizedSampleRate || !normalizedBitDepth))
    return false;
  if (isLossy && (normalizedSampleRate || normalizedBitDepth)) return false;
  if (normalizedBitRate < minBitRate) return false;
  if (normalizedBitDepth < minBitDepth) return false;
  if (
    minSampleRate &&
    normalizedSampleRate &&
    normalizedSampleRate < minSampleRate
  )
    return false;
  if (normalizedSize < minFileSize) return false;
  if (normalizedSize > maxFileSize) return false;
  if (normalizedLength < minLength) return false;

  // Filter by file extension
  if (extensions.length > 0) {
    const fileExtension = filename.split('.').pop()?.toLowerCase();
    if (!fileExtension || !extensions.includes(fileExtension)) return false;
  }

  if (
    include.length > 0 &&
    include.filter((term) => filename.toLowerCase().includes(term)).length !==
      include.length
  ) {
    return false;
  }

  if (exclude.some((term) => filename.toLowerCase().includes(term)))
    return false;

  return true;
};

export const filterResponse = ({
  filters = {
    exclude: [],
    extensions: [],
    include: [],
    isCBR: false,
    isLossless: false,
    isLossy: false,
    isVBR: false,
    maxFileSize: Number.MAX_SAFE_INTEGER,
    minBitDepth: 0,
    minBitRate: 0,
    minFilesInFolder: 0,
    minFileSize: 0,
    minLength: 0,
    minSampleRate: 0,
    preferExtensions: [],
    preferLossless: false,
    preferMinBitRate: 0,
  },
  response = {
    files: [],
    lockedFiles: [],
  },
}) => {
  const responseObject =
    response && typeof response === 'object' && !Array.isArray(response)
      ? response
      : {};
  const files = Array.isArray(responseObject.files) ? responseObject.files : [];
  const lockedFiles = Array.isArray(responseObject.lockedFiles)
    ? responseObject.lockedFiles
    : [];
  const activeFilters =
    filters && typeof filters === 'object' && !Array.isArray(filters)
      ? filters
      : {};
  const reportedFileCount = Number(responseObject.fileCount);
  const reportedLockedFileCount = Number(responseObject.lockedFileCount);
  const fileCount = Number.isFinite(reportedFileCount)
    ? Math.max(0, reportedFileCount, files.length)
    : files.length;
  const lockedFileCount = Number.isFinite(reportedLockedFileCount)
    ? Math.max(0, reportedLockedFileCount, lockedFiles.length)
    : lockedFiles.length;

  if (fileCount + lockedFileCount < Number(activeFilters.minFilesInFolder || 0)) {
    return {
      ...responseObject,
      fileCount: 0,
      files: [],
      lockedFileCount: 0,
      lockedFiles: [],
    };
  }

  const filterFiles = (filesToFilter) =>
    filesToFilter.filter(
      (file) =>
        file && typeof file === 'object' && !Array.isArray(file) &&
        filterFile(file, activeFilters),
    );

  const filteredFiles = filterFiles(files);
  const filteredLockedFiles = filterFiles(lockedFiles);

  return {
    ...responseObject,
    fileCount: filteredFiles.length,
    files: filteredFiles,
    lockedFileCount: filteredLockedFiles.length,
    lockedFiles: filteredLockedFiles,
  };
};

export const serializeFiltersToString = (filters) => {
  const parts = [];

  if (filters.include && filters.include.length > 0)
    parts.push(...filters.include);
  if (filters.exclude && filters.exclude.length > 0)
    parts.push(...filters.exclude.map((term) => `-${term}`));

  if (filters.minBitRate) parts.push(`minbr:${filters.minBitRate}`);
  if (filters.minBitDepth) parts.push(`minbd:${filters.minBitDepth}`);
  if (filters.minSampleRate) parts.push(`minsr:${filters.minSampleRate}`);
  if (filters.minFileSize) parts.push(`minfs:${filters.minFileSize}`);
  if (filters.maxFileSize && filters.maxFileSize < Number.MAX_SAFE_INTEGER)
    parts.push(`maxfs:${filters.maxFileSize}`);
  if (filters.minLength) parts.push(`minlen:${filters.minLength}`);
  if (filters.minFilesInFolder)
    parts.push(`minfif:${filters.minFilesInFolder}`);

  if (filters.isVBR) parts.push('isvbr');
  if (filters.isCBR) parts.push('iscbr');
  if (filters.isLossless) parts.push('islossless');
  if (filters.isLossy) parts.push('islossy');
  if (filters.preferLossless) parts.push('preferlossless');
  if (filters.preferMinBitRate) parts.push(`prefbr:${filters.preferMinBitRate}`);

  if (filters.extensions && filters.extensions.length > 0) {
    parts.push(`ext:${filters.extensions.join(',')}`);
  }

  if (filters.preferExtensions && filters.preferExtensions.length > 0) {
    parts.push(`prefext:${filters.preferExtensions.join(',')}`);
  }

  return parts.join(' ');
};
