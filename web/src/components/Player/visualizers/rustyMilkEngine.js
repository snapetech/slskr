import {
  createRustyMilkEngine as createRustyMilkWebEngine,
  getRustyMilkBeatUpdate,
  getRustyMilkTransitionAlphas,
  getRustyMilkTransitionProgress,
  loadRustyMilkPack,
  loadRustyMilkPackManifest,
  loadRustyMilkPackPresetSource,
  normalizeRustyMilkPackManifest,
  validateRustyMilkPackManifest,
} from '@rustymilk/web';

export {
  getRustyMilkBeatUpdate,
  getRustyMilkTransitionAlphas,
  getRustyMilkTransitionProgress,
  loadRustyMilkPack,
  loadRustyMilkPackManifest,
  loadRustyMilkPackPresetSource,
  normalizeRustyMilkPackManifest,
  validateRustyMilkPackManifest,
};

export const createRustyMilkEngine = async ({
  audioContext,
  audioNode,
  canvas,
  modulePath = '/slskr_web.js',
  rendererBackend = 'webgl2',
}) => {
  const engine = await createRustyMilkWebEngine({
    audioContext,
    audioNode,
    canvas,
    modulePath,
    rendererBackend,
  });

  return {
    ...engine,
    name: rendererBackend === 'webgpu'
      ? 'slskr RustyMilk WebGL2 fallback'
      : 'slskr RustyMilk WebGL2',
  };
};
