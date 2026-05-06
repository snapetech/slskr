import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Button, Icon, Popup } from 'semantic-ui-react';
import {
  getLocalStorageItem,
  removeLocalStorageItem,
  setLocalStorageItem,
} from '../../lib/storage';
import { resumeAudioGraph } from './audioGraph';
import SpectrumAnalyzer from './SpectrumAnalyzer';
import { createRustyMilkEngine } from './visualizers/rustyMilkEngine';

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
  try {
    return normalizeRustyMilkAutomationSettings(JSON.parse(stored || '{}'));
  } catch {
    return defaultRustyMilkAutomationSettings;
  }
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
  const detail = error?.message ? ` ${error.message}` : '';
  return isRustyMilkEngine(engineType)
    ? `RustyMilk render failed.${detail}`
    : 'RustyMilk failed. Showing analyzer fallback.';
};

const readStoredRustyMilkPreset = () => {
  try {
    return JSON.parse(getLocalStorageItem(rustyMilkPresetStorageKey, 'null'));
  } catch {
    return null;
  }
};

const readStoredRustyMilkPresetLibrary = () => {
  try {
    const library = JSON.parse(
      getLocalStorageItem(rustyMilkPresetLibraryStorageKey, '[]'),
    );
    return Array.isArray(library)
      ? library.filter((preset) => preset?.id && preset?.source)
      : [];
  } catch {
    return [];
  }
};

const readStoredRustyMilkPresetFavorites = () => {
  try {
    const favorites = JSON.parse(
      getLocalStorageItem(rustyMilkPresetFavoritesStorageKey, '[]'),
    );
    return Array.isArray(favorites)
      ? favorites.filter((id) => typeof id === 'string' && id.length > 0)
      : [];
  } catch {
    return [];
  }
};

const readStoredRustyMilkPresetLibraryMode = () => {
  return getLocalStorageItem(rustyMilkPresetLibraryModeStorageKey) === 'favorites'
    ? 'favorites'
    : 'all';
};

const readStoredRustyMilkPresetSearch = () => {
  return getLocalStorageItem(rustyMilkPresetSearchStorageKey, '');
};

const readStoredRustyMilkPresetPlaylists = () => {
  try {
    const playlists = JSON.parse(
      getLocalStorageItem(rustyMilkPresetPlaylistsStorageKey, '[]'),
    );
    return Array.isArray(playlists)
      ? playlists
        .filter((playlist) =>
          playlist?.id
          && playlist?.name
          && Array.isArray(playlist?.presetIds))
        .map((playlist) => ({
          ...playlist,
          presetIds: playlist.presetIds.filter((id) => typeof id === 'string' && id.length > 0),
        }))
        .filter((playlist) => playlist.presetIds.length > 0)
      : [];
  } catch {
    return [];
  }
};

