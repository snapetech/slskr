import {
  getLocalStorageItem,
  removeLocalStorageItem,
  setLocalStorageItem,
} from '../../lib/storage';
import { toDisplayError } from '../../lib/errors';
import { readBoundedJson } from '../../lib/persistedJson';

const visualizerEngineStorageKey = 'slskr.player.visualizerEngine';
const rustyMilkPresetStorageKey = 'slskr.player.rustyMilkPreset';
const rustyMilkPresetLibraryStorageKey = 'slskr.player.rustyMilkPresetLibrary';
const rustyMilkPresetAutomationStorageKey = 'slskr.player.rustyMilkPresetAutomation';
const rustyMilkPresetFavoritesStorageKey = 'slskr.player.rustyMilkPresetFavorites';
const rustyMilkPresetFpsCapStorageKey = 'slskr.player.rustyMilkFpsCap';
const rustyMilkPresetQualityStorageKey = 'slskr.player.rustyMilkQuality';
const rustyMilkPresetLibraryModeStorageKey = 'slskr.player.rustyMilkPresetLibraryMode';
const rustyMilkPresetSearchStorageKey = 'slskr.player.rustyMilkPresetSearch';
const rustyMilkPresetPlaylistsStorageKey = 'slskr.player.rustyMilkPresetPlaylists';
const activeRustyMilkPresetPlaylistStorageKey = 'slskr.player.rustyMilkActivePresetPlaylist';
const rustyMilkPresetLibraryLimit = 20;
const rustyMilkPresetHistoryLimit = 12;
const rustyMilkPresetPlaylistLimit = 12;
const nativeTextureAssetMaxBytes = 1024 * 1024;
const nativeTextSourceMaxBytes = 1024 * 1024;
const nativeTextureAssetMaxCount = 32;
const nativeTextureAssetTotalBytes = 8 * 1024 * 1024;
const nativeTextureAssetMaxDataUrlBytes = 2 * 1024 * 1024;
const nativePresetFieldMaxBytes = 512;
const nativeTextureAssetFileNameMaxBytes = 1024;
const nativeStoredPresetJsonMaxCharacters = 16 * 1024 * 1024;
const nativeStoredPresetLibraryJsonMaxCharacters = 32 * 1024 * 1024;
const nativeStoredPlaylistJsonMaxCharacters = 512 * 1024;
const nativeStoredAutomationJsonMaxCharacters = 16 * 1024;
const nativeEditableParameters = [
  {
    defaultValue: 0.9,
    key: 'decay',
    label: 'Decay',
    max: 1,
    min: 0.5,
    step: 0.01,
  },
  {
    defaultValue: 1,
    key: 'zoom',
    label: 'Zoom',
    max: 1.5,
    min: 0.5,
    step: 0.01,
  },
  {
    defaultValue: 0,
    key: 'rot',
    label: 'Rotation',
    max: 0.5,
    min: -0.5,
    step: 0.01,
  },
  {
    defaultValue: 0.7,
    key: 'wave_r',
    label: 'Wave red',
    max: 1,
    min: 0,
    step: 0.01,
  },
  {
    defaultValue: 0.7,
    key: 'wave_g',
    label: 'Wave green',
    max: 1,
    min: 0,
    step: 0.01,
  },
  {
    defaultValue: 0.7,
    key: 'wave_b',
    label: 'Wave blue',
    max: 1,
    min: 0,
    step: 0.01,
  },
  {
    defaultValue: 1,
    key: 'wave_a',
    label: 'Wave alpha',
    max: 1,
    min: 0,
    step: 0.01,
  },
];

const readStoredEngine = () => {
  const stored = getLocalStorageItem(visualizerEngineStorageKey);
  if (stored === 'native') return 'rustymilk-webgl2';
  return ['rustymilk-webgl2', 'rustymilk-webgpu'].includes(stored)
    ? stored
    : 'rustymilk-webgl2';
};

const visualizerEngineModes = ['rustymilk-webgl2', 'rustymilk-webgpu'];

