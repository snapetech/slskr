import { useCallback } from 'react';
import { removeLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import { toDisplayError } from '../../lib/errors';
import { readFileTextBounded } from '../../lib/fileReaders';
import {
  downloadTextFile,
  getRustyMilkPresetFileId,
  getRustyMilkPresetImportMessage,
  isPromiseLike,
  isRustyMilkFragmentFile,
  isRustyMilkPresetFile,
  nativeTextSourceMaxBytes,
  readNativeTextureAssets,
  readStoredRustyMilkPreset,
  rustyMilkPresetStorageKey,
  selectRustyMilkPresetTextureAssets,
  upsertRustyMilkPresetLibraryEntry,
  writeStoredRustyMilkPresetLibrary,
} from './visualizerPresetLibrary';

const useVisualizerPresetFileOperations = ({
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
}) => {
  const importRustyMilkPreset = useCallback(async (event) => {
    const files = Array.from(event.target.files || []);
    event.target.value = '';
    if (
      files.length === 0 ||
      !engineRef.current?.loadPresetText ||
      !beginNativeOperation()
    ) return;

    try {
      setError(null);
      const imported = [];
      let activePresetEntry = null;
      let importedFragmentCount = 0;
      const skipped = [];
      const { skippedTextureAssets, textureAssets } = await readNativeTextureAssets(files);
      if (!mountedRef.current) return;

      for (const file of files.filter(isRustyMilkPresetFile)) {
        if (!mountedRef.current) return;
        try {
          const source = await readFileTextBounded(file, nativeTextSourceMaxBytes);
          if (!mountedRef.current) return;
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
            message: toDisplayError(
              presetError,
              'Unsupported syntax or shader features may be present.',
            ),
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
        if (!mountedRef.current) return;
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
        if (!mountedRef.current) return;
        if (!engineRef.current?.loadPresetFragmentText) {
          skipped.push({
            fileName: file.name,
            message: 'Native fragment import is not available.',
          });
          continue;
        }
        try {
          const source = await readFileTextBounded(file, nativeTextSourceMaxBytes);
          if (!mountedRef.current) return;
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
          if (!mountedRef.current) return;
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
            message: toDisplayError(
              presetError,
              'Unsupported fragment syntax may be present.',
            ),
          });
        }
      }

      const importMessage = getRustyMilkPresetImportMessage({
        importedCount: imported.length + importedFragmentCount,
        skipped,
        skippedTextureAssets,
      });
      if (mountedRef.current && importMessage) {
        setError(importMessage);
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
      setError(toDisplayError(exportError, 'Native fragment export failed.'));
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
      setError(toDisplayError(exportError, 'Native preset export failed.'));
    }
  }, []);

  return {
    exportRustyMilkFragment,
    exportRustyMilkPreset,
    importRustyMilkPreset,
  };
};

export default useVisualizerPresetFileOperations;
