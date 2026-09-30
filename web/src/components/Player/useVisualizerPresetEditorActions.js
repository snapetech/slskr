import { useCallback } from 'react';
import { removeLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import { toDisplayError } from '../../lib/errors';
import {
  isPromiseLike,
  readStoredRustyMilkPreset,
  rustyMilkPresetStorageKey,
  upsertRustyMilkPresetLibraryEntry,
  writeStoredRustyMilkPresetLibrary,
} from './visualizerPresetLibrary';

const useVisualizerPresetEditorActions = ({
  engineRef,
  operation,
  parameters,
  refreshRustyMilkFragmentSummary,
  selectedFragmentIndices,
  setters,
  sizeCanvas,
}) => {
  const { beginNativeOperation, finishNativeOperation, mountedRef } = operation;
  const { rustyMilkParameterValue, selectedRustyMilkParameter } = parameters;
  const { selectedNativeShapeIndex, selectedNativeWaveIndex } = selectedFragmentIndices;
  const {
    setActiveRustyMilkPresetId,
    setError,
    setPresetName,
    setRustyMilkParameterDrafts,
    setRustyMilkParameterValues,
    setRustyMilkPresetLibrary,
  } = setters;
  const removeRustyMilkFragment = useCallback(async (type) => {
    if (
      !engineRef.current?.removePresetFragment ||
      !beginNativeOperation()
    ) return;
    const selectedIndex = type === 'wave' ? selectedNativeWaveIndex : selectedNativeShapeIndex;
    try {
      const storedPreset = readStoredRustyMilkPreset();
      let result = engineRef.current.removePresetFragment(type, selectedIndex, {
        textureAssets: storedPreset?.textureAssets,
      });
      if (isPromiseLike(result)) {
        result = await result;
      }
      if (!mountedRef.current) return;
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
      if (mountedRef.current) {
        setError(toDisplayError(removeError, 'Native fragment removal failed.'));
      }
    } finally {
      finishNativeOperation();
    }
  }, [
    beginNativeOperation,
    finishNativeOperation,
    mountedRef,
    refreshRustyMilkFragmentSummary,
    selectedNativeShapeIndex,
    selectedNativeWaveIndex,
    sizeCanvas,
  ]);

  const applyRustyMilkParameterEdit = useCallback(async () => {
    if (
      !engineRef.current?.updatePresetBaseValue ||
      !beginNativeOperation()
    ) return;
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
      if (!mountedRef.current) return;
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
      if (mountedRef.current) {
        setError(toDisplayError(editError, 'Native parameter edit failed.'));
      }
    } finally {
      finishNativeOperation();
    }
  }, [
    beginNativeOperation,
    finishNativeOperation,
    mountedRef,
    rustyMilkParameterValue,
    selectedRustyMilkParameter,
    sizeCanvas,
  ]);

  const randomizeRustyMilkPresetParameters = useCallback(async () => {
    if (
      !engineRef.current?.randomizePresetParameters ||
      !beginNativeOperation()
    ) return;
    try {
      const storedPreset = readStoredRustyMilkPreset();
      let result = engineRef.current.randomizePresetParameters({
        textureAssets: storedPreset?.textureAssets,
      });
      if (isPromiseLike(result)) {
        result = await result;
      }
      if (!mountedRef.current) return;
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
      if (mountedRef.current) {
        setError(toDisplayError(randomizeError, 'Native parameter randomization failed.'));
      }
    } finally {
      finishNativeOperation();
    }
  }, [
    beginNativeOperation,
    finishNativeOperation,
    mountedRef,
    refreshRustyMilkFragmentSummary,
    sizeCanvas,
  ]);

  return {
    applyRustyMilkParameterEdit,
    randomizeRustyMilkPresetParameters,
    removeRustyMilkFragment,
  };
};

export default useVisualizerPresetEditorActions;
