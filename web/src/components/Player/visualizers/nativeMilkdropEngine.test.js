import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  createNativeMilkdropEngine,
  getNativeMilkdropBeatUpdate,
  getNativeMilkdropTransitionAlphas,
  getNativeMilkdropTransitionProgress,
} from './nativeMilkdropEngine';

const createAnalyser = () => ({
  fftSize: 0,
  frequencyBinCount: 4,
  getByteFrequencyData: vi.fn((data) => {
    data.set([0, 128, 255, 64]);
  }),
  getByteTimeDomainData: vi.fn((data) => {
    data.set([0, 128, 255, 128]);
  }),
});

const createRustEngineMock = () => ({
  exportPresetFragment: vi.fn((type) =>
    JSON.stringify({ fileName: `active.${type}`, source: `[${type}]\nenabled=1\n` })),
  exportPresetText: vi.fn(() =>
    JSON.stringify({ fileName: 'active.milk', source: 'name=Active\nzoom=1\n' })),
  free: vi.fn(),
  getPresetDebugSnapshotJson: vi.fn(() =>
    JSON.stringify({ renderer: 'Rust WebGL2 renderer active', title: 'Active' })),
  getPresetFragmentSummaryJson: vi.fn(() =>
    JSON.stringify({ shapes: [{ index: 0, label: 'Shape 1' }], waves: [] })),
  getPresetParameterSummaryJson: vi.fn(() => JSON.stringify({ decay: 0.91, zoom: 1 })),
  inspectPresetText: vi.fn((_source, fileName) =>
    JSON.stringify({ title: fileName.replace(/\.milk2?$/, '') || 'Imported preset' })),
  loadPresetFragmentText: vi.fn((_source, fileName) =>
    JSON.stringify({
      source: `name=Active\n; merged ${fileName}`,
      title: `Active + ${fileName}`,
    })),
  loadPresetText: vi.fn((_source, fileName) =>
    fileName?.replace(/\.milk2?$/, '') || 'Imported preset'),
  randomizePresetParameters: vi.fn(() =>
    JSON.stringify({ source: 'name=Random', title: 'Random', values: { zoom: 1.2 } })),
  removePresetFragment: vi.fn((type) =>
    JSON.stringify({ source: `name=Active\n; removed ${type}`, title: 'Active edited' })),
  render: vi.fn(),
  resize: vi.fn(),
  updatePresetBaseValue: vi.fn((key, value) =>
    JSON.stringify({
      source: `name=Active\n${key}=${value}`,
      title: 'Active edited',
      values: { [key]: value },
    })),
});

