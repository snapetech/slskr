import { useCallback, useEffect, useRef } from 'react';
import { removeLocalStorageItem } from '../../lib/storage';
import { resumeAudioGraph } from './audioGraph';
import {
  getRustyMilkFpsCapMs,
  getRustyMilkRendererBackend,
  getVisualizerErrorMessage,
  isPromiseLike,
  isRustyMilkEngine,
  readStoredRustyMilkPreset,
  rustyMilkPresetStorageKey,
  supportsWebGl2,
} from './visualizerPresetLibrary';
import { createRustyMilkEngine } from './visualizers/rustyMilkEngine';

const useVisualizerEngineLifecycle = ({
  activeEngineType,
  audioElement,
  callbacks,
  mode,
  nativeFpsCap,
  refs,
  rustyMilkAutomationSettingsRef,
  showNativeDebug,
}) => {
  const {
    mountedRef,
    refreshRustyMilkFragmentSummary,
    setActiveRustyMilkPresetId,
    setEngineName,
    setError,
    setFallbackMode,
    setPresetName,
    setRustyMilkFrameMs,
    sizeCanvas,
  } = callbacks;
  const {
    canvasRef,
    containerRef,
    engineRef,
    lastNativeRenderAtRef,
    nativeEngineGenerationRef,
  } = refs;
  const engineAudioNodeRef = useRef(null);
  const rafRef = useRef(null);

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

  useEffect(() => {
    if (mode === 'off' || !audioElement || !canvasRef.current) return undefined;

    let cancelled = false;
    let resizeObserver = null;
    let createdEngine = null;
    const generation = ++nativeEngineGenerationRef.current;
    const isCurrent = () =>
      !cancelled &&
      mountedRef.current &&
      generation === nativeEngineGenerationRef.current;

    (async () => {
      try {
        setError(null);
        setFallbackMode(false);
        const graph = await resumeAudioGraph(audioElement);
        if (!isCurrent()) return;
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
        if (!isCurrent()) {
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
          if (!isCurrent()) {
            engine.dispose();
            return;
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
        if (isCurrent()) {
          // eslint-disable-next-line no-console
          console.error('Failed to load RustyMilk visualizer', importError);
          setError(getVisualizerErrorMessage(activeEngineType, importError));
          setFallbackMode(true);
        }
      }
    })();

    return () => {
      cancelled = true;
      nativeEngineGenerationRef.current += 1;
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
      if (mountedRef.current) setEngineName('');
    };
  }, [
    mode,
    audioElement,
    activeEngineType,
    mountedRef,
    refreshRustyMilkFragmentSummary,
    renderLoop,
    sizeCanvas,
  ]);
};

export default useVisualizerEngineLifecycle;