const isRustyMilkEngine = (engine) => engine === 'rustymilk-webgl2' || engine === 'rustymilk-webgpu';

const getRustyMilkRendererBackend = (engine) => (engine === 'rustymilk-webgpu' ? 'webgpu' : 'webgl2');

const getNextEngine = (engine) => {
  const index = visualizerEngineModes.indexOf(engine);
  return visualizerEngineModes[(index + 1) % visualizerEngineModes.length];
};

const getEngineLabel = (engine) => {
  if (engine === 'rustymilk-webgl2') return 'RustyMilk WebGL2';
  if (engine === 'rustymilk-webgpu') return 'RustyMilk WebGPU';
  return 'RustyMilk WebGL2';
};

const getEngineIcon = (engine) => {
  if (engine === 'rustymilk-webgpu') return 'bolt';
  return 'microchip';
};

const isPromiseLike = (value) => value && typeof value.then === 'function';

const getNextRustyMilkAutomationMode = (mode) => {
  if (mode === 'off') return 'beat';
  if (mode === 'beat') return 'timed';
  return 'off';
};

const getRustyMilkAutomationLabel = (mode) => {
  if (mode === 'beat') return 'Beat';
  if (mode === 'timed') return 'Timed';
  return 'Off';
};

const defaultRustyMilkAutomationSettings = {
  beatsPerPreset: 8,
  mode: 'off',
  timedIntervalSeconds: 30,
};

const normalizeRustyMilkAutomationSettings = (settings = {}) => ({
  ...defaultRustyMilkAutomationSettings,
  ...settings,
  beatsPerPreset: [4, 8, 16].includes(Number(settings.beatsPerPreset))
    ? Number(settings.beatsPerPreset)
    : defaultRustyMilkAutomationSettings.beatsPerPreset,
  mode: ['beat', 'timed'].includes(settings.mode) ? settings.mode : 'off',
  timedIntervalSeconds: [15, 30, 60].includes(Number(settings.timedIntervalSeconds))
    ? Number(settings.timedIntervalSeconds)
    : defaultRustyMilkAutomationSettings.timedIntervalSeconds,
});

const readStoredRustyMilkAutomationSettings = () => {
  const stored = getLocalStorageItem(rustyMilkPresetAutomationStorageKey);
  if (['beat', 'timed', 'off'].includes(stored)) {
    return normalizeRustyMilkAutomationSettings({ mode: stored });
  }
  return normalizeRustyMilkAutomationSettings(
    readBoundedJson(
      getLocalStorageItem,
      rustyMilkPresetAutomationStorageKey,
      {},
      nativeStoredAutomationJsonMaxCharacters,
    ),
  );
};

const writeStoredRustyMilkAutomationSettings = (settings) => {
  setLocalStorageItem(
    rustyMilkPresetAutomationStorageKey,
    JSON.stringify(normalizeRustyMilkAutomationSettings(settings)),
  );
};

const getRustyMilkEditableParameter = (key) =>
  nativeEditableParameters.find((parameter) => parameter.key === key)
  || nativeEditableParameters[0];

const readStoredRustyMilkFpsCap = () => {
  const value = getLocalStorageItem(rustyMilkPresetFpsCapStorageKey, 'full');
  return ['full', '60', '30', '24'].includes(value) ? value : 'full';
};

const getRustyMilkFpsCapMs = (fpsCap) => {
  if (fpsCap === '60') return 1000 / 60;
  if (fpsCap === '30') return 1000 / 30;
  if (fpsCap === '24') return 1000 / 24;
  return 0;
};

const nativeQualityPresets = {
  balanced: {
    fpsCap: '60',
    label: 'Balanced',
  },
  efficient: {
    fpsCap: '30',
    label: 'Efficient',
  },
  full: {
    fpsCap: 'full',
    label: 'Full',
  },
};

const readStoredRustyMilkQuality = () => {
  const value = getLocalStorageItem(rustyMilkPresetQualityStorageKey, 'balanced');
  return Object.keys(nativeQualityPresets).includes(value) || value === 'custom'
    ? value
    : 'balanced';
};