describe('createNativeMilkdropEngine', () => {
  let rustEngine;

  beforeEach(() => {
    rustEngine = createRustEngineMock();
    class RustMilkdropEngineMock {
      constructor() {
        return rustEngine;
      }
    }
    globalThis.__slskrRustMilkdropModule = {
      RustMilkdropEngine: RustMilkdropEngineMock,
    };
  });

  it('eases native transition progress between renderer sets', () => {
    expect(getNativeMilkdropTransitionProgress(10, 2, 10)).toBe(0);
    expect(getNativeMilkdropTransitionProgress(10, 2, 11)).toBe(0.5);
    expect(getNativeMilkdropTransitionProgress(10, 2, 12)).toBe(1);
    expect(getNativeMilkdropTransitionProgress(10, 0, 10)).toBe(1);
  });

  it('maps native transition modes to incoming and outgoing alphas', () => {
    expect(getNativeMilkdropTransitionAlphas(0.25, 'crossfade')).toEqual({
      incoming: 0.25,
      outgoing: 0.75,
    });
    expect(getNativeMilkdropTransitionAlphas(0.25, 'fade_through_black')).toEqual({
      incoming: 0,
      outgoing: 0.5,
    });
    expect(getNativeMilkdropTransitionAlphas(0.5, 'overlay')).toEqual({
      incoming: 0.5,
      outgoing: 1,
    });
    expect(getNativeMilkdropTransitionAlphas(0.5, 'cut')).toEqual({
      incoming: 1,
      outgoing: 0,
    });
  });

  it('detects beat pulses from low-frequency spectrum energy', () => {
    const baseline = getNativeMilkdropBeatUpdate(
      {},
      [16, 18, 17, 19],
      1,
      { beatSensitivity: 1.35, minBeatIntervalSeconds: 0.25 },
    );
    const pulse = getNativeMilkdropBeatUpdate(
      baseline,
      [240, 230, 220, 210],
      1.4,
      { beatSensitivity: 1.35, minBeatIntervalSeconds: 0.25 },
    );

    expect(baseline.isBeat).toBe(false);
    expect(pulse.isBeat).toBe(true);
    expect(pulse.beatCount).toBe(1);
  });

  it('feeds waveform, spectrum, and mouse state into the Rust renderer', async () => {
    const analyser = createAnalyser();
    const audioNode = {
      connect: vi.fn(),
      disconnect: vi.fn(),
    };
    const engine = await createNativeMilkdropEngine({
      audioContext: {
        createAnalyser: () => analyser,
        currentTime: 12,
      },
      audioNode,
      canvas: { getContext: vi.fn() },
    });

    engine.setMouseState({
      mouse_down: 1,
      mouse_dx: 0.2,
      mouse_dy: -0.1,
      mouse_x: 0.75,
      mouse_y: 0.25,
    });
    engine.render();
    engine.resize(320, 180);

    expect(engine.name).toBe('slskr Rust MilkDrop WebGL2');
    expect(audioNode.connect).toHaveBeenCalledWith(analyser);
    expect(rustEngine.render).toHaveBeenCalledWith(
      12,
      expect.any(Number),
      expect.any(Number),
      expect.any(Number),
      expect.stringContaining('-1'),
      expect.any(String),
      1,
      0.75,
      0.25,
      0.2,
      -0.1,
    );
    expect(rustEngine.resize).toHaveBeenCalledWith(320, 180);
  });

  it('loads, edits, exports, and disposes through the Rust WASM boundary', async () => {
    const analyser = createAnalyser();
    const audioNode = {
      connect: vi.fn(),
      disconnect: vi.fn(),
    };
    const engine = await createNativeMilkdropEngine({
      audioContext: {
        createAnalyser: () => analyser,
        currentTime: 0,
      },
      audioNode,
      canvas: { getContext: vi.fn() },
      rendererBackend: 'webgpu',
    });

    expect(engine.name).toBe('slskr Rust MilkDrop WebGL2 fallback');
    expect(engine.loadPresetText('name=Imported', 'imported.milk')).toBe('imported');
    expect(engine.inspectPresetText('name=Imported', 'imported.milk')).toEqual({
      title: 'imported',
    });
    expect(engine.loadPresetFragmentText('enabled=1', 'shape.shape')).toEqual({
      source: 'name=Active\n; merged shape.shape',
      title: 'Active + shape.shape',
    });
    expect(engine.updatePresetBaseValue('zoom', 1.2)).toEqual({
      source: 'name=Active\nzoom=1.2',
      title: 'Active edited',
      values: { zoom: 1.2 },
    });
    expect(engine.randomizePresetParameters()).toEqual({
      source: 'name=Random',
      title: 'Random',
      values: { zoom: 1.2 },
    });
    expect(engine.removePresetFragment('shape', 0)).toEqual({
      source: 'name=Active\n; removed shape',
      title: 'Active edited',
    });
    expect(engine.exportPresetText()).toEqual({
      fileName: 'active.milk',
      source: 'name=Active\nzoom=1\n',
    });
    expect(engine.exportPresetFragment('shape', 0)).toEqual({
      fileName: 'active.shape',
      source: '[shape]\nenabled=1\n',
    });
    expect(engine.getPresetParameterSummary()).toEqual({ decay: 0.91, zoom: 1 });
    expect(engine.getPresetFragmentSummary()).toEqual({
      shapes: [{ index: 0, label: 'Shape 1' }],
      waves: [],
    });
    expect(engine.getPresetDebugSnapshot()).toEqual({
      renderer: 'Rust WebGL2 renderer active',
      title: 'Active',
    });

    engine.dispose();

    expect(audioNode.disconnect).toHaveBeenCalledWith(analyser);
    expect(rustEngine.free).toHaveBeenCalled();
  });
});
