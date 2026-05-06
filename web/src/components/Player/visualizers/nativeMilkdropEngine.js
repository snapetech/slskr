const nativePresets = [
  {
    name: 'slskr native grid smoke',
    source: `
      name=slskr native grid smoke
      decay=0.91
      wave_r=0.12
      wave_g=0.64
      wave_b=0.88
      wave_a=0.86
      wave_scale=1.2
      zoom=1
      rot=0
      per_frame_1=wave_r=0.35+0.25*bass_att;
      per_frame_2=wave_g=0.45+0.2*mid_att;
      per_frame_3=wave_b=0.55+0.2*treb_att;
      per_frame_4=rot=0.01*sin(time*0.7);
      per_frame_5=zoom=1+0.03*sin(time*0.5);
      per_frame_6=dx=0.015*sin(time*0.6);
      per_frame_7=dy=0.015*cos(time*0.5);
      shape00_enabled=1
      shape00_sides=5
      shape00_rad=0.18
      shape00_x=0.5
      shape00_y=0.5
      shape00_r=0.1
      shape00_g=0.9
      shape00_b=0.45
      shape00_a=0.35
      shape00_r2=0.9
      shape00_g2=0.8
      shape00_b2=0.2
      shape00_a2=0.18
      shape00_border_a=0.9
      shape00_per_frame1=ang=time*0.5;
      wavecode_0_enabled=1
      wavecode_0_samples=96
      wavecode_0_spectrum=1
      wavecode_0_dots=1
      wavecode_0_r=0.7
      wavecode_0_g=0.95
      wavecode_0_b=0.25
      wavecode_0_a=0.75
      wavecode_0_per_point1=x=i;
      wavecode_0_per_point2=y=0.08+sample*0.55;
    `,
  },
  {
    name: 'slskr native waveform smoke',
    source: `
      name=slskr native waveform smoke
      decay=0.88
      wave_r=0.85
      wave_g=0.34
      wave_b=0.18
      wave_scale=1.5
      per_frame_1=dx=0.02*sin(time*0.4);
      per_frame_2=dy=0.015*cos(time*0.3);
      per_frame_3=rot=0.02*sin(time*0.2);
      shape00_enabled=1
      shape00_sides=3
      shape00_rad=0.12+0.03*bass_att
      shape00_x=0.35
      shape00_y=0.55
      shape00_r=0.9
      shape00_g=0.2
      shape00_b=0.1
      shape00_a=0.28
      shape00_additive=1
      shape01_enabled=1
      shape01_sides=6
      shape01_rad=0.08+0.02*treb_att
      shape01_x=0.67
      shape01_y=0.45
      shape01_r=0.1
      shape01_g=0.55
      shape01_b=0.95
      shape01_a=0.35
      wavecode_0_enabled=1
      wavecode_0_samples=128
      wavecode_0_r=0.95
      wavecode_0_g=0.85
      wavecode_0_b=0.2
      wavecode_0_a=0.8
      wavecode_0_per_point1=x=i;
      wavecode_0_per_point2=y=0.5+sample*0.35;
    `,
  },
];

const defaultTransitionSeconds = 1.5;
const defaultAutomation = {
  beatSensitivity: 1.35,
  beatsPerPreset: 8,
  minBeatIntervalSeconds: 0.25,
  mode: 'off',
  timedIntervalSeconds: 30,
};

let rustModulePromise;

const loadRustMilkdropModule = async () => {
  if (!rustModulePromise) {
    rustModulePromise = import(/* @vite-ignore */ '/slskr_web.js');
  }
  return rustModulePromise;
};

const createFrameReader = (audioContext, audioNode) => {
  const analyser = audioContext.createAnalyser();
  analyser.fftSize = 2048;
  audioNode.connect(analyser);

  const waveform = new Uint8Array(analyser.fftSize);
  const frequency = new Uint8Array(analyser.frequencyBinCount);

  return {
    disconnect: () => {
      try {
        audioNode.disconnect(analyser);
      } catch {
        // The shared audio graph may have been rebuilt or torn down first.
      }
    },
    read: () => {
      analyser.getByteTimeDomainData(waveform);
      analyser.getByteFrequencyData(frequency);
      const spectrum = Array.from(frequency);
      return {
        bands: getAudioBands(spectrum),
        samples: Array.from(waveform, (value) => (value - 128) / 128),
        spectrum,
      };
    },
  };
};

const getAudioBands = (spectrum = []) => {
  if (!spectrum.length) return { bass: 0, mid: 0, treble: 0 };
  const normalized = (start, end) => {
    const safeEnd = Math.max(start + 1, Math.min(end, spectrum.length));
    let total = 0;
    for (let index = start; index < safeEnd; index += 1) {
      total += spectrum[index] / 255;
    }
    return total / (safeEnd - start);
  };
  return {
    bass: normalized(0, Math.max(1, Math.floor(spectrum.length / 8))),
    mid: normalized(Math.floor(spectrum.length / 8), Math.floor(spectrum.length / 3)),
    treble: normalized(Math.floor(spectrum.length / 3), spectrum.length),
  };
};