const getRustyMilkWebGpuDebugLabel = (status = {}) => {
  if (!status.available) {
    return status.reason ? `WebGL2 baseline (${status.reason})` : 'WebGL2 baseline';
  }
  const adapterLabel = [
    status.adapterInfo?.vendor,
    status.adapterInfo?.architecture,
    status.adapterInfo?.device,
  ].filter(Boolean).join(' ');
  return adapterLabel ? `WebGPU ${adapterLabel}` : 'WebGPU adapter ready';
};

const getVisualizerErrorMessage = (engineType, error) => {
  const detail = toDisplayError(error, '');
  return isRustyMilkEngine(engineType)
    ? `RustyMilk render failed.${detail ? ` ${detail}` : ''}`
    : 'RustyMilk failed. Showing analyzer fallback.';
};

const getUtf8ByteLength = (value) => {
  if (typeof TextEncoder === 'function') {
    return new TextEncoder().encode(value).byteLength;
  }
  return value.length;
};

const truncateUtf8 = (value, maximumBytes) => {
  if (typeof value !== 'string' || getUtf8ByteLength(value) <= maximumBytes) return value;
  let truncated = '';
  for (const character of value) {
    const next = truncated + character;
    if (getUtf8ByteLength(next) > maximumBytes) break;
    truncated = next;
  }
  return truncated;
};

const readStoredJson = (key, fallback, maximumCharacters) => {
  return readBoundedJson(getLocalStorageItem, key, fallback, maximumCharacters);
};

const normalizeStoredRustyMilkTextureAssets = (textureAssets) => {
  if (!textureAssets || typeof textureAssets !== 'object' || Array.isArray(textureAssets)) {
    return {};
  }
  const normalized = {};
  let totalBytes = 0;
  for (const [key, asset] of Object.entries(textureAssets)) {
    if (Object.keys(normalized).length >= nativeTextureAssetMaxCount) break;
    if (
      typeof key !== 'string'
      || key.length === 0
      || getUtf8ByteLength(key) > nativeTextureAssetFileNameMaxBytes
      || !asset
      || typeof asset !== 'object'
      || typeof asset.dataUrl !== 'string'
      || typeof asset.fileName !== 'string'
      || asset.fileName.length === 0
      || getUtf8ByteLength(asset.fileName) > nativeTextureAssetFileNameMaxBytes
      || getUtf8ByteLength(asset.dataUrl) > nativeTextureAssetMaxDataUrlBytes
    ) {
      continue;
    }
    const dataUrlBytes = getUtf8ByteLength(asset.dataUrl);
    if (totalBytes + dataUrlBytes > nativeTextureAssetTotalBytes) break;
    normalized[key] = {
      dataUrl: asset.dataUrl,
      fileName: asset.fileName,
    };
    totalBytes += dataUrlBytes;
  }
  return normalized;
};

const normalizeStoredRustyMilkPreset = (preset, { requireId = false } = {}) => {
  if (!preset || typeof preset !== 'object' || Array.isArray(preset)) return null;
  if (
    typeof preset.source !== 'string'
    || preset.source.length === 0
    || getUtf8ByteLength(preset.source) > nativeTextSourceMaxBytes
  ) {
    return null;
  }
  const id = typeof preset.id === 'string'
    && preset.id.length > 0
    && getUtf8ByteLength(preset.id) <= nativePresetFieldMaxBytes
    ? preset.id
    : '';
  if (requireId && !id) return null;
  const fileName = typeof preset.fileName === 'string'
    && preset.fileName.length > 0
    && getUtf8ByteLength(preset.fileName) <= nativePresetFieldMaxBytes
    ? preset.fileName
    : 'native.milk';
  const title = typeof preset.title === 'string'
    && preset.title.length > 0
    && getUtf8ByteLength(preset.title) <= nativePresetFieldMaxBytes
    ? preset.title
    : 'Native preset';
  const normalized = {
    fileName,
    id,
    source: preset.source,
    title,
  };
  if (Object.prototype.hasOwnProperty.call(preset, 'textureAssets')) {
    normalized.textureAssets = normalizeStoredRustyMilkTextureAssets(preset.textureAssets);
  }
  return normalized;
};