const readStoredActiveRustyMilkPresetPlaylistId = () => {
  return getLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey, '');
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
  for (const file of files.filter((entry) =>
    !isRustyMilkPresetFile(entry) && !isRustyMilkFragmentFile(entry))) {
    const skip = getRustyMilkTextureAssetSkip(file);
    if (skip) {
      skippedTextureAssets.push(skip);
      continue;
    }
    let dataUrl = null;
    try {
      dataUrl = await readFileAsDataUrl(file);
    } catch (textureError) {
      skippedTextureAssets.push({
        fileName: file.name,
        message: textureError?.message || 'Texture asset could not be read.',
      });
      continue;
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

const Visualizer = ({
  audioElement,
  compactControls = false,
  engineOverride,
  mode,
  onEngineChange,
  onModeChange,
}) => {
  const containerRef = useRef(null);
  const canvasRef = useRef(null);
  const directoryInputRef = useRef(null);
  const engineRef = useRef(null);
  const fileInputRef = useRef(null);
  const lastNativeMouseRef = useRef({ x: 0.5, y: 0.5 });
  const lastNativeRenderAtRef = useRef(0);
  const rafRef = useRef(null);
  const engineAudioNodeRef = useRef(null);
  const rustyMilkAutomationSettingsRef = useRef(readStoredRustyMilkAutomationSettings());
  const [fallbackMode, setFallbackMode] = useState(false);
  const [engineType, setEngineType] = useState(readStoredEngine);
  const [engineName, setEngineName] = useState('');
  const [rustyMilkAutomationSettings, setRustyMilkAutomationSettings] = useState(
    () => rustyMilkAutomationSettingsRef.current,
  );
  const [activeRustyMilkPresetId, setActiveRustyMilkPresetId] = useState(
    () => readStoredRustyMilkPreset()?.id || '',
  );
  const [rustyMilkFavoritePresetIds, setRustyMilkFavoritePresetIds] = useState(
    readStoredRustyMilkPresetFavorites,
  );
  const [rustyMilkLibraryMode, setRustyMilkLibraryMode] = useState(
    readStoredRustyMilkPresetLibraryMode,
  );
  const [rustyMilkPresetHistory, setRustyMilkPresetHistory] = useState([]);
  const [rustyMilkPresetLibrary, setRustyMilkPresetLibrary] = useState(readStoredRustyMilkPresetLibrary);
  const [nativeFpsCap, setNativeFpsCap] = useState(readStoredRustyMilkFpsCap);
  const [rustyMilkFrameMs, setRustyMilkFrameMs] = useState(0);
  const [nativeQualityPreset, setNativeQualityPreset] = useState(readStoredRustyMilkQuality);
  const [rustyMilkPresetSearch, setRustyMilkPresetSearch] = useState(readStoredRustyMilkPresetSearch);
  const [rustyMilkPresetPlaylists, setRustyMilkPresetPlaylists] = useState(
    readStoredRustyMilkPresetPlaylists,
  );
  const [rustyMilkFragmentSummary, setRustyMilkFragmentSummary] = useState({
    shapes: [],
    waves: [],
  });
  const [rustyMilkParameterValues, setRustyMilkParameterValues] = useState({});
  const [rustyMilkParameterDrafts, setRustyMilkParameterDrafts] = useState({});
  const [selectedRustyMilkParameter, setSelectedRustyMilkParameter] = useState(
    nativeEditableParameters[0].key,
  );
  const [showNativeDebug, setShowNativeDebug] = useState(false);
  const [nativeDebugSnapshot, setNativeDebugSnapshot] = useState(null);
  const [selectedNativeShapeIndex, setSelectedNativeShapeIndex] = useState(0);
  const [selectedNativeWaveIndex, setSelectedNativeWaveIndex] = useState(0);
  const [activeNativePlaylistId, setActiveNativePlaylistId] = useState(
    readStoredActiveRustyMilkPresetPlaylistId,
  );
  const [presetName, setPresetName] = useState('');
  const [error, setError] = useState(null);
  const activeEngineType = engineOverride || engineType;

  const activeNativePlaylist = rustyMilkPresetPlaylists.find(
    (playlist) => playlist.id === activeNativePlaylistId,
  );
  const playlistScopedRustyMilkPresetLibrary = activeNativePlaylist
    ? activeNativePlaylist.presetIds
      .map((presetId) => rustyMilkPresetLibrary.find((preset) => preset.id === presetId))
      .filter(Boolean)
    : rustyMilkPresetLibrary;
  const modeFilteredRustyMilkPresetLibrary = rustyMilkLibraryMode === 'favorites'
    ? playlistScopedRustyMilkPresetLibrary.filter(
      (preset) => rustyMilkFavoritePresetIds.includes(preset.id),
    )
    : playlistScopedRustyMilkPresetLibrary;
  const visibleRustyMilkPresetLibrary = filterRustyMilkPresetLibrary(
    modeFilteredRustyMilkPresetLibrary,
    rustyMilkPresetSearch,
  );
  const visibleRustyMilkPresetIndex = visibleRustyMilkPresetLibrary.findIndex(
    (preset) => preset.id === activeRustyMilkPresetId,
  );
  const activeRustyMilkPresetIsFavorite = rustyMilkFavoritePresetIds.includes(activeRustyMilkPresetId);
  const selectedRustyMilkPresetValue = visibleRustyMilkPresetLibrary.some(
    (preset) => preset.id === activeRustyMilkPresetId,
  )
    ? activeRustyMilkPresetId
    : '';
  const hasRustyMilkPresetSearch = rustyMilkPresetSearch.trim().length > 0;
  const nativeBankNavigationDisabled = isRustyMilkEngine(activeEngineType)
    && rustyMilkPresetLibrary.length > 0
    && visibleRustyMilkPresetLibrary.length === 0;
  const canSaveNativePlaylist = visibleRustyMilkPresetLibrary.length > 0;
  const hasNativeShapes = rustyMilkFragmentSummary.shapes.length > 0;
  const hasNativeWaves = rustyMilkFragmentSummary.waves.length > 0;
  const rustyMilkAutomationMode = rustyMilkAutomationSettings.mode;
  const rustyMilkParameter = getRustyMilkEditableParameter(selectedRustyMilkParameter);
  const rustyMilkParameterValue = Number(
    rustyMilkParameterDrafts[selectedRustyMilkParameter]
    ?? rustyMilkParameterValues[selectedRustyMilkParameter]
    ?? rustyMilkParameter.defaultValue,
  );

  const refreshRustyMilkFragmentSummary = useCallback(() => {
    const summary = engineRef.current?.getPresetFragmentSummary?.() || {
      shapes: [],
      waves: [],
    };
    setRustyMilkFragmentSummary(summary);
    setSelectedNativeShapeIndex((index) =>
      Math.min(index, Math.max(0, summary.shapes.length - 1)));
    setSelectedNativeWaveIndex((index) =>
      Math.min(index, Math.max(0, summary.waves.length - 1)));
    setRustyMilkParameterValues(engineRef.current?.getPresetParameterSummary?.() || {});
    setRustyMilkParameterDrafts({});
    setNativeDebugSnapshot(engineRef.current?.getPresetDebugSnapshot?.() || null);
  }, []);

  const renderLoop = useCallback((timestamp = performance.now()) => {
    if (!engineRef.current) return;
    try {
      const fpsCapMs = isRustyMilkEngine(activeEngineType) ? getRustyMilkFpsCapMs(nativeFpsCap) : 0;
      if (
        fpsCapMs > 0
        && lastNativeRenderAtRef.current
        && timestamp - lastNativeRenderAtRef.current < fpsCapMs
      ) {
        rafRef.current = window.requestAnimationFrame(renderLoop);
        return;
      }
      const startedAt = performance.now();
      const renderResult = engineRef.current.render();
      if (isRustyMilkEngine(activeEngineType)) {
        lastNativeRenderAtRef.current = timestamp;
        if (showNativeDebug) {
          setRustyMilkFrameMs(Number((performance.now() - startedAt).toFixed(1)));
        }
      }
      if (renderResult?.presetName) {
        setPresetName(renderResult.presetName);
      }
    } catch (renderError) {
      // eslint-disable-next-line no-console
      console.error('Failed to render RustyMilk visualizer', renderError);
      if (isRustyMilkEngine(activeEngineType)) {
        removeLocalStorageItem(rustyMilkPresetStorageKey);
      }
      setError(getVisualizerErrorMessage(activeEngineType, renderError));
      return;
    }
    rafRef.current = window.requestAnimationFrame(renderLoop);
  }, [activeEngineType, nativeFpsCap, showNativeDebug]);

  const cycleRustyMilkAutomationMode = useCallback(() => {
    setRustyMilkAutomationSettings((current) =>
      normalizeRustyMilkAutomationSettings({
        ...current,
        mode: getNextRustyMilkAutomationMode(current.mode),
      }));
  }, []);

  const updateRustyMilkAutomationBeats = useCallback((event) => {
    setRustyMilkAutomationSettings((current) =>
      normalizeRustyMilkAutomationSettings({
        ...current,
        beatsPerPreset: Number(event.target.value),
      }));
  }, []);

  const updateRustyMilkAutomationInterval = useCallback((event) => {
    setRustyMilkAutomationSettings((current) =>
      normalizeRustyMilkAutomationSettings({
        ...current,
        timedIntervalSeconds: Number(event.target.value),
      }));
  }, []);

  const updateNativeFpsCap = useCallback((event) => {
    const fpsCap = event.target.value;
    setNativeFpsCap(fpsCap);
    setNativeQualityPreset('custom');
    setLocalStorageItem(rustyMilkPresetQualityStorageKey, 'custom');
    if (fpsCap === 'full') {
      removeLocalStorageItem(rustyMilkPresetFpsCapStorageKey);
    } else {
      setLocalStorageItem(rustyMilkPresetFpsCapStorageKey, fpsCap);
    }
    lastNativeRenderAtRef.current = 0;
  }, []);

  const updateNativeQualityPreset = useCallback((event) => {
    const quality = event.target.value;
    const preset = nativeQualityPresets[quality];
    if (!preset) return;
    setNativeQualityPreset(quality);
    setLocalStorageItem(rustyMilkPresetQualityStorageKey, quality);
    setNativeFpsCap(preset.fpsCap);
    if (preset.fpsCap === 'full') {
      removeLocalStorageItem(rustyMilkPresetFpsCapStorageKey);
    } else {
      setLocalStorageItem(rustyMilkPresetFpsCapStorageKey, preset.fpsCap);
    }
    lastNativeRenderAtRef.current = 0;
  }, []);

  const selectRustyMilkParameter = useCallback((event) => {
    setSelectedRustyMilkParameter(event.target.value);
  }, []);

  const updateRustyMilkParameterDraft = useCallback((event) => {
    const value = Number(event.target.value);
    setRustyMilkParameterDrafts((drafts) => ({
      ...drafts,
      [selectedRustyMilkParameter]: value,
    }));
  }, [selectedRustyMilkParameter]);

  const sizeCanvas = useCallback(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    const engine = engineRef.current;
    if (!container || !canvas || !engine) return;
    const rect = container.getBoundingClientRect();
    const width = Math.max(1, Math.floor(rect.width));
    const height = Math.max(1, Math.floor(rect.height));
    canvas.width = width;
    canvas.height = height;
    engine.resize(width, height);
  }, []);

  const loadRustyMilkPresetEntry = useCallback(async (preset, options = {}) => {
    if (!preset || !engineRef.current?.loadPresetText) return false;
    const { pushHistory = true } = options;

    try {
      setError(null);
      let loadedPresetName = engineRef.current.loadPresetText(
        preset.source,
        preset.fileName,
        { textureAssets: preset.textureAssets },
      );
      if (isPromiseLike(loadedPresetName)) {
        loadedPresetName = await loadedPresetName;
      }
      setLocalStorageItem(rustyMilkPresetStorageKey, JSON.stringify(preset));
      if (pushHistory && activeRustyMilkPresetId && activeRustyMilkPresetId !== preset.id) {
        setRustyMilkPresetHistory((history) => [
          activeRustyMilkPresetId,
          ...history.filter((id) => id !== activeRustyMilkPresetId && id !== preset.id),
        ].slice(0, rustyMilkPresetHistoryLimit));
      }
      setActiveRustyMilkPresetId(preset.id);
      setPresetName(loadedPresetName);
      refreshRustyMilkFragmentSummary();
      sizeCanvas();
      return true;
    } catch (presetError) {
      // eslint-disable-next-line no-console
      console.error('Failed to load RustyMilk preset from library', presetError);
      setError(presetError?.message || 'Native preset load failed.');
      return false;
    }
  }, [activeRustyMilkPresetId, refreshRustyMilkFragmentSummary, sizeCanvas]);

  const loadRustyMilkPresetByOffset = useCallback((offset) => {
    if (visibleRustyMilkPresetLibrary.length === 0) return false;
    const currentIndex = visibleRustyMilkPresetIndex >= 0
      ? visibleRustyMilkPresetIndex
      : (offset > 0 ? -1 : 0);
    const nextIndex = (
      currentIndex + offset + visibleRustyMilkPresetLibrary.length
    ) % visibleRustyMilkPresetLibrary.length;
    return loadRustyMilkPresetEntry(visibleRustyMilkPresetLibrary[nextIndex]);
  }, [loadRustyMilkPresetEntry, visibleRustyMilkPresetIndex, visibleRustyMilkPresetLibrary]);

  const cyclePreset = useCallback(async () => {
    if (isRustyMilkEngine(activeEngineType) && rustyMilkPresetLibrary.length > 0) {
      await loadRustyMilkPresetByOffset(1);
      return;
    }
    if (!engineRef.current) return;
    let nextPresetName = engineRef.current.nextPreset();
    if (isPromiseLike(nextPresetName)) {
      nextPresetName = await nextPresetName;
    }
    if (nextPresetName) {
      setPresetName(nextPresetName);
    }
  }, [activeEngineType, loadRustyMilkPresetByOffset, rustyMilkPresetLibrary.length]);

  const cycleEngineType = useCallback(() => {
    const nextEngine = getNextEngine(activeEngineType);
    if (onEngineChange) {
      onEngineChange(nextEngine);
      return;
    }
    setEngineType(nextEngine);
  }, [activeEngineType, onEngineChange]);

  const previousRustyMilkLibraryPreset = useCallback(async () => {
    if (rustyMilkPresetHistory.length > 0) {
      const [previousId, ...remainingHistory] = rustyMilkPresetHistory;
      const previousPreset = rustyMilkPresetLibrary.find((preset) => preset.id === previousId);
      setRustyMilkPresetHistory(remainingHistory);
      await loadRustyMilkPresetEntry(previousPreset, { pushHistory: false });
      return;
    }
    await loadRustyMilkPresetByOffset(-1);
  }, [
    loadRustyMilkPresetByOffset,
    loadRustyMilkPresetEntry,
    rustyMilkPresetHistory,
    rustyMilkPresetLibrary,
  ]);

  const randomRustyMilkLibraryPreset = useCallback(async () => {
    if (visibleRustyMilkPresetLibrary.length === 0) return;
    const candidates = visibleRustyMilkPresetLibrary.filter(
      (preset) => preset.id !== activeRustyMilkPresetId,
    );
    const pool = candidates.length > 0 ? candidates : visibleRustyMilkPresetLibrary;
    const randomIndex = Math.floor(Math.random() * pool.length);
    await loadRustyMilkPresetEntry(pool[randomIndex]);
  }, [activeRustyMilkPresetId, loadRustyMilkPresetEntry, visibleRustyMilkPresetLibrary]);

  const toggleRustyMilkPresetFavorite = useCallback(() => {
    if (!activeRustyMilkPresetId) return;
    setRustyMilkFavoritePresetIds((favoriteIds) => {
      const nextFavoriteIds = favoriteIds.includes(activeRustyMilkPresetId)
        ? favoriteIds.filter((id) => id !== activeRustyMilkPresetId)
        : [activeRustyMilkPresetId, ...favoriteIds];
      writeStoredRustyMilkPresetFavorites(nextFavoriteIds);
      return nextFavoriteIds;
    });
  }, [activeRustyMilkPresetId]);

  const toggleRustyMilkLibraryMode = useCallback(() => {
    setRustyMilkLibraryMode((current) => {
      const nextMode = current === 'favorites' ? 'all' : 'favorites';
      setLocalStorageItem(rustyMilkPresetLibraryModeStorageKey, nextMode);
      return nextMode;
    });
  }, []);

  const updateRustyMilkPresetSearch = useCallback((event) => {
    const nextSearch = event.target.value;
    setRustyMilkPresetSearch(nextSearch);
    if (nextSearch.trim()) {
      setLocalStorageItem(rustyMilkPresetSearchStorageKey, nextSearch);
    } else {
      removeLocalStorageItem(rustyMilkPresetSearchStorageKey);
    }
  }, []);

  const clearRustyMilkPresetSearch = useCallback(() => {
    setRustyMilkPresetSearch('');
    removeLocalStorageItem(rustyMilkPresetSearchStorageKey);
  }, []);

  const saveNativePlaylistFromVisibleBank = useCallback(() => {
    if (visibleRustyMilkPresetLibrary.length === 0) return;
    const defaultName = getRustyMilkPresetPlaylistName({
      mode: rustyMilkLibraryMode,
      search: rustyMilkPresetSearch,
    });
    const nextName = window.prompt?.('Name this RustyMilk playlist', defaultName);
    if (!nextName || !nextName.trim()) return;
    const playlist = {
      createdAt: new Date().toISOString(),
      id: getRustyMilkPresetPlaylistId(),
      name: nextName.trim(),
      presetIds: visibleRustyMilkPresetLibrary.map((preset) => preset.id),
    };
    setRustyMilkPresetPlaylists((playlists) => {
      const nextPlaylists = [
        playlist,
        ...playlists.filter((entry) => entry.name !== playlist.name),
      ].slice(0, rustyMilkPresetPlaylistLimit);
      writeStoredRustyMilkPresetPlaylists(nextPlaylists);
      return nextPlaylists;
    });
    setActiveNativePlaylistId(playlist.id);
    setLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey, playlist.id);
  }, [rustyMilkLibraryMode, rustyMilkPresetSearch, visibleRustyMilkPresetLibrary]);

  const selectNativePlaylist = useCallback((event) => {
    const playlistId = event.target.value;
    setActiveNativePlaylistId(playlistId);
    if (playlistId) {
      setLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey, playlistId);
    } else {
      removeLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey);
    }
  }, []);

  const clearActiveNativePlaylist = useCallback(() => {
    setActiveNativePlaylistId('');
    removeLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey);
  }, []);

  const renameActiveNativePlaylist = useCallback(() => {
    const activePlaylist = rustyMilkPresetPlaylists.find(
      (playlist) => playlist.id === activeNativePlaylistId,
    );
    if (!activePlaylist) return;
    const nextName = window.prompt?.('Rename RustyMilk playlist', activePlaylist.name);
    if (!nextName || !nextName.trim()) return;
    setRustyMilkPresetPlaylists((playlists) => {
      const nextPlaylists = playlists.map((playlist) =>
        (playlist.id === activePlaylist.id
          ? { ...playlist, name: nextName.trim(), updatedAt: new Date().toISOString() }
          : playlist));
      writeStoredRustyMilkPresetPlaylists(nextPlaylists);
      return nextPlaylists;
    });
  }, [activeNativePlaylistId, rustyMilkPresetPlaylists]);

  const removeActiveNativePlaylist = useCallback(() => {
    if (!activeNativePlaylistId) return;
    setRustyMilkPresetPlaylists((playlists) => {
      const nextPlaylists = playlists.filter((playlist) => playlist.id !== activeNativePlaylistId);
      writeStoredRustyMilkPresetPlaylists(nextPlaylists);
      return nextPlaylists;
    });
    setActiveNativePlaylistId('');
    removeLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey);
  }, [activeNativePlaylistId]);

  useEffect(() => {
    if (mode === 'off' || !audioElement || !canvasRef.current) return undefined;

    let cancelled = false;
    let resizeObserver = null;
    let createdEngine = null;

    (async () => {
      try {
        setError(null);
        setFallbackMode(false);
        const graph = await resumeAudioGraph(audioElement);
        if (!graph) {
          setError('Web Audio is not available in this browser.');
          setFallbackMode(true);
          return;
        }

        if (activeEngineType === 'rustymilk-webgl2' && !supportsWebGl2()) {
          setError('RustyMilk WebGL2 needs WebGL2. Showing analyzer fallback.');
          setFallbackMode(true);
          return;
        }

        const engine = await createRustyMilkEngine({
          audioContext: graph.ctx,
          audioNode: graph.visualizerInput,
          canvas: canvasRef.current,
          pixelRatio: window.devicePixelRatio || 1,
          rendererBackend: getRustyMilkRendererBackend(activeEngineType),
        });
        createdEngine = engine;
        if (cancelled) {
          engine.dispose();
          return;
        }

        engineRef.current = engine;
        engineAudioNodeRef.current = graph.visualizerInput;
        setEngineName(engine.name);
        setPresetName(engine.presetName);
        if (isRustyMilkEngine(activeEngineType) && engine.setPresetAutomation) {
          engine.setPresetAutomation(rustyMilkAutomationSettingsRef.current);
        }
        if (isRustyMilkEngine(activeEngineType)) {
          refreshRustyMilkFragmentSummary();
        }
        const storedRustyMilkPreset = isRustyMilkEngine(activeEngineType) ? readStoredRustyMilkPreset() : null;
        if (storedRustyMilkPreset?.source && engine.loadPresetText) {
          let importedPresetName = engine.loadPresetText(
            storedRustyMilkPreset.source,
            storedRustyMilkPreset.fileName,
            { textureAssets: storedRustyMilkPreset.textureAssets },
          );
          if (isPromiseLike(importedPresetName)) {
            importedPresetName = await importedPresetName;
          }
          setActiveRustyMilkPresetId(storedRustyMilkPreset.id || '');
          setPresetName(importedPresetName);
          refreshRustyMilkFragmentSummary();
        }
        sizeCanvas();

        if (typeof window.ResizeObserver === 'function' && containerRef.current) {
          resizeObserver = new window.ResizeObserver(() => sizeCanvas());
          resizeObserver.observe(containerRef.current);
        }

        rafRef.current = window.requestAnimationFrame(renderLoop);
      } catch (importError) {
        if (createdEngine) {
          try {
            createdEngine.dispose();
          } catch {
            // The renderer may have failed while the browser was tearing down its context.
          }
        }
        if (engineRef.current === createdEngine) {
          engineRef.current = null;
          engineAudioNodeRef.current = null;
        }
        // eslint-disable-next-line no-console
        console.error('Failed to load RustyMilk visualizer', importError);
        setError(getVisualizerErrorMessage(activeEngineType, importError));
        setFallbackMode(true);
      }
    })();

    return () => {
      cancelled = true;
      if (rafRef.current) {
        window.cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
      if (engineRef.current && engineAudioNodeRef.current) {
        try {
          engineRef.current.dispose();
        } catch {
          // The engine may already have disconnected during canvas teardown.
        }
      }
      engineRef.current = null;
      engineAudioNodeRef.current = null;
      setEngineName('');
    };
  }, [mode, audioElement, activeEngineType, refreshRustyMilkFragmentSummary, renderLoop, sizeCanvas]);

  useEffect(() => {
    if (engineOverride) return;
    setLocalStorageItem(visualizerEngineStorageKey, engineType);
  }, [engineOverride, engineType]);

  useEffect(() => {
    rustyMilkAutomationSettingsRef.current = rustyMilkAutomationSettings;
    writeStoredRustyMilkAutomationSettings(rustyMilkAutomationSettings);
    if (isRustyMilkEngine(activeEngineType) && engineRef.current?.setPresetAutomation) {
      engineRef.current.setPresetAutomation(rustyMilkAutomationSettings);
    }
  }, [activeEngineType, rustyMilkAutomationSettings]);

  useEffect(() => {
    setRustyMilkFavoritePresetIds((favoriteIds) => {
      const nextFavoriteIds = pruneRustyMilkPresetFavorites(favoriteIds, rustyMilkPresetLibrary);
      if (nextFavoriteIds.length !== favoriteIds.length) {
        writeStoredRustyMilkPresetFavorites(nextFavoriteIds);
        if (nextFavoriteIds.length === 0 && rustyMilkLibraryMode === 'favorites') {
          setRustyMilkLibraryMode('all');
          setLocalStorageItem(rustyMilkPresetLibraryModeStorageKey, 'all');
        }
        return nextFavoriteIds;
      }
      return favoriteIds;
    });
    setRustyMilkPresetHistory((history) => {
      const libraryIds = new Set(rustyMilkPresetLibrary.map((preset) => preset.id));
      return history.filter((id) => libraryIds.has(id));
    });
    setRustyMilkPresetPlaylists((playlists) => {
      const nextPlaylists = pruneRustyMilkPresetPlaylists(playlists, rustyMilkPresetLibrary);
      if (
        nextPlaylists.length !== playlists.length
        || nextPlaylists.some((playlist, index) =>
          playlist.presetIds.length !== playlists[index].presetIds.length)
      ) {
        writeStoredRustyMilkPresetPlaylists(nextPlaylists);
        if (
          activeNativePlaylistId
          && !nextPlaylists.some((playlist) => playlist.id === activeNativePlaylistId)
        ) {
          setActiveNativePlaylistId('');
          removeLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey);
        }
        return nextPlaylists;
      }
      return playlists;
    });
  }, [activeNativePlaylistId, rustyMilkLibraryMode, rustyMilkPresetLibrary]);

  useEffect(() => {
    sizeCanvas();
  }, [mode, sizeCanvas]);

  useEffect(() => {
    const handleFullscreenChange = () => {
      const fsElement = document.fullscreenElement;
      if (mode === 'fullscreen' && !fsElement) {
        onModeChange('inline');
      }
    };
    document.addEventListener('fullscreenchange', handleFullscreenChange);
    return () =>
      document.removeEventListener('fullscreenchange', handleFullscreenChange);
  }, [mode, onModeChange]);

  const enterFullscreen = useCallback(async () => {
    const target = containerRef.current;
    if (!target || !target.requestFullscreen) {
      onModeChange('fullwindow');
      return;
    }
    try {
      await target.requestFullscreen();
      onModeChange('fullscreen');
    } catch {
      onModeChange('fullwindow');
    }
  }, [onModeChange]);

  const exitFullscreen = useCallback(async () => {
    if (document.fullscreenElement) {
      try {
        await document.exitFullscreen();
      } catch {
        // ignore; fullscreenchange handler will reset mode
      }
    }
    onModeChange('inline');
  }, [onModeChange]);

  const importRustyMilkPreset = useCallback(async (event) => {
    const files = Array.from(event.target.files || []);
    event.target.value = '';
    if (files.length === 0 || !engineRef.current?.loadPresetText) return;

    setError(null);
    const imported = [];
    let activePresetEntry = null;
    let importedFragmentCount = 0;
    const skipped = [];
    const { skippedTextureAssets, textureAssets } = await readNativeTextureAssets(files);

    for (const file of files.filter(isRustyMilkPresetFile)) {
      try {
        const source = await file.text();
        const presetTextureAssets = selectRustyMilkPresetTextureAssets(source, textureAssets);
        const importedPresetName = engineRef.current.inspectPresetText
          ? engineRef.current.inspectPresetText(source, file.name).title
          : engineRef.current.loadPresetText(source, file.name);
        imported.push({
          fileName: file.name,
          id: getRustyMilkPresetFileId(file),
          source,
          textureAssets: presetTextureAssets,
          title: importedPresetName,
        });
      } catch (presetError) {
        // eslint-disable-next-line no-console
        console.error('Failed to import RustyMilk preset', presetError);
        skipped.push({
          fileName: file.name,
          message: presetError?.message || 'Unsupported syntax or shader features may be present.',
        });
      }
    }

    if (imported.length > 0) {
      const activePreset = imported[imported.length - 1];
      let activePresetName = engineRef.current.loadPresetText(
        activePreset.source,
        activePreset.fileName,
        { textureAssets: activePreset.textureAssets },
      );
      if (isPromiseLike(activePresetName)) {
        activePresetName = await activePresetName;
      }
      activePreset.title = activePresetName;
      activePresetEntry = activePreset;
      setLocalStorageItem(rustyMilkPresetStorageKey, JSON.stringify(activePreset));
      setActiveRustyMilkPresetId(activePreset.id);
      refreshRustyMilkFragmentSummary();
      setRustyMilkPresetLibrary((library) => {
        const nextLibrary = imported.reduce(
          (next, entry) => upsertRustyMilkPresetLibraryEntry(next, entry),
          library,
        );
        writeStoredRustyMilkPresetLibrary(nextLibrary);
        return nextLibrary;
      });
      setPresetName(activePresetName);
      sizeCanvas();
    }

    for (const file of files.filter(isRustyMilkFragmentFile)) {
      if (!engineRef.current?.loadPresetFragmentText) {
        skipped.push({
          fileName: file.name,
          message: 'Native fragment import is not available.',
        });
        continue;
      }
      try {
        const source = await file.text();
        const fragmentTextureAssets = selectRustyMilkPresetTextureAssets(source, textureAssets);
        const mergedTextureAssets = {
          ...(activePresetEntry?.textureAssets || {}),
          ...fragmentTextureAssets,
        };
        let result = engineRef.current.loadPresetFragmentText(source, file.name, {
          textureAssets: mergedTextureAssets,
        });
        if (isPromiseLike(result)) {
          result = await result;
        }
        const existingPreset = activePresetEntry || readStoredRustyMilkPreset();
        const mergedPreset = {
          fileName: existingPreset?.fileName || file.name,
          id: existingPreset?.id || `fragment:${getRustyMilkPresetFileId(file)}`,
          source: result.source,
          textureAssets: {
            ...(existingPreset?.textureAssets || {}),
            ...fragmentTextureAssets,
          },
          title: result.title,
        };
        activePresetEntry = mergedPreset;
        importedFragmentCount += 1;
        setLocalStorageItem(rustyMilkPresetStorageKey, JSON.stringify(mergedPreset));
        setActiveRustyMilkPresetId(mergedPreset.id);
        refreshRustyMilkFragmentSummary();
        setRustyMilkPresetLibrary((library) => {
          const nextLibrary = upsertRustyMilkPresetLibraryEntry(library, mergedPreset);
          writeStoredRustyMilkPresetLibrary(nextLibrary);
          return nextLibrary;
        });
        setPresetName(result.title);
        sizeCanvas();
      } catch (presetError) {
        // eslint-disable-next-line no-console
        console.error('Failed to import RustyMilk fragment', presetError);
        skipped.push({
          fileName: file.name,
          message: presetError?.message || 'Unsupported fragment syntax may be present.',
        });
      }
    }

    const importMessage = getRustyMilkPresetImportMessage({
      importedCount: imported.length + importedFragmentCount,
      skipped,
      skippedTextureAssets,
    });
    if (importMessage) {
      setError(importMessage);
    }
  }, [refreshRustyMilkFragmentSummary, sizeCanvas]);

  const exportRustyMilkFragment = useCallback((type) => {
    if (!engineRef.current?.exportPresetFragment) return;
    try {
      const selectedIndex = type === 'wave' ? selectedNativeWaveIndex : selectedNativeShapeIndex;
      const exported = engineRef.current.exportPresetFragment(type, selectedIndex);
      if (!exported) {
        setError(`No ${type} fragment is available in the active RustyMilk preset.`);
        return;
      }
      downloadTextFile(exported.fileName, exported.source);
      setError(null);
    } catch (exportError) {
      // eslint-disable-next-line no-console
      console.error('Failed to export RustyMilk fragment', exportError);
      setError(exportError?.message || 'Native fragment export failed.');
    }
  }, [selectedNativeShapeIndex, selectedNativeWaveIndex]);

  const exportRustyMilkPreset = useCallback(() => {
    if (!engineRef.current?.exportPresetText) return;
    try {
      const exported = engineRef.current.exportPresetText();
      if (!exported) {
        setError('No RustyMilk preset is available to export.');
        return;
      }
      downloadTextFile(exported.fileName, exported.source);
      setError(null);
    } catch (exportError) {
      // eslint-disable-next-line no-console
      console.error('Failed to export RustyMilk preset', exportError);
      setError(exportError?.message || 'Native preset export failed.');
    }
  }, []);

  const removeRustyMilkFragment = useCallback(async (type) => {
    if (!engineRef.current?.removePresetFragment) return;
    const selectedIndex = type === 'wave' ? selectedNativeWaveIndex : selectedNativeShapeIndex;
    try {
      const storedPreset = readStoredRustyMilkPreset();
      let result = engineRef.current.removePresetFragment(type, selectedIndex, {
        textureAssets: storedPreset?.textureAssets,
      });
      if (isPromiseLike(result)) {
        result = await result;
      }
      if (!result) {
        setError(`No ${type} fragment is available in the active RustyMilk preset.`);
        return;
      }
      const editedPreset = {
        fileName: storedPreset?.fileName || 'edited-native.milk',
        id: storedPreset?.id || `edited:${Date.now().toString(36)}`,
        source: result.source,
        textureAssets: storedPreset?.textureAssets || {},
        title: result.title,
      };
      setLocalStorageItem(rustyMilkPresetStorageKey, JSON.stringify(editedPreset));
      setActiveRustyMilkPresetId(editedPreset.id);
      setRustyMilkPresetLibrary((library) => {
        const nextLibrary = upsertRustyMilkPresetLibraryEntry(library, editedPreset);
        writeStoredRustyMilkPresetLibrary(nextLibrary);
        return nextLibrary;
      });
      setPresetName(result.title);
      refreshRustyMilkFragmentSummary();
      setError(null);
      sizeCanvas();
    } catch (removeError) {
      // eslint-disable-next-line no-console
      console.error('Failed to remove RustyMilk fragment', removeError);
      setError(removeError?.message || 'Native fragment removal failed.');
    }
  }, [
    refreshRustyMilkFragmentSummary,
    selectedNativeShapeIndex,
    selectedNativeWaveIndex,
    sizeCanvas,
  ]);

  const applyRustyMilkParameterEdit = useCallback(async () => {
    if (!engineRef.current?.updatePresetBaseValue) return;
    try {
      const storedPreset = readStoredRustyMilkPreset();
      let result = engineRef.current.updatePresetBaseValue(
        selectedRustyMilkParameter,
        rustyMilkParameterValue,
        {
          textureAssets: storedPreset?.textureAssets,
        },
      );
      if (isPromiseLike(result)) {
        result = await result;
      }
      if (!result) {
        setError('Native parameter editing is not available for this value.');
        return;
      }
      const editedPreset = {
        fileName: storedPreset?.fileName || 'edited-native.milk',
        id: storedPreset?.id || `edited:${Date.now().toString(36)}`,
        source: result.source,
        textureAssets: storedPreset?.textureAssets || {},
        title: result.title,
      };
      setLocalStorageItem(rustyMilkPresetStorageKey, JSON.stringify(editedPreset));
      setActiveRustyMilkPresetId(editedPreset.id);
      setRustyMilkPresetLibrary((library) => {
        const nextLibrary = upsertRustyMilkPresetLibraryEntry(library, editedPreset);
        writeStoredRustyMilkPresetLibrary(nextLibrary);
        return nextLibrary;
      });
      setPresetName(result.title);
      setRustyMilkParameterValues(result.values || {});
      setRustyMilkParameterDrafts({});
      setError(null);
      sizeCanvas();
    } catch (editError) {
      // eslint-disable-next-line no-console
      console.error('Failed to edit RustyMilk parameter', editError);
      setError(editError?.message || 'Native parameter edit failed.');
    }
  }, [rustyMilkParameterValue, selectedRustyMilkParameter, sizeCanvas]);

  const randomizeRustyMilkPresetParameters = useCallback(async () => {
    if (!engineRef.current?.randomizePresetParameters) return;
    try {
      const storedPreset = readStoredRustyMilkPreset();
      let result = engineRef.current.randomizePresetParameters({
        textureAssets: storedPreset?.textureAssets,
      });
      if (isPromiseLike(result)) {
        result = await result;
      }
      if (!result) {
        setError('Native parameter randomization is not available for this preset.');
        return;
      }
      const editedPreset = {
        fileName: storedPreset?.fileName || 'randomized-native.milk',
        id: storedPreset?.id || `randomized:${Date.now().toString(36)}`,
        source: result.source,
        textureAssets: storedPreset?.textureAssets || {},
        title: result.title,
      };
      setLocalStorageItem(rustyMilkPresetStorageKey, JSON.stringify(editedPreset));
      setActiveRustyMilkPresetId(editedPreset.id);
      setRustyMilkPresetLibrary((library) => {
        const nextLibrary = upsertRustyMilkPresetLibraryEntry(library, editedPreset);
        writeStoredRustyMilkPresetLibrary(nextLibrary);
        return nextLibrary;
      });
      setPresetName(result.title);
      setRustyMilkParameterValues(result.values || {});
      setRustyMilkParameterDrafts({});
      refreshRustyMilkFragmentSummary();
      setError(null);
      sizeCanvas();
    } catch (randomizeError) {
      // eslint-disable-next-line no-console
      console.error('Failed to randomize RustyMilk preset', randomizeError);
      setError(randomizeError?.message || 'Native parameter randomization failed.');
    }
  }, [refreshRustyMilkFragmentSummary, sizeCanvas]);

  const loadRustyMilkLibraryPreset = useCallback((event) => {
    const preset = rustyMilkPresetLibrary.find((entry) => entry.id === event.target.value);
    void loadRustyMilkPresetEntry(preset);
  }, [loadRustyMilkPresetEntry, rustyMilkPresetLibrary]);

  const clearRustyMilkPresetLibrary = useCallback(() => {
    removeLocalStorageItem(rustyMilkPresetStorageKey);
    removeLocalStorageItem(rustyMilkPresetLibraryStorageKey);
    removeLocalStorageItem(rustyMilkPresetFavoritesStorageKey);
    removeLocalStorageItem(rustyMilkPresetLibraryModeStorageKey);
    removeLocalStorageItem(rustyMilkPresetSearchStorageKey);
    removeLocalStorageItem(rustyMilkPresetPlaylistsStorageKey);
    removeLocalStorageItem(activeRustyMilkPresetPlaylistStorageKey);
    setActiveRustyMilkPresetId('');
    setRustyMilkFavoritePresetIds([]);
    setRustyMilkFragmentSummary({ shapes: [], waves: [] });
    setNativeDebugSnapshot(null);
    setRustyMilkParameterDrafts({});
    setRustyMilkParameterValues({});
    setRustyMilkLibraryMode('all');
    setRustyMilkPresetHistory([]);
    setRustyMilkPresetLibrary([]);
    setRustyMilkPresetSearch('');
    setRustyMilkPresetPlaylists([]);
    setActiveNativePlaylistId('');
    setError(null);
  }, []);

  const removeActiveRustyMilkPreset = useCallback(() => {
    if (!activeRustyMilkPresetId) return;
    setRustyMilkPresetLibrary((library) => {
      const nextLibrary = library.filter((preset) => preset.id !== activeRustyMilkPresetId);
      if (nextLibrary.length > 0) {
        writeStoredRustyMilkPresetLibrary(nextLibrary);
      } else {
        removeLocalStorageItem(rustyMilkPresetLibraryStorageKey);
      }
      const nextFavoriteIds = pruneRustyMilkPresetFavorites(rustyMilkFavoritePresetIds, nextLibrary);
      setRustyMilkFavoritePresetIds(nextFavoriteIds);
      writeStoredRustyMilkPresetFavorites(nextFavoriteIds);
      return nextLibrary;
    });
    setRustyMilkPresetHistory((history) => history.filter((id) => id !== activeRustyMilkPresetId));
    const storedRustyMilkPreset = readStoredRustyMilkPreset();
    if (storedRustyMilkPreset?.id === activeRustyMilkPresetId) {
      removeLocalStorageItem(rustyMilkPresetStorageKey);
    }
    setActiveRustyMilkPresetId('');
    setError(null);
  }, [activeRustyMilkPresetId, rustyMilkFavoritePresetIds]);

  const updateNativeMouseState = useCallback((event) => {
    if (!isRustyMilkEngine(activeEngineType) || !engineRef.current?.setMouseState) return;
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect?.width || !rect?.height) return;
    const mouseX = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    const mouseY = Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height));
    const previous = lastNativeMouseRef.current;
    lastNativeMouseRef.current = { x: mouseX, y: mouseY };
    engineRef.current.setMouseState({
      mouse_down: event.buttons > 0 ? 1 : 0,
      mouse_dx: mouseX - previous.x,
      mouse_dy: mouseY - previous.y,
      mouse_x: mouseX,
      mouse_y: mouseY,
    });
  }, [activeEngineType]);

  const clearNativeMouseState = useCallback(() => {
    if (!isRustyMilkEngine(activeEngineType) || !engineRef.current?.setMouseState) return;
    engineRef.current.setMouseState({
      mouse_down: 0,
      mouse_dx: 0,
      mouse_dy: 0,
    });
  }, [activeEngineType]);

  if (mode === 'off') return null;

  const className = [
    'player-visualizer',
    `player-visualizer-${mode}`,
    compactControls ? 'player-visualizer-compact' : '',
  ].filter(Boolean).join(' ');
  const displayedError = compactControls && error
    ? error.split('.')[0]
    : error;

  return (
    <div
      className={className}
      data-testid="player-visualizer"
      onPointerDown={updateNativeMouseState}
      onPointerLeave={clearNativeMouseState}
      onPointerMove={updateNativeMouseState}
      onPointerUp={clearNativeMouseState}
      ref={containerRef}
    >
      <canvas
        className="player-visualizer-canvas"
        hidden={fallbackMode}
        key={activeEngineType}
        ref={canvasRef}
      />
      {fallbackMode ? (
        <SpectrumAnalyzer
          audioElement={audioElement}
          className="player-visualizer-fallback"
          mode="spectrum"
        />
      ) : null}
      {displayedError ? <div className="player-visualizer-error">{displayedError}</div> : null}
      <div className="player-visualizer-overlay">
        {mode !== 'inline' && (engineName || presetName) ? (
          <div className="player-visualizer-preset" title={presetName}>
            {[engineName, presetName].filter(Boolean).join(' · ')}
          </div>
          ) : null}
        {showNativeDebug && nativeDebugSnapshot ? (
          <div className="player-visualizer-debug" data-testid="visualizer-native-debug">
            <div title={nativeDebugSnapshot.title}>{nativeDebugSnapshot.title}</div>
            <div>
              {nativeDebugSnapshot.format}
              {' · '}
              {nativeDebugSnapshot.presetCount}
              {' preset'}
              {nativeDebugSnapshot.presetCount === 1 ? '' : 's'}
            </div>
            <div>
              {nativeDebugSnapshot.shapes}
              {' shapes · '}
              {nativeDebugSnapshot.waves}
              {' waves · '}
              {nativeDebugSnapshot.sprites}
              {' sprites'}
            </div>
            <div>
              {nativeDebugSnapshot.shaderSections.warp ? 'warp' : 'no warp'}
              {' / '}
              {nativeDebugSnapshot.shaderSections.comp ? 'comp' : 'no comp'}
            </div>
            <div>
              {nativeFpsCap === 'full' ? 'uncapped' : `${nativeFpsCap} fps`}
              {' · '}
              {rustyMilkFrameMs.toFixed(1)}
              {' ms'}
            </div>
            <div>
              {nativeQualityPreset === 'custom'
                ? 'custom quality'
                : `${nativeQualityPresets[nativeQualityPreset]?.label || 'Balanced'} quality`}
              {' · '}
              {getRustyMilkWebGpuDebugLabel(nativeDebugSnapshot.webGpu)}
            </div>
          </div>
        ) : null}
        <div
          className="player-visualizer-overlay-controls"
          onClick={(event) => event.stopPropagation()}
        >
          <input
            accept=".milk,.milk2,.shape,.wave,text/plain,image/png,image/jpeg,image/webp,image/gif"
            hidden
            multiple
            onChange={importRustyMilkPreset}
            ref={fileInputRef}
            type="file"
          />
          <input
            data-testid="visualizer-native-pack-input"
            directory=""
            hidden
            multiple
            onChange={importRustyMilkPreset}
            ref={directoryInputRef}
            type="file"
            webkitdirectory=""
          />
          <Popup
            content={`Cycle visualizer engine to ${getEngineLabel(getNextEngine(activeEngineType))}.`}
            trigger={
              <Button
                aria-label={`Cycle visualizer engine to ${getEngineLabel(getNextEngine(activeEngineType))}`}
                data-testid="visualizer-switch-engine"
                icon
                onClick={cycleEngineType}
                size="mini"
              >
                <Icon name={getEngineIcon(activeEngineType)} />
              </Button>
            }
          />
          {isRustyMilkEngine(activeEngineType) && !compactControls ? (
            <>
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Filter imported RustyMilk presets by title or file name. The current filter also scopes next and random preset jumps."
                  trigger={
                    <input
                      aria-label="Search RustyMilk presets"
                      className="player-visualizer-native-search"
                      data-testid="visualizer-native-preset-search"
                      onChange={updateRustyMilkPresetSearch}
                      placeholder="Search presets"
                      type="search"
                      value={rustyMilkPresetSearch}
                    />
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Clear the RustyMilk preset search filter."
                  trigger={
                    <Button
                      aria-label="Clear RustyMilk preset search"
                      data-testid="visualizer-clear-native-preset-search"
                      disabled={!hasRustyMilkPresetSearch}
                      icon
                      onClick={clearRustyMilkPresetSearch}
                      size="mini"
                    >
                      <Icon name="remove" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetPlaylists.length > 0 ? (
                <Popup
                  content="Use a saved native playlist as the active preset bank."
                  trigger={
                    <select
                      aria-label="RustyMilk playlist"
                      className="player-visualizer-native-library"
                      data-testid="visualizer-native-playlist"
                      onChange={selectNativePlaylist}
                      value={activeNativePlaylistId}
                    >
                      <option value="">All imported</option>
                      {rustyMilkPresetPlaylists.map((playlist) => (
                        <option key={playlist.id} value={playlist.id}>
                          {playlist.name}
                        </option>
                      ))}
                    </select>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Save the current visible RustyMilk preset bank as a browser-local playlist."
                  trigger={
                    <Button
                      aria-label="Save visible RustyMilk presets as playlist"
                      data-testid="visualizer-save-native-playlist"
                      disabled={!canSaveNativePlaylist}
                      icon
                      onClick={saveNativePlaylistFromVisibleBank}
                      size="mini"
                    >
                      <Icon name="save outline" />
                    </Button>
                  }
                />
              ) : null}
              {activeNativePlaylistId ? (
                <Popup
                  content="Rename the active native playlist in this browser."
                  trigger={
                    <Button
                      aria-label="Rename active native playlist"
                      data-testid="visualizer-rename-native-playlist"
                      icon
                      onClick={renameActiveNativePlaylist}
                      size="mini"
                    >
                      <Icon name="edit outline" />
                    </Button>
                  }
                />
              ) : null}
              {activeNativePlaylistId ? (
                <Popup
                  content="Return to the full imported RustyMilk preset bank without deleting this playlist."
                  trigger={
                    <Button
                      aria-label="Clear active native playlist"
                      data-testid="visualizer-clear-active-native-playlist"
                      icon
                      onClick={clearActiveNativePlaylist}
                      size="mini"
                    >
                      <Icon name="list" />
                    </Button>
                  }
                />
              ) : null}
              {activeNativePlaylistId ? (
                <Popup
                  content="Delete the active native playlist from this browser."
                  trigger={
                    <Button
                      aria-label="Delete active native playlist"
                      data-testid="visualizer-remove-native-playlist"
                      icon
                      onClick={removeActiveNativePlaylist}
                      size="mini"
                    >
                      <Icon name="times circle outline" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content={
                    rustyMilkLibraryMode === 'favorites'
                      ? 'Reload a favorite RustyMilk preset from this browser.'
                      : 'Reload a previously imported RustyMilk preset from this browser.'
                  }
                  trigger={
                    <select
                      aria-label="RustyMilk preset library"
                      className="player-visualizer-native-library"
                      data-testid="visualizer-native-preset-library"
                      onChange={loadRustyMilkLibraryPreset}
                      value={selectedRustyMilkPresetValue}
                    >
                      <option value="">
                        {visibleRustyMilkPresetLibrary.length === 0 ? 'No matches' : (
                          rustyMilkLibraryMode === 'favorites' ? 'Favorites' : 'Presets'
                        )}
                      </option>
                      {visibleRustyMilkPresetLibrary.map((preset) => (
                        <option key={preset.id} value={preset.id}>
                          {rustyMilkFavoritePresetIds.includes(preset.id) ? '(favorite) ' : ''}
                          {preset.title || preset.fileName}
                        </option>
                      ))}
                    </select>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content={
                    activeRustyMilkPresetIsFavorite
                      ? 'Remove the active RustyMilk preset from favorites.'
                      : 'Mark the active RustyMilk preset as a favorite.'
                  }
                  trigger={
                    <Button
                      aria-label={
                        activeRustyMilkPresetIsFavorite
                          ? 'Unfavorite active RustyMilk preset'
                          : 'Favorite active RustyMilk preset'
                      }
                      active={activeRustyMilkPresetIsFavorite}
                      data-testid="visualizer-toggle-native-favorite"
                      disabled={!activeRustyMilkPresetId}
                      icon
                      onClick={toggleRustyMilkPresetFavorite}
                      size="mini"
                    >
                      <Icon name={activeRustyMilkPresetIsFavorite ? 'star' : 'star outline'} />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content={
                    rustyMilkLibraryMode === 'favorites'
                      ? 'Show all imported RustyMilk presets.'
                      : 'Show only favorite RustyMilk presets.'
                  }
                  trigger={
                    <Button
                      aria-label={
                        rustyMilkLibraryMode === 'favorites'
                          ? 'Show all RustyMilk presets'
                          : 'Show favorite RustyMilk presets'
                      }
                      active={rustyMilkLibraryMode === 'favorites'}
                      data-testid="visualizer-toggle-native-favorites-only"
                      disabled={rustyMilkFavoritePresetIds.length === 0}
                      icon
                      onClick={toggleRustyMilkLibraryMode}
                      size="mini"
                    >
                      <Icon name="filter" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 1 ? (
                <Popup
                  content="Return to the previous RustyMilk preset, or move backward in the local preset library."
                  trigger={
                    <Button
                      aria-label="Previous RustyMilk preset"
                      data-testid="visualizer-previous-native-preset"
                      disabled={visibleRustyMilkPresetLibrary.length === 0}
                      icon
                      onClick={previousRustyMilkLibraryPreset}
                      size="mini"
                    >
                      <Icon name="step backward" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 1 ? (
                <Popup
                  content="Jump to a random imported RustyMilk preset from this browser."
                  trigger={
                    <Button
                      aria-label="Random imported RustyMilk preset"
                      data-testid="visualizer-random-native-preset"
                      disabled={visibleRustyMilkPresetLibrary.length === 0}
                      icon
                      onClick={randomRustyMilkLibraryPreset}
                      size="mini"
                    >
                      <Icon name="random" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Remove the selected RustyMilk preset from this browser."
                  trigger={
                    <Button
                      aria-label="Remove selected RustyMilk preset"
                      data-testid="visualizer-remove-native-preset"
                      disabled={!activeRustyMilkPresetId}
                      icon
                      onClick={removeActiveRustyMilkPreset}
                      size="mini"
                    >
                      <Icon name="minus circle" />
                    </Button>
                  }
                />
              ) : null}
              {rustyMilkPresetLibrary.length > 0 ? (
                <Popup
                  content="Clear imported RustyMilk presets from this browser."
                  trigger={
                    <Button
                      aria-label="Clear imported RustyMilk presets"
                      data-testid="visualizer-clear-native-preset-library"
                      icon
                      onClick={clearRustyMilkPresetLibrary}
                      size="mini"
                    >
                      <Icon name="trash alternate outline" />
                    </Button>
                  }
                />
              ) : null}
              <Popup
                content="Import a local .milk or .milk2 preset into the RustyMilk renderer."
                trigger={
                  <Button
                    aria-label="Import RustyMilk preset"
                    data-testid="visualizer-import-native-preset"
                    icon
                    onClick={() => fileInputRef.current?.click()}
                    size="mini"
                  >
                    <Icon name="upload" />
                  </Button>
                }
              />
              <Popup
                content="Import a RustyMilk preset folder with its local image assets."
                trigger={
                  <Button
                    aria-label="Import RustyMilk preset folder"
                    data-testid="visualizer-import-native-preset-folder"
                    icon
                    onClick={() => directoryInputRef.current?.click()}
                    size="mini"
                  >
                    <Icon name="folder open outline" />
                  </Button>
                }
              />
              <Popup
                content="Choose a global RustyMilk parameter to edit on the active preset."
                trigger={
                  <select
                    aria-label="RustyMilk editable parameter"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-parameter"
                    onChange={selectRustyMilkParameter}
                    value={selectedRustyMilkParameter}
                  >
                    {nativeEditableParameters.map((parameter) => (
                      <option key={parameter.key} value={parameter.key}>
                        {parameter.label}
                      </option>
                    ))}
                  </select>
                }
              />
              <Popup
                content={`Adjust ${rustyMilkParameter.label.toLowerCase()} for the active RustyMilk preset before applying the edited copy locally.`}
                trigger={
                  <input
                    aria-label={`Adjust RustyMilk ${rustyMilkParameter.label}`}
                    className="player-visualizer-native-range"
                    data-testid="visualizer-native-parameter-value"
                    max={rustyMilkParameter.max}
                    min={rustyMilkParameter.min}
                    onChange={updateRustyMilkParameterDraft}
                    step={rustyMilkParameter.step}
                    type="range"
                    value={rustyMilkParameterValue}
                  />
                }
              />
              <Popup
                content="Apply the selected RustyMilk parameter value and save the edited preset in this browser."
                trigger={
                  <Button
                    aria-label="Apply RustyMilk parameter edit"
                    data-testid="visualizer-apply-native-parameter"
                    icon
                    onClick={applyRustyMilkParameterEdit}
                    size="mini"
                  >
                    <Icon name="sliders horizontal" />
                  </Button>
                }
              />
              <Popup
                content="Randomize the active RustyMilk preset's common visual parameters and save the edited copy locally."
                trigger={
                  <Button
                    aria-label="Randomize RustyMilk visual parameters"
                    data-testid="visualizer-randomize-native-parameters"
                    icon
                    onClick={randomizeRustyMilkPresetParameters}
                    size="mini"
                  >
                    <Icon name="shuffle" />
                  </Button>
                }
              />
              <Popup
                content="Show or hide RustyMilk debug details for the active preset."
                trigger={
                  <Button
                    aria-label={showNativeDebug ? 'Hide RustyMilk debug details' : 'Show RustyMilk debug details'}
                    active={showNativeDebug}
                    data-testid="visualizer-toggle-native-debug"
                    icon
                    onClick={() => setShowNativeDebug((current) => !current)}
                    size="mini"
                  >
                    <Icon name="bug" />
                  </Button>
                }
              />
              <Popup
                content="Cap RustyMilk rendering for lower GPU load, or leave it uncapped for maximum smoothness."
                trigger={
                  <select
                    aria-label="RustyMilk FPS cap"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-fps-cap"
                    onChange={updateNativeFpsCap}
                    value={nativeFpsCap}
                  >
                    <option value="full">Full FPS</option>
                    <option value="60">60 FPS</option>
                    <option value="30">30 FPS</option>
                    <option value="24">24 FPS</option>
                  </select>
                }
              />
              <Popup
                content="Choose a RustyMilk quality preset. Efficient lowers GPU load, Balanced caps at 60 FPS, and Full leaves rendering uncapped."
                trigger={
                  <select
                    aria-label="RustyMilk quality preset"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-quality"
                    onChange={updateNativeQualityPreset}
                    value={nativeQualityPreset}
                  >
                    <option value="balanced">Balanced</option>
                    <option value="efficient">Efficient</option>
                    <option value="full">Full</option>
                    <option value="custom">Custom</option>
                  </select>
                }
              />
              <Popup
                content="Export the active RustyMilk preset text after any local edits."
                trigger={
                  <Button
                    aria-label="Export active RustyMilk preset"
                    data-testid="visualizer-export-native-preset"
                    icon
                    onClick={exportRustyMilkPreset}
                    size="mini"
                  >
                    <Icon name="file alternate outline" />
                  </Button>
                }
              />
              <Popup
                content="Choose which custom shape from the active RustyMilk preset should be exported or removed."
                trigger={
                  <select
                    aria-label="RustyMilk shape fragment"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-shape-fragment"
                    disabled={!hasNativeShapes}
                    onChange={(event) => setSelectedNativeShapeIndex(Number(event.target.value))}
                    value={hasNativeShapes ? selectedNativeShapeIndex : 0}
                  >
                    {hasNativeShapes ? rustyMilkFragmentSummary.shapes.map((shape) => (
                      <option key={shape.index} value={shape.index}>
                        {shape.label}
                      </option>
                    )) : (
                      <option value={0}>No shapes</option>
                    )}
                  </select>
                }
              />
              <Popup
                content="Export the selected custom shape in the active RustyMilk preset as a .shape fragment."
                trigger={
                  <Button
                    aria-label="Export RustyMilk shape fragment"
                    data-testid="visualizer-export-native-shape"
                    disabled={!hasNativeShapes}
                    icon
                    onClick={() => exportRustyMilkFragment('shape')}
                    size="mini"
                  >
                    <Icon name="download" />
                  </Button>
                }
              />
              <Popup
                content="Remove the selected custom shape from the active RustyMilk preset and persist the edited copy locally."
                trigger={
                  <Button
                    aria-label="Remove RustyMilk shape fragment"
                    data-testid="visualizer-remove-native-shape"
                    disabled={!hasNativeShapes}
                    icon
                    onClick={() => removeRustyMilkFragment('shape')}
                    size="mini"
                  >
                    <Icon name="erase" />
                  </Button>
                }
              />
              <Popup
                content="Choose which custom wave from the active RustyMilk preset should be exported or removed."
                trigger={
                  <select
                    aria-label="RustyMilk wave fragment"
                    className="player-visualizer-native-library"
                    data-testid="visualizer-native-wave-fragment"
                    disabled={!hasNativeWaves}
                    onChange={(event) => setSelectedNativeWaveIndex(Number(event.target.value))}
                    value={hasNativeWaves ? selectedNativeWaveIndex : 0}
                  >
                    {hasNativeWaves ? rustyMilkFragmentSummary.waves.map((wave) => (
                      <option key={wave.index} value={wave.index}>
                        {wave.label}
                      </option>
                    )) : (
                      <option value={0}>No waves</option>
                    )}
                  </select>
                }
              />
              <Popup
                content="Export the selected custom wave in the active RustyMilk preset as a .wave fragment."
                trigger={
                  <Button
                    aria-label="Export RustyMilk wave fragment"
                    data-testid="visualizer-export-native-wave"
                    disabled={!hasNativeWaves}
                    icon
                    onClick={() => exportRustyMilkFragment('wave')}
                    size="mini"
                  >
                    <Icon name="download" />
                  </Button>
                }
              />
              <Popup
                content="Remove the selected custom wave from the active RustyMilk preset and persist the edited copy locally."
                trigger={
                  <Button
                    aria-label="Remove RustyMilk wave fragment"
                    data-testid="visualizer-remove-native-wave"
                    disabled={!hasNativeWaves}
                    icon
                    onClick={() => removeRustyMilkFragment('wave')}
                    size="mini"
                  >
                    <Icon name="erase" />
                  </Button>
                }
              />
              <Popup
                content={`Native automatic preset changes: ${getRustyMilkAutomationLabel(rustyMilkAutomationMode)}. Beat mode advances after repeated detected bass beats; timed mode advances on an interval.`}
                trigger={
                  <Button
                    aria-label={`Native automatic preset changes: ${getRustyMilkAutomationLabel(rustyMilkAutomationMode)}`}
                    active={rustyMilkAutomationMode !== 'off'}
                    data-testid="visualizer-native-automation"
                    icon
                    onClick={cycleRustyMilkAutomationMode}
                    size="mini"
                  >
                    <Icon name={rustyMilkAutomationMode === 'beat' ? 'heartbeat' : 'clock outline'} />
                  </Button>
                }
              />
              {rustyMilkAutomationMode === 'beat' ? (
                <Popup
                  content="Choose how many detected bass beats should pass before RustyMilk advances to another preset."
                  trigger={
                    <select
                      aria-label="RustyMilk beats per preset"
                      className="player-visualizer-native-library"
                      data-testid="visualizer-native-automation-beats"
                      onChange={updateRustyMilkAutomationBeats}
                      value={rustyMilkAutomationSettings.beatsPerPreset}
                    >
                      <option value={4}>4 beats</option>
                      <option value={8}>8 beats</option>
                      <option value={16}>16 beats</option>
                    </select>
                  }
                />
              ) : null}
              {rustyMilkAutomationMode === 'timed' ? (
                <Popup
                  content="Choose how long RustyMilk should wait before timed preset changes."
                  trigger={
                    <select
                      aria-label="RustyMilk timed preset interval"
                      className="player-visualizer-native-library"
                      data-testid="visualizer-native-automation-interval"
                      onChange={updateRustyMilkAutomationInterval}
                      value={rustyMilkAutomationSettings.timedIntervalSeconds}
                    >
                      <option value={15}>15 sec</option>
                      <option value={30}>30 sec</option>
                      <option value={60}>60 sec</option>
                    </select>
                  }
                />
              ) : null}
            </>
          ) : null}
          <Popup
            content={
              nativeBankNavigationDisabled
                ? 'No imported RustyMilk presets match the current filter.'
                : 'Load a different RustyMilk preset.'
            }
            trigger={
              <Button
                aria-label="Next visualizer preset"
                data-testid="visualizer-next-preset"
                disabled={nativeBankNavigationDisabled}
                icon
                onClick={cyclePreset}
                size="mini"
              >
                <Icon name="random" />
              </Button>
            }
          />
          {mode === 'inline' && !compactControls ? (
            <>
              <Popup
                content="Expand visualizer to fill the browser window."
                trigger={
                  <Button
                    aria-label="Expand visualizer to full browser window"
                    data-testid="visualizer-fullwindow"
                    icon
                    onClick={() => onModeChange('fullwindow')}
                    size="mini"
                  >
                    <Icon name="expand arrows alternate" />
                  </Button>
                }
              />
              <Popup
                content="Enter true fullscreen."
                trigger={
                  <Button
                    aria-label="Enter fullscreen visualizer"
                    data-testid="visualizer-fullscreen"
                    icon
                    onClick={enterFullscreen}
                    size="mini"
                  >
                    <Icon name="expand" />
                  </Button>
                }
              />
            </>
          ) : mode === 'inline' ? null : (
            <>
              {mode === 'fullwindow' ? (
                <Popup
                  content="Enter true fullscreen."
                  trigger={
                    <Button
                      aria-label="Enter fullscreen visualizer"
                      data-testid="visualizer-fullscreen"
                      icon
                      onClick={enterFullscreen}
                      size="mini"
                    >
                      <Icon name="expand" />
                    </Button>
                  }
                />
              ) : null}
              <Popup
                content="Return visualizer to the player bar."
                trigger={
                  <Button
                    aria-label="Collapse visualizer"
                    data-testid="visualizer-collapse"
                    icon
                    onClick={exitFullscreen}
                    size="mini"
                  >
                    <Icon name="compress" />
                  </Button>
                }
              />
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default Visualizer;