const getSpectrumEnergy = (spectrum = []) => {
  if (!spectrum.length) return 0;
  const limit = Math.max(1, Math.min(24, spectrum.length));
  let total = 0;
  for (let index = 0; index < limit; index += 1) {
    total += Number(spectrum[index]) || 0;
  }
  return total / (limit * 255);
};

export const getNativeMilkdropBeatUpdate = (
  previous = {},
  spectrum = [],
  now = 0,
  automation = defaultAutomation,
) => {
  const energy = getSpectrumEnergy(spectrum);
  const smoothedEnergy = previous.smoothedEnergy === undefined
    ? energy
    : (previous.smoothedEnergy * 0.85) + (energy * 0.15);
  const secondsSinceBeat = now - (previous.lastBeatAt ?? -Infinity);
  const isBeat = energy > Math.max(0.05, smoothedEnergy * automation.beatSensitivity)
    && secondsSinceBeat >= automation.minBeatIntervalSeconds;
  const beatCount = isBeat ? (previous.beatCount || 0) + 1 : (previous.beatCount || 0);
  return {
    beatCount,
    energy,
    isBeat,
    lastBeatAt: isBeat ? now : previous.lastBeatAt,
    smoothedEnergy,
  };
};

export const getNativeMilkdropTransitionProgress = (startedAt, seconds, now) => {
  if (!Number.isFinite(seconds) || seconds <= 0) return 1;
  const linear = Math.max(0, Math.min(1, (now - startedAt) / seconds));
  return linear * linear * (3 - linear * 2);
};

export const getNativeMilkdropTransitionAlphas = (progress, mode = 'crossfade') => {
  const clampedProgress = Math.max(0, Math.min(1, Number(progress) || 0));
  const normalizedMode = String(mode || '').trim().toLowerCase().replace(/[\s_-]+/g, '');
  if (['fade', 'fadeblack', 'fadethroughblack'].includes(normalizedMode)) {
    return {
      incoming: clampedProgress <= 0.5 ? 0 : (clampedProgress - 0.5) * 2,
      outgoing: clampedProgress >= 0.5 ? 0 : 1 - (clampedProgress * 2),
    };
  }
  if (['overlay', 'burnin', 'hold'].includes(normalizedMode)) {
    return {
      incoming: clampedProgress,
      outgoing: 1,
    };
  }
  if (['cut', 'instant', 'none'].includes(normalizedMode)) {
    return {
      incoming: 1,
      outgoing: 0,
    };
  }
  return {
    incoming: clampedProgress,
    outgoing: 1 - clampedProgress,
  };
};

const normalizeAutomation = (automation = {}) => ({
  ...defaultAutomation,
  ...automation,
  mode: ['beat', 'timed'].includes(automation.mode) ? automation.mode : 'off',
});

const parseJson = (value, fallback = null) => {
  if (value === null || value === undefined || value === 'null') return fallback;
  if (typeof value !== 'string') return value;
  try {
    return JSON.parse(value);
  } catch {
    return fallback;
  }
};

const toCsv = (values = []) => values.join(',');

const textureAssetsJson = (textureAssets = {}) => JSON.stringify(textureAssets || {});

const getWebGpuStatus = (rendererBackend) => ({
  available: false,
  backend: rendererBackend === 'webgpu' ? 'rust-webgl2-fallback' : 'rust-webgl2',
  reason: rendererBackend === 'webgpu'
    ? 'Rust MilkDrop migration currently renders through the Rust WebGL2/canvas backend.'
    : '',
});