const readStoredRustyMilkPreset = () => {
  return normalizeStoredRustyMilkPreset(
    readStoredJson(
      rustyMilkPresetStorageKey,
      null,
      nativeStoredPresetJsonMaxCharacters,
    ),
  );
};

const readStoredRustyMilkPresetLibrary = () => {
  const library = readStoredJson(
    rustyMilkPresetLibraryStorageKey,
    [],
    nativeStoredPresetLibraryJsonMaxCharacters,
  );
  return Array.isArray(library)
    ? library
      .slice(0, rustyMilkPresetLibraryLimit)
      .map((preset) => normalizeStoredRustyMilkPreset(preset, { requireId: true }))
      .filter(Boolean)
    : [];
};

const readStoredRustyMilkPresetFavorites = () => {
  const favorites = readStoredJson(
    rustyMilkPresetFavoritesStorageKey,
    [],
    nativeStoredPlaylistJsonMaxCharacters,
  );
  return Array.isArray(favorites)
    ? favorites
      .slice(0, rustyMilkPresetLibraryLimit)
      .filter((id) => (
        typeof id === 'string'
        && id.length > 0
        && getUtf8ByteLength(id) <= nativePresetFieldMaxBytes
      ))
    : [];
};

const readStoredRustyMilkPresetLibraryMode = () => {
  return getLocalStorageItem(rustyMilkPresetLibraryModeStorageKey) === 'favorites'
    ? 'favorites'
    : 'all';
};

const readStoredRustyMilkPresetSearch = () => {
  const value = getLocalStorageItem(rustyMilkPresetSearchStorageKey, '');
  return typeof value === 'string' && getUtf8ByteLength(value) <= nativePresetFieldMaxBytes
    ? value
    : '';
};

const readStoredRustyMilkPresetPlaylists = () => {
  const playlists = readStoredJson(
    rustyMilkPresetPlaylistsStorageKey,
    [],
    nativeStoredPlaylistJsonMaxCharacters,
  );
  return Array.isArray(playlists)
    ? playlists
      .slice(0, rustyMilkPresetPlaylistLimit)
      .map((playlist) => {
        if (!playlist || typeof playlist !== 'object' || Array.isArray(playlist)) return null;
        const id = typeof playlist.id === 'string'
          && playlist.id.length > 0
          && getUtf8ByteLength(playlist.id) <= nativePresetFieldMaxBytes
          ? playlist.id
          : null;
        const name = typeof playlist.name === 'string'
          && playlist.name.length > 0
          && getUtf8ByteLength(playlist.name) <= nativePresetFieldMaxBytes
          ? playlist.name
          : null;
        const presetIds = Array.isArray(playlist.presetIds)
          ? playlist.presetIds
            .slice(0, rustyMilkPresetLibraryLimit)
            .filter((presetId) => (
              typeof presetId === 'string'
              && presetId.length > 0
              && getUtf8ByteLength(presetId) <= nativePresetFieldMaxBytes
            ))
          : [];
        if (!id || !name || presetIds.length === 0) return null;
        const createdAt = typeof playlist.createdAt === 'string'
          && getUtf8ByteLength(playlist.createdAt) <= nativePresetFieldMaxBytes
          ? playlist.createdAt
          : undefined;
        const updatedAt = typeof playlist.updatedAt === 'string'
          && getUtf8ByteLength(playlist.updatedAt) <= nativePresetFieldMaxBytes
          ? playlist.updatedAt
          : undefined;
        return {
          createdAt,
          id,
          name,
          presetIds,
          updatedAt,
        };
      })
      .filter(Boolean)
    : [];
};

const readStoredActiveRustyMilkPresetPlaylistId = () => {
  const value = getLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey, '');
  return typeof value === 'string' && getUtf8ByteLength(value) <= nativePresetFieldMaxBytes
    ? value
    : '';
};

