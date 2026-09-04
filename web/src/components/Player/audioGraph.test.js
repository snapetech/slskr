import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  fadeOutputGain,
  getOrCreateAudioGraph,
  setEqGains,
  setOutputGain,
} from './audioGraph';

const createParam = () => ({
  cancelScheduledValues: vi.fn(),
  linearRampToValueAtTime: vi.fn(),
  setValueAtTime: vi.fn(),
  value: 0,
});

const createNode = () => ({
  connect: vi.fn(),
  disconnect: vi.fn(),
  gain: createParam(),
});

class FakeAudioContext {
  constructor() {
    this.currentTime = 12;
    this.destination = {};
    this.state = 'running';
  }

  createAnalyser() {
    return {
      connect: vi.fn(),
      disconnect: vi.fn(),
      fftSize: 0,
      frequencyBinCount: 32,
    };
  }

  createBiquadFilter() {
    return {
      connect: vi.fn(),
      disconnect: vi.fn(),
      frequency: { value: 0 },
      gain: { value: 0 },
      Q: { value: 0 },
    };
  }

  createChannelMerger() {
    return createNode();
  }

  createChannelSplitter() {
    return createNode();
  }

  createGain() {
    return createNode();
  }

  createMediaElementSource() {
    return createNode();
  }

  close() {
    this.state = 'closed';
    return Promise.resolve();
  }

  resume() {
    this.state = 'running';
    return Promise.resolve();
  }
}

describe('audioGraph', () => {
  beforeEach(() => {
    window.AudioContext = FakeAudioContext;
    window.webkitAudioContext = undefined;
  });

  it('normalizes malformed equalizer and output values at the Web Audio boundary', () => {
    const audioElement = { volume: 0.5 };
    const graph = getOrCreateAudioGraph(audioElement);

    setEqGains(audioElement, [Infinity, '3', {}, -50]);
    setOutputGain(audioElement, 2);
    fadeOutputGain(audioElement, -1, Number.NaN, Number.NaN);

    expect(graph.eq[0].gain.value).toBe(0);
    expect(graph.eq[1].gain.value).toBe(3);
    expect(graph.eq[2].gain.value).toBe(0);
    expect(graph.eq[3].gain.value).toBe(-40);
    expect(graph.outputGain.gain.setValueAtTime).toHaveBeenLastCalledWith(0, 12);
    expect(graph.outputGain.gain.linearRampToValueAtTime).not.toHaveBeenCalled();
  });

  it('falls back to native volume when the cached context has closed', () => {
    const audioElement = { volume: 0.5 };
    const graph = getOrCreateAudioGraph(audioElement);
    graph.ctx.state = 'closed';

    setOutputGain(audioElement, 2);
    fadeOutputGain(audioElement, 0, 0.25, 1);

    expect(audioElement.volume).toBe(0.25);
  });

  it('returns no graph when browser graph construction fails', () => {
    class FailingAudioContext extends FakeAudioContext {
      createMediaElementSource() {
        throw new Error('MediaElementSource unavailable');
      }
    }
    window.AudioContext = FailingAudioContext;

    const audioElement = { volume: 0.5 };
    expect(getOrCreateAudioGraph(audioElement)).toBeNull();
    expect(() => setOutputGain(audioElement, 0.75)).not.toThrow();
    expect(audioElement.volume).toBe(0.75);
  });
});