export const createNativeMilkdropEngine = async ({
  audioContext,
  audioNode,
  canvas,
  rendererBackend = 'webgl2',
}) => {
  const rustModule = await loadRustMilkdropModule();
  const rustEngine = new rustModule.RustMilkdropEngine(canvas);
  const frameReader = createFrameReader(audioContext, audioNode);
  const webGpuStatus = getWebGpuStatus(rendererBackend);
  let presetIndex = 0;
  let activePresetTitle = rustEngine.loadPresetText(
    nativePresets[presetIndex].source,
    nativePresets[presetIndex].name,
    '{}',
  );
  let automation = normalizeAutomation();
  let beatState = {};
  let lastAutomatedPresetAt = 0;
  let mouseState = {
    mouse_down: 0,
    mouse_dx: 0,
    mouse_dy: 0,
    mouse_x: 0.5,
    mouse_y: 0.5,
  };

  const loadPreset = (index, options = {}) => {
    presetIndex = (index + nativePresets.length) % nativePresets.length;
    activePresetTitle = rustEngine.loadPresetText(
      nativePresets[presetIndex].source,
      nativePresets[presetIndex].name,
      textureAssetsJson(options.textureAssets),
    );
    return activePresetTitle;
  };

  const maybeAdvanceAutomatedPreset = (renderFrame, now) => {
    if (automation.mode === 'off') return null;
    if (automation.mode === 'timed') {
      if (now - lastAutomatedPresetAt < automation.timedIntervalSeconds) return null;
      lastAutomatedPresetAt = now;
      return loadPreset(presetIndex + 1);
    }

    const nextBeatState = getNativeMilkdropBeatUpdate(
      beatState,
      renderFrame.spectrum,
      now,
      automation,
    );
    beatState = nextBeatState;
    if (
      !nextBeatState.isBeat
      || nextBeatState.beatCount < automation.beatsPerPreset
      || now - lastAutomatedPresetAt < defaultTransitionSeconds
    ) {
      return null;
    }
    beatState = {
      ...nextBeatState,
      beatCount: 0,
    };
    lastAutomatedPresetAt = now;
    return loadPreset(presetIndex + 1);
  };

  return {
    name: rendererBackend === 'webgpu'
      ? 'slskr Rust MilkDrop WebGL2 fallback'
      : 'slskr Rust MilkDrop WebGL2',
    presetName: activePresetTitle,
    dispose: () => {
      frameReader.disconnect();
      rustEngine.free?.();
    },
    exportPresetFragment: (type = 'shape', index = 0) =>
      parseJson(rustEngine.exportPresetFragment(type, index)),
    exportPresetText: () => parseJson(rustEngine.exportPresetText()),
    getPresetDebugSnapshot: () => parseJson(
      rustEngine.getPresetDebugSnapshotJson(JSON.stringify(webGpuStatus)),
      {},
    ),
    getPresetFragmentSummary: () => parseJson(rustEngine.getPresetFragmentSummaryJson(), {
      shapes: [],
      waves: [],
    }),
    getPresetParameterSummary: () => parseJson(rustEngine.getPresetParameterSummaryJson(), {}),
    inspectPresetText: (source, fileName = '') =>
      parseJson(rustEngine.inspectPresetText(source, fileName), {
        title: fileName || 'Imported preset',
      }),
    loadPresetFragmentText: (source, fileName = '', options = {}) => {
      const type = options.type || (String(fileName).toLowerCase().endsWith('.wave')
        ? 'wave'
        : 'shape');
      const result = parseJson(rustEngine.loadPresetFragmentText(
        source,
        fileName,
        type,
        textureAssetsJson(options.textureAssets),
      ));
      if (result?.title) activePresetTitle = result.title;
      return result;
    },
    loadPresetText: (source, fileName = '', options = {}) => {
      activePresetTitle = rustEngine.loadPresetText(
        source,
        fileName,
        textureAssetsJson(options.textureAssets),
      );
      return activePresetTitle;
    },
    nextPreset: (options = {}) => loadPreset(presetIndex + 1, options),
    randomizePresetParameters: (options = {}) => {
      const result = parseJson(rustEngine.randomizePresetParameters(
        textureAssetsJson(options.textureAssets),
      ));
      if (result?.title) activePresetTitle = result.title;
      return result;
    },
    removePresetFragment: (type = 'shape', index = 0, options = {}) => {
      const result = parseJson(rustEngine.removePresetFragment(
        type === 'wave' ? 'wave' : 'shape',
        index,
        textureAssetsJson(options.textureAssets),
      ));
      if (result?.title) activePresetTitle = result.title;
      return result;
    },
    render: () => {
      const now = audioContext.currentTime || 0;
      const frame = frameReader.read();
      const automatedPresetName = maybeAdvanceAutomatedPreset(frame, now);
      rustEngine.render(
        now,
        frame.bands.bass,
        frame.bands.mid,
        frame.bands.treble,
        toCsv(frame.samples),
        toCsv(frame.spectrum.map((value) => value / 255)),
        mouseState.mouse_down,
        mouseState.mouse_x,
        mouseState.mouse_y,
        mouseState.mouse_dx,
        mouseState.mouse_dy,
      );
      return automatedPresetName ? { presetName: automatedPresetName } : null;
    },
    resize: (width, height) => {
      rustEngine.resize(width, height);
    },
    setMouseState: (nextMouseState = {}) => {
      mouseState = {
        ...mouseState,
        ...nextMouseState,
      };
      return mouseState;
    },
    setPresetAutomation: (nextAutomation = {}) => {
      automation = normalizeAutomation(nextAutomation);
      beatState = {};
      lastAutomatedPresetAt = audioContext.currentTime || 0;
      return automation;
    },
    updatePresetBaseValue: (key, value, options = {}) => {
      const result = parseJson(rustEngine.updatePresetBaseValue(
        key,
        Number(value),
        textureAssetsJson(options.textureAssets),
      ));
      if (result?.title) activePresetTitle = result.title;
      return result;
    },
  };
};