const writeStoredRustyMilkPresetLibrary = (library) => {
  setLocalStorageItem(
    rustyMilkPresetLibraryStorageKey,
    JSON.stringify(library.slice(0, rustyMilkPresetLibraryLimit)),
  );
};

const writeStoredRustyMilkPresetFavorites = (favoriteIds) => {
  if (favoriteIds.length === 0) {
    removeLocalStorageItem(rustyMilkPresetFavoritesStorageKey);
    return;
  }
  setLocalStorageItem(
    rustyMilkPresetFavoritesStorageKey,
    JSON.stringify(favoriteIds),
  );
};

const writeStoredRustyMilkPresetPlaylists = (playlists) => {
  if (playlists.length === 0) {
    removeLocalStorageItem(rustyMilkPresetPlaylistsStorageKey);
    return;
  }
  setLocalStorageItem(
    rustyMilkPresetPlaylistsStorageKey,
    JSON.stringify(playlists.slice(0, rustyMilkPresetPlaylistLimit)),
  );
};

const upsertRustyMilkPresetLibraryEntry = (library, entry) => [
  entry,
  ...library.filter((preset) => preset.id !== entry.id),
].slice(0, rustyMilkPresetLibraryLimit);

const pruneRustyMilkPresetFavorites = (favoriteIds, library) => {
  const libraryIds = new Set(library.map((preset) => preset.id));
  return favoriteIds.filter((id) => libraryIds.has(id));
};

const pruneRustyMilkPresetPlaylists = (playlists, library) => {
  const libraryIds = new Set(library.map((preset) => preset.id));
  return playlists
    .map((playlist) => ({
      ...playlist,
      presetIds: playlist.presetIds.filter((id) => libraryIds.has(id)),
    }))
    .filter((playlist) => playlist.presetIds.length > 0)
    .slice(0, rustyMilkPresetPlaylistLimit);
};

const getRustyMilkPresetSearchText = (preset) =>
  [preset.title, preset.fileName].filter(Boolean).join(' ').toLowerCase();

const filterRustyMilkPresetLibrary = (library, search) => {
  const query = search.trim().toLowerCase();
  if (!query) return library;
  const terms = query.split(/\s+/).filter(Boolean);
  return library.filter((preset) => {
    const text = getRustyMilkPresetSearchText(preset);
    return terms.every((term) => text.includes(term));
  });
};

const getRustyMilkPresetPlaylistName = ({ mode, search }) => {
  const query = search.trim();
  if (query) return `Search: ${query}`;
  if (mode === 'favorites') return 'Favorites';
  return 'Native playlist';
};

const getRustyMilkPresetPlaylistId = () =>
  `playlist:${Date.now().toString(36)}:${Math.random().toString(36).slice(2, 8)}`;

const getRustyMilkPresetFileId = (file) =>
  [file.name, file.size, file.lastModified].filter((part) => part !== undefined).join(':');

const isRustyMilkPresetFile = (file) => /\.(milk2?|txt)$/i.test(file.name);

const isRustyMilkFragmentFile = (file) => /\.(shape|wave)$/i.test(file.name);

const getRustyMilkImportFilePath = (file) =>
  file.webkitRelativePath || file.name;

const isNativeTextureAssetCandidateFile = (file) =>
  /^image\//i.test(file.type) || /\.(png|jpe?g|webp|gif)$/i.test(file.name);

const getRustyMilkTextureAssetSkip = (file) => {
  if (isRustyMilkPresetFile(file) || isRustyMilkFragmentFile(file)) return null;
  if (!isNativeTextureAssetCandidateFile(file)) {
    return {
      fileName: file.name,
      message: 'Unsupported file type.',
    };
  }
  if (file.size > nativeTextureAssetMaxBytes) {
    return {
      fileName: file.name,
      message: 'Texture asset is larger than 1 MB.',
    };
  }
  return null;
};

