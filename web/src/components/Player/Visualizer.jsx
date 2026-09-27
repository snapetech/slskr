import React, { useCallback, useEffect, useRef, useState } from 'react';
import { removeLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import { toDisplayError } from '../../lib/errors';
import SpectrumAnalyzer from './SpectrumAnalyzer';
import useVisualizerPresetFileOperations from './useVisualizerPresetFileOperations';
import useVisualizerEngineLifecycle from './useVisualizerEngineLifecycle';
import useVisualizerPresetEditorActions from './useVisualizerPresetEditorActions';
import VisualizerOverlayControls from './VisualizerOverlayControls';
import {
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
  nativePresetFieldMaxBytes,
  nativeEditableParameters,
  readStoredEngine,
  isRustyMilkEngine,
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
  nativeQualityPresets,
  readStoredRustyMilkQuality,
  getRustyMilkWebGpuDebugLabel,
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
  pruneRustyMilkPresetFavorites,
  pruneRustyMilkPresetPlaylists,
  filterRustyMilkPresetLibrary,
  getRustyMilkPresetPlaylistName,
  getRustyMilkPresetPlaylistId,
} from './visualizerPresetLibrary';
import { useMountedRef } from '../../lib/useMountedRef';

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
  const mountedRef = useMountedRef();
  const nativeOperationInFlightRef = useRef(false);
  const nativeEngineGenerationRef = useRef(0);
  const activeEngineType = engineOverride || engineType;

  const beginNativeOperation = useCallback(() => {
    if (!mountedRef.current || nativeOperationInFlightRef.current) return false;
    nativeOperationInFlightRef.current = true;
    return true;
  }, [mountedRef]);

  const finishNativeOperation = useCallback(() => {
    nativeOperationInFlightRef.current = false;
  }, []);

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
    if (
      !preset ||
      !engineRef.current?.loadPresetText ||
      !beginNativeOperation()
    ) return false;
    const { pushHistory = true } = options;
    const generation = nativeEngineGenerationRef.current;
    const isCurrentOperation = () =>
      mountedRef.current &&
      generation === nativeEngineGenerationRef.current &&
      Boolean(engineRef.current);

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
      if (!isCurrentOperation()) return false;
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
      if (isCurrentOperation()) {
        setError(toDisplayError(presetError, 'Native preset load failed.'));
      }
      return false;
    } finally {
      finishNativeOperation();
    }
  }, [
    activeRustyMilkPresetId,
    beginNativeOperation,
    finishNativeOperation,
    mountedRef,
    refreshRustyMilkFragmentSummary,
    sizeCanvas,
  ]);

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
    if (!mountedRef.current) return;
    if (isRustyMilkEngine(activeEngineType) && rustyMilkPresetLibrary.length > 0) {
      await loadRustyMilkPresetByOffset(1);
      return;
    }
    if (!engineRef.current) return;
    let nextPresetName = engineRef.current.nextPreset();
    if (isPromiseLike(nextPresetName)) {
      nextPresetName = await nextPresetName;
    }
    if (mountedRef.current && nextPresetName) {
      setPresetName(nextPresetName);
    }
  }, [
    activeEngineType,
    loadRustyMilkPresetByOffset,
    mountedRef,
    rustyMilkPresetLibrary.length,
  ]);

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
    const nextSearch = truncateUtf8(event.target.value, nativePresetFieldMaxBytes);
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
    const normalizedName = truncateUtf8(nextName?.trim(), nativePresetFieldMaxBytes);
    if (!normalizedName) return;
    const playlist = {
      createdAt: new Date().toISOString(),
      id: getRustyMilkPresetPlaylistId(),
      name: normalizedName,
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
    const normalizedName = truncateUtf8(nextName?.trim(), nativePresetFieldMaxBytes);
    if (!normalizedName) return;
    setRustyMilkPresetPlaylists((playlists) => {
      const nextPlaylists = playlists.map((playlist) =>
        (playlist.id === activePlaylist.id
          ? { ...playlist, name: normalizedName, updatedAt: new Date().toISOString() }
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

  useVisualizerEngineLifecycle({
    activeEngineType,
    audioElement,
    callbacks: {
      mountedRef,
      refreshRustyMilkFragmentSummary,
      setActiveRustyMilkPresetId,
      setEngineName,
      setError,
      setFallbackMode,
      setPresetName,
      setRustyMilkFrameMs,
      sizeCanvas,
    },
    mode,
    nativeFpsCap,
    refs: {
      canvasRef,
      containerRef,
      engineRef,
      lastNativeRenderAtRef,
      nativeEngineGenerationRef,
    },
    rustyMilkAutomationSettingsRef,
    showNativeDebug,
  });

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

  const {
    exportRustyMilkFragment,
    exportRustyMilkPreset,
    importRustyMilkPreset,
  } = useVisualizerPresetFileOperations({
    beginNativeOperation,
    engineRef,
    finishNativeOperation,
    mountedRef,
    refreshRustyMilkFragmentSummary,
    selectedNativeShapeIndex,
    selectedNativeWaveIndex,
    setActiveRustyMilkPresetId,
    setError,
    setPresetName,
    setRustyMilkPresetLibrary,
    sizeCanvas,
  });

  const {
    applyRustyMilkParameterEdit,
    randomizeRustyMilkPresetParameters,
    removeRustyMilkFragment,
  } = useVisualizerPresetEditorActions({
    engineRef,
    operation: { beginNativeOperation, finishNativeOperation, mountedRef },
    parameters: { rustyMilkParameterValue, selectedRustyMilkParameter },
    refreshRustyMilkFragmentSummary,
    selectedFragmentIndices: { selectedNativeShapeIndex, selectedNativeWaveIndex },
    setters: {
      setActiveRustyMilkPresetId,
      setError,
      setPresetName,
      setRustyMilkParameterDrafts,
      setRustyMilkParameterValues,
      setRustyMilkPresetLibrary,
    },
    sizeCanvas,
  });

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
        <VisualizerOverlayControls
          actions={{
            applyRustyMilkParameterEdit,
            clearActiveNativePlaylist,
            clearRustyMilkPresetLibrary,
            clearRustyMilkPresetSearch,
            cycleEngineType,
            cyclePreset,
            cycleRustyMilkAutomationMode,
            enterFullscreen,
            exitFullscreen,
            exportRustyMilkFragment,
            exportRustyMilkPreset,
            importRustyMilkPreset,
            loadRustyMilkLibraryPreset,
            previousRustyMilkLibraryPreset,
            randomRustyMilkLibraryPreset,
            randomizeRustyMilkPresetParameters,
            removeActiveNativePlaylist,
            removeActiveRustyMilkPreset,
            removeRustyMilkFragment,
            renameActiveNativePlaylist,
            saveNativePlaylistFromVisibleBank,
            selectNativePlaylist,
            selectRustyMilkParameter,
            setSelectedNativeShapeIndex,
            setSelectedNativeWaveIndex,
            setShowNativeDebug,
            toggleRustyMilkLibraryMode,
            toggleRustyMilkPresetFavorite,
            updateNativeFpsCap,
            updateNativeQualityPreset,
            updateRustyMilkAutomationBeats,
            updateRustyMilkAutomationInterval,
            updateRustyMilkParameterDraft,
            updateRustyMilkPresetSearch,
          }}
          compactControls={compactControls}
          mode={mode}
          onModeChange={onModeChange}
          refs={{ directoryInputRef, fileInputRef }}
          state={{
            activeEngineType,
            activeNativePlaylistId,
            activeRustyMilkPresetId,
            activeRustyMilkPresetIsFavorite,
            canSaveNativePlaylist,
            hasNativeShapes,
            hasNativeWaves,
            hasRustyMilkPresetSearch,
            nativeBankNavigationDisabled,
            nativeFpsCap,
            nativeQualityPreset,
            rustyMilkAutomationMode,
            rustyMilkAutomationSettings,
            rustyMilkFavoritePresetIds,
            rustyMilkFragmentSummary,
            rustyMilkLibraryMode,
            rustyMilkParameter,
            rustyMilkParameterValue,
            rustyMilkPresetLibrary,
            rustyMilkPresetPlaylists,
            rustyMilkPresetSearch,
            selectedNativeShapeIndex,
            selectedNativeWaveIndex,
            selectedRustyMilkParameter,
            selectedRustyMilkPresetValue,
            showNativeDebug,
            visibleRustyMilkPresetLibrary,
          }}
        />
      </div>
    </div>
  );
};

export default Visualizer;
