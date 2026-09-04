const audioGraphCache = new WeakMap();

const eqBands = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

const toFiniteNumber = (value, fallback = 0) => {
  const number = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(number) ? number : fallback;
};

const clamp = (value, minimum, maximum, fallback = 0) =>
  Math.max(minimum, Math.min(maximum, toFiniteNumber(value, fallback)));

const isAudioElement = (audioElement) =>
  audioElement !== null &&
  (typeof audioElement === 'object' || typeof audioElement === 'function');

const closeAudioContext = (ctx) => {
  try {
    const closeResult = ctx?.close?.();
    if (closeResult && typeof closeResult.catch === 'function') {
      void closeResult.catch(() => {});
    }
  } catch {
    // The context may have failed before close became available.
  }
};

const disconnect = (node) => {
  try {
    node.disconnect();
  } catch {
    // Nodes may already be disconnected when the graph is rebuilt.
  }
};

const rebuildGraph = (graph) => {
  const {
    analyser,
    ctx,
    eq,
    inputGain,
    outputGain,
    source,
    visualizerInput,
  } = graph;

  disconnect(source);
  disconnect(inputGain);
  graph.karaokeNodes.forEach(disconnect);
  eq.forEach(disconnect);
  disconnect(analyser);
  disconnect(outputGain);

  source.connect(inputGain);
  let tail = inputGain;

  if (graph.karaokeEnabled) {
    const splitter = ctx.createChannelSplitter(2);
    const leftGain = ctx.createGain();
    const rightGain = ctx.createGain();
    const merger = ctx.createChannelMerger(2);

    rightGain.gain.value = -1;
    tail.connect(splitter);
    splitter.connect(leftGain, 0);
    splitter.connect(rightGain, 1);
    leftGain.connect(merger, 0, 0);
    rightGain.connect(merger, 0, 1);

    graph.karaokeNodes = [splitter, leftGain, rightGain, merger];
    tail = merger;
  } else {
    graph.karaokeNodes = [];
  }

  eq.forEach((filter) => {
    tail.connect(filter);
    tail = filter;
  });

  tail.connect(visualizerInput);
  tail.connect(analyser);
  analyser.connect(outputGain);
  outputGain.connect(ctx.destination);
};

export const getOrCreateAudioGraph = (audioElement) => {
  if (!isAudioElement(audioElement) || typeof window === 'undefined') return null;
  const cached = audioGraphCache.get(audioElement);
  if (cached) return cached;

  const AudioCtx = window.AudioContext || window.webkitAudioContext;
  if (!AudioCtx) return null;

  let ctx = null;
  try {
    ctx = new AudioCtx();
    const source = ctx.createMediaElementSource(audioElement);
    const inputGain = ctx.createGain();
    const outputGain = ctx.createGain();
    const visualizerInput = ctx.createGain();
    const visualizerOutput = ctx.createGain();
    const analyser = ctx.createAnalyser();
    const eq = eqBands.map((frequency) => {
      const filter = ctx.createBiquadFilter();
      filter.type = 'peaking';
      filter.frequency.value = frequency;
      filter.Q.value = 1.15;
      filter.gain.value = 0;
      return filter;
    });

    analyser.fftSize = 2048;
    outputGain.gain.value = 1;
    visualizerOutput.gain.value = 0;
    visualizerInput.connect(visualizerOutput);
    visualizerOutput.connect(ctx.destination);

    const graph = {
      analyser,
      ctx,
      eq,
      inputGain,
      karaokeEnabled: false,
      karaokeNodes: [],
      outputGain,
      source,
      visualizerInput,
      visualizerOutput,
    };

    rebuildGraph(graph);
    audioGraphCache.set(audioElement, graph);
    return graph;
  } catch {
    // Browsers reject a second MediaElementSource for the same element and
    // can reject graph construction while audio permissions are changing.
    // Leave the native element usable as the fallback path.
    closeAudioContext(ctx);
    return null;
  }
};

export const resumeAudioGraph = async (audioElement) => {
  const graph = getOrCreateAudioGraph(audioElement);
  if (!graph || graph.ctx.state === 'closed') return null;
  if (graph?.ctx.state === 'suspended') {
    await graph.ctx.resume();
  }
  return graph.ctx.state === 'closed' ? null : graph;
};

export const setEqGains = (audioElement, gains) => {
  const graph = getOrCreateAudioGraph(audioElement);
  if (!graph || graph.ctx.state === 'closed') return;
  const values = Array.isArray(gains) ? gains : [];
  graph.eq.forEach((filter, index) => {
    filter.gain.value = clamp(values[index], -40, 40);
  });
};

export const setKaraokeEnabled = (audioElement, enabled) => {
  const graph = getOrCreateAudioGraph(audioElement);
  const nextEnabled = enabled === true;
  if (!graph || graph.ctx.state === 'closed' || graph.karaokeEnabled === nextEnabled) return;
  const previousEnabled = graph.karaokeEnabled;
  graph.karaokeEnabled = nextEnabled;
  try {
    rebuildGraph(graph);
  } catch {
    graph.karaokeEnabled = previousEnabled;
    try {
      rebuildGraph(graph);
    } catch {
      // The browser is tearing down the context; native element playback can
      // continue without the enhanced graph.
      closeAudioContext(graph.ctx);
    }
  }
};

export const setOutputGain = (audioElement, value) => {
  const graph = getOrCreateAudioGraph(audioElement);
  const nextGain = clamp(value, 0, 1);
  if (!graph) {
    if (isAudioElement(audioElement)) audioElement.volume = nextGain;
    return;
  }
  if (graph.ctx.state === 'closed') {
    audioElement.volume = nextGain;
    return;
  }
  graph.outputGain.gain.cancelScheduledValues(graph.ctx.currentTime);
  graph.outputGain.gain.setValueAtTime(nextGain, graph.ctx.currentTime);
};

export const fadeOutputGain = (audioElement, from, to, durationSeconds) => {
  const graph = getOrCreateAudioGraph(audioElement);
  const startGain = clamp(from, 0, 1);
  const endGain = clamp(to, 0, 1);
  const duration = Math.max(0, toFiniteNumber(durationSeconds));
  if (!graph) {
    if (isAudioElement(audioElement)) audioElement.volume = endGain;
    return;
  }
  if (graph.ctx.state === 'closed') {
    audioElement.volume = endGain;
    return;
  }

  const now = graph.ctx.currentTime;
  graph.outputGain.gain.cancelScheduledValues(now);
  graph.outputGain.gain.setValueAtTime(startGain, now);
  if (duration === 0) {
    graph.outputGain.gain.setValueAtTime(endGain, now);
    return;
  }
  graph.outputGain.gain.linearRampToValueAtTime(endGain, now + duration);
};

export const bands = eqBands;