const getTextureAssetKeys = (fileName) => {
  const normalized = fileName.trim().replace(/^['"]|['"]$/g, '').replace(/\\/g, '/').toLowerCase();
  const basename = normalized.replace(/^.*[\\/]/, '');
  const stem = basename.replace(/\.[^.]+$/, '');
  return Array.from(new Set([normalized, basename, stem].filter(Boolean)));
};

const textureReferencePattern =
  /(?:shape|sprite)\d+_(?:texture|tex|tex_name|image|img|file|filename)\s*=\s*([^\r\n;]+)/gi;
const standaloneTextureReferencePattern =
  /^\s*(?:texture|tex|tex_name|image|img|file|filename)\s*=\s*([^\r\n;]+)/gim;

const collectRustyMilkPresetTextureReferences = (source) => {
  const references = new Set();
  let match = textureReferencePattern.exec(source || '');
  while (match) {
    getTextureAssetKeys(match[1]).forEach((key) => references.add(key));
    match = textureReferencePattern.exec(source || '');
  }
  match = standaloneTextureReferencePattern.exec(source || '');
  while (match) {
    getTextureAssetKeys(match[1]).forEach((key) => references.add(key));
    match = standaloneTextureReferencePattern.exec(source || '');
  }
  return references;
};

const selectRustyMilkPresetTextureAssets = (source, textureAssets) => {
  const references = collectRustyMilkPresetTextureReferences(source);
  if (references.size === 0) return {};
  const selected = {};
  Object.entries(textureAssets).forEach(([key, asset]) => {
    if (!references.has(key)) return;
    getTextureAssetKeys(asset.fileName).forEach((alias) => {
      selected[alias] = asset;
    });
  });
  return selected;
};

const readFileAsDataUrl = (file) => new Promise((resolve, reject) => {
  if (typeof FileReader !== 'function') {
    reject(new Error('Texture asset imports require FileReader support.'));
    return;
  }
  const reader = new FileReader();
  reader.onerror = () => reject(reader.error || new Error(`Failed to read ${file.name}.`));
  reader.onload = () => resolve(reader.result);
  reader.readAsDataURL(file);
});

const readNativeTextureAssets = async (files) => {
  const textureAssets = {};
  const skippedTextureAssets = [];
  let acceptedTextureCount = 0;
  let acceptedTextureBytes = 0;
  for (const file of files.filter((entry) =>
    !isRustyMilkPresetFile(entry) && !isRustyMilkFragmentFile(entry))) {
    const skip = getRustyMilkTextureAssetSkip(file);
    if (skip) {
      skippedTextureAssets.push(skip);
      continue;
    }
    const declaredSize = Number(file.size);
    if (
      acceptedTextureCount >= nativeTextureAssetMaxCount
      || (
        Number.isFinite(declaredSize)
        && declaredSize >= 0
        && acceptedTextureBytes + declaredSize > nativeTextureAssetTotalBytes
      )
    ) {
      skippedTextureAssets.push({
        fileName: file.name,
        message: 'Texture asset import budget exceeded.',
      });
      continue;
    }
    let dataUrl = null;
    try {
      dataUrl = await readFileAsDataUrl(file);
    } catch (textureError) {
      skippedTextureAssets.push({
        fileName: file.name,
        message: toDisplayError(textureError, 'Texture asset could not be read.'),
      });
      continue;
    }
    if (
      typeof dataUrl !== 'string'
      || getUtf8ByteLength(dataUrl) > nativeTextureAssetMaxDataUrlBytes
    ) {
      skippedTextureAssets.push({
        fileName: file.name,
        message: 'Texture asset data exceeds the browser import limit.',
      });
      continue;
    }
    acceptedTextureCount += 1;
    if (Number.isFinite(declaredSize) && declaredSize >= 0) {
      acceptedTextureBytes += declaredSize;
    }
    const filePath = getRustyMilkImportFilePath(file);
    getTextureAssetKeys(filePath).forEach((key) => {
      textureAssets[key] = {
        dataUrl,
        fileName: filePath,
      };
    });
  }
  return { skippedTextureAssets, textureAssets };
};

const downloadTextFile = (fileName, source) => {
  const blob = new Blob([source], { type: 'text/plain' });
  const url = window.URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  link.remove();
  window.URL.revokeObjectURL(url);
};

const formatSkippedFileNames = (skipped) => {
  const skippedNames = skipped.slice(0, 3).map((entry) => entry.fileName).join(', ');
  const remaining = skipped.length > 3 ? `, +${skipped.length - 3} more` : '';
  return `${skippedNames}${remaining}`;
};

const getRustyMilkPresetImportMessage = ({ importedCount, skipped, skippedTextureAssets }) => {
  const messages = [];
  if (skipped.length > 0) {
    const prefix = importedCount > 0
      ? `Imported ${importedCount}; skipped ${skipped.length}`
      : `Native preset import failed for ${skipped.length}`;
    messages.push(`${prefix}: ${formatSkippedFileNames(skipped)}.`);
  }
  if (skippedTextureAssets.length > 0) {
    const noun = skippedTextureAssets.length === 1 ? 'texture asset' : 'texture assets';
    messages.push(
      `Skipped ${skippedTextureAssets.length} ${noun}: ${formatSkippedFileNames(skippedTextureAssets)}.`,
    );
  }
  return messages.length > 0 ? messages.join(' ') : null;
};

const supportsWebGl2 = () => {
  try {
    const canvas = document.createElement('canvas');
    return Boolean(canvas.getContext('webgl2'));
  } catch {
    return false;
  }
};


export {
  visualizerEngineStorageKey,
  rustyMilkPresetStorageKey,
  rustyMilkPresetLibraryStorageKey,
  rustyMilkPresetFavoritesStorageKey,
  rustyMilkPresetFpsCapStorageKey,
  rustyMilkPresetQualityStorageKey,
  rustyMilkPresetLibraryModeStorageKey,
  rustyMilkPresetSearchStorageKey,
  rustyMilkPresetPlaylistsStorageKey,
  activeRustyMilkPresetPlaylistStorageKey,
  rustyMilkPresetHistoryLimit,
  rustyMilkPresetPlaylistLimit,
  nativeTextSourceMaxBytes,
  nativePresetFieldMaxBytes,
  nativeEditableParameters,
  readStoredEngine,
  isRustyMilkEngine,
  getRustyMilkRendererBackend,
  getNextEngine,
  getEngineLabel,
  getEngineIcon,
  isPromiseLike,
  getNextRustyMilkAutomationMode,
  getRustyMilkAutomationLabel,
  normalizeRustyMilkAutomationSettings,
  readStoredRustyMilkAutomationSettings,
  writeStoredRustyMilkAutomationSettings,
  getRustyMilkEditableParameter,
  readStoredRustyMilkFpsCap,
  getRustyMilkFpsCapMs,
  nativeQualityPresets,
  readStoredRustyMilkQuality,
  getRustyMilkWebGpuDebugLabel,
  getVisualizerErrorMessage,
  truncateUtf8,
  readStoredRustyMilkPreset,
  readStoredRustyMilkPresetLibrary,
  readStoredRustyMilkPresetFavorites,
  readStoredRustyMilkPresetLibraryMode,
  readStoredRustyMilkPresetSearch,
  readStoredRustyMilkPresetPlaylists,
  readStoredActiveRustyMilkPresetPlaylistId,
  writeStoredRustyMilkPresetLibrary,
  writeStoredRustyMilkPresetFavorites,
  writeStoredRustyMilkPresetPlaylists,
  upsertRustyMilkPresetLibraryEntry,
  pruneRustyMilkPresetFavorites,
  pruneRustyMilkPresetPlaylists,
  filterRustyMilkPresetLibrary,
  getRustyMilkPresetPlaylistName,
  getRustyMilkPresetPlaylistId,
  getRustyMilkPresetFileId,
  isRustyMilkPresetFile,
  isRustyMilkFragmentFile,
  selectRustyMilkPresetTextureAssets,
  readNativeTextureAssets,
  downloadTextFile,
  getRustyMilkPresetImportMessage,
  supportsWebGl2,
};
