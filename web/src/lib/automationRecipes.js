import {
  getLocalStorageItem,
  setLocalStorageItem,
} from './storage';
import {
  maxPersistedJsonCharacters,
  readBoundedJson,
  writeBoundedObject,
} from './persistedJson';

const storageKey = 'slskr.automationRecipeState';
const inputStorageKey = 'slskr.automationRecipeInputs';
const maxAutomationStorageCharacters = 128 * 1024;
const maxAutomationTextCharacters = 2_048;
const maxAutomationInputEntries = 16;
const executableRecipeIds = new Set([
  'dashboard-refresh',
  'library-health-scan',
  'local-diagnostics',
  'stale-cache-reminders',
  'wishlist-retry',
]);

export const automationRecipes = [
  {
    approvalGate: 'None required',
    cadence: 'Continuous',
    cooldown: '5 minutes',
    description: 'Checks connection, shares, paths, and credentials for setup drift.',
    enabledByDefault: true,
    fileImpact: 'Read only',
    icon: 'stethoscope',
    id: 'local-diagnostics',
    maxRunTime: '30 seconds',
    networkImpact: 'Local',
    title: 'Local Diagnostics',
  },
  {
    approvalGate: 'None required',
    cadence: 'Daily',
    cooldown: '24 hours',
    description: 'Surfaces stale share-cache and library-scan reminders before users hit missing results.',
    enabledByDefault: true,
    fileImpact: 'Read only',
    icon: 'bell outline',
    id: 'stale-cache-reminders',
    maxRunTime: '1 minute',
    networkImpact: 'Local',
    title: 'Share and Library Reminders',
  },
  {
    approvalGate: 'None required',
    cadence: 'Every 15 minutes',
    cooldown: '15 minutes',
    description: 'Keeps local dashboard summaries fresh without contacting public peers.',
    enabledByDefault: true,
    fileImpact: 'Read only',
    icon: 'dashboard',
    id: 'dashboard-refresh',
    maxRunTime: '20 seconds',
    networkImpact: 'Local',
    title: 'Dashboard Refresh',
  },
  {
    approvalGate: 'Download approval',
    cadence: 'Manual or scheduled',
    cooldown: '2 hours',
    description: 'Retries failed Wishlist items using the selected acquisition profile.',
    enabledByDefault: false,
    fileImpact: 'Downloads after approval',
    icon: 'redo',
    id: 'wishlist-retry',
    maxRunTime: '20 minutes',
    networkImpact: 'Public peers possible',
    title: 'Wishlist Retry',
  },
  {
    approvalGate: 'Fix confirmation',
    cadence: 'Manual or scheduled',
    cooldown: '24 hours',
    description: 'Finds duplicates, dead files, metadata gaps, fake lossless files, and missing art.',
    enabledByDefault: false,
    fileImpact: 'Read only until fixed',
    icon: 'heartbeat',
    id: 'library-health-scan',
    maxRunTime: '30 minutes',
    networkImpact: 'Local',
    title: 'Library Health Scan',
  },
  {
    approvalGate: 'Configured import success',
    cadence: 'After import',
    cooldown: '10 minutes',
    description: 'Asks configured media servers to rescan after successful library imports.',
    enabledByDefault: false,
    fileImpact: 'Media-server scan',
    icon: 'server',
    id: 'media-server-rescan',
    maxRunTime: '2 minutes',
    networkImpact: 'Local network',
    title: 'Media Server Rescan',
  },
  {
    approvalGate: 'Explicit evidence publication opt-in',
    cadence: 'Manual or scheduled',
    cooldown: '12 hours',
    description: 'Publishes explicit opt-in signed quality and verification evidence to trusted mesh peers.',
    enabledByDefault: false,
    fileImpact: 'No file writes',
    icon: 'share alternate',
    id: 'mesh-evidence-publish',
    maxRunTime: '10 minutes',
    networkImpact: 'Trusted mesh',
    title: 'Mesh Evidence Publish',
  },
];

const defaultState = automationRecipes.reduce((state, recipe) => {
  state[recipe.id] = {
    enabled: recipe.enabledByDefault,
    lastDryRunAt: null,
  };
  return state;
}, {});

const isPlainObject = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

const normalizeText = (value, fallback = '') =>
  typeof value === 'string' || typeof value === 'number'
    ? String(value).trim().slice(0, maxAutomationTextCharacters)
    : fallback;

const normalizeCount = (value) => {
  const count = Number(value);
  return Number.isFinite(count) ? Math.max(0, Math.floor(count)) : 0;
};

const normalizeTimestamp = (value) => normalizeText(value) || null;

const normalizeReport = (report) => {
  if (!isPlainObject(report)) return null;

  const normalized = {
    approvalGate: normalizeText(report.approvalGate),
    cooldown: normalizeText(report.cooldown),
    executed: report.executed === true,
    failed: normalizeCount(report.failed),
    fileImpact: normalizeText(report.fileImpact),
    generatedAt: normalizeText(report.generatedAt),
    maxRunTime: normalizeText(report.maxRunTime),
    networkImpact: normalizeText(report.networkImpact),
    recipeId: normalizeText(report.recipeId),
    runLimit: normalizeCount(report.runLimit),
    skipped: normalizeCount(report.skipped),
    started: normalizeCount(report.started),
    summary: normalizeText(report.summary),
    title: normalizeText(report.title),
  };
  const scanId = normalizeText(report.scanId);
  if (scanId) normalized.scanId = scanId;
  return normalized;
};

const normalizeRecipeState = (recipe, state = {}) => {
  const source = isPlainObject(state) ? state : {};
  return {
    enabled:
      typeof source.enabled === 'boolean'
        ? source.enabled
        : recipe.enabledByDefault,
    lastDryRunAt: normalizeTimestamp(source.lastDryRunAt),
    lastDryRunReport: normalizeReport(source.lastDryRunReport),
    lastRunAt: normalizeTimestamp(source.lastRunAt),
    lastRunReport: normalizeReport(source.lastRunReport),
  };
};

const readStoredState = () => {
  const parsed = readBoundedJson(
    getLocalStorageItem,
    storageKey,
    {},
    Math.min(maxPersistedJsonCharacters, maxAutomationStorageCharacters),
  );
  if (!isPlainObject(parsed)) return {};

  return automationRecipes.reduce((state, recipe) => {
    state[recipe.id] = normalizeRecipeState(recipe, parsed[recipe.id]);
    return state;
  }, {});
};

const readStoredInputs = () => {
  const parsed = readBoundedJson(
    getLocalStorageItem,
    inputStorageKey,
    {},
    Math.min(maxPersistedJsonCharacters, maxAutomationStorageCharacters),
  );
  if (!isPlainObject(parsed)) return {};

  return automationRecipes.reduce((inputs, recipe) => {
    const source = isPlainObject(parsed[recipe.id]) ? parsed[recipe.id] : {};
    const libraryPath = normalizeText(source.libraryPath);
    if (libraryPath) inputs[recipe.id] = { libraryPath };
    return inputs;
  }, {});
};

const writeRecipeState = (state) =>
  writeBoundedObject(
    setLocalStorageItem,
    storageKey,
    automationRecipes.reduce((normalized, recipe) => {
      normalized[recipe.id] = normalizeRecipeState(recipe, state[recipe.id]);
      return normalized;
    }, {}),
    {
      maxCharacters: maxAutomationStorageCharacters,
      maxEntries: automationRecipes.length,
    },
  );

const writeRecipeInputs = (inputs) =>
  writeBoundedObject(
    setLocalStorageItem,
    inputStorageKey,
    automationRecipes.reduce((normalized, recipe) => {
      const libraryPath = normalizeText(inputs[recipe.id]?.libraryPath);
      if (libraryPath) normalized[recipe.id] = { libraryPath };
      return normalized;
    }, {}),
    {
      maxCharacters: maxAutomationStorageCharacters,
      maxEntries: maxAutomationInputEntries,
    },
  );

export const buildAutomationDryRunReport = (
  recipe,
  timestamp = new Date().toISOString(),
) => ({
  approvalGate: recipe.approvalGate,
  cooldown: recipe.cooldown,
  executed: false,
  fileImpact: recipe.fileImpact,
  generatedAt: timestamp,
  maxRunTime: recipe.maxRunTime,
  networkImpact: recipe.networkImpact,
  recipeId: recipe.id,
  title: recipe.title,
});

export const isAutomationRecipeExecutable = (recipe) =>
  executableRecipeIds.has(recipe?.id);

export const buildAutomationExecutionReport = (
  recipe,
  result = {},
  timestamp = new Date().toISOString(),
) => ({
  approvalGate: recipe.approvalGate,
  cooldown: recipe.cooldown,
  executed: true,
  failed: result.failed || 0,
  fileImpact: recipe.fileImpact,
  generatedAt: timestamp,
  maxRunTime: recipe.maxRunTime,
  networkImpact: recipe.networkImpact,
  recipeId: recipe.id,
  runLimit: result.runLimit || 0,
  ...(normalizeText(result.scanId) ? { scanId: normalizeText(result.scanId) } : {}),
  skipped: result.skipped || 0,
  started: result.started || 0,
  summary:
    normalizeText(result.summary) ||
    `Started ${result.started || 0} action(s); ${result.failed || 0} failed; ${result.skipped || 0} skipped.`,
  title: recipe.title,
});

export const buildAutomationRunHistory = (state = getAutomationRecipeState()) =>
  automationRecipes
    .map((recipe) => ({
      enabled: state[recipe.id]?.enabled === true,
      lastDryRunAt: state[recipe.id]?.lastDryRunAt || null,
      lastDryRunReport: state[recipe.id]?.lastDryRunReport || null,
      lastRunAt: state[recipe.id]?.lastRunAt || null,
      lastRunReport: state[recipe.id]?.lastRunReport || null,
      recipeId: recipe.id,
      title: recipe.title,
    }))
    .filter((entry) => entry.enabled || entry.lastDryRunAt || entry.lastRunAt);

export const formatAutomationRunHistoryReport = (
  history = buildAutomationRunHistory(),
) => {
  const lines = [
    'slskr automation review history',
    `Entries: ${history.length}`,
    '',
  ];

  if (history.length === 0) {
    lines.push('No enabled recipes or dry-run checkpoints.');
    return lines.join('\n');
  }

  history.forEach((entry) => {
    lines.push(`- ${entry.title}`);
    lines.push(`  Enabled: ${entry.enabled ? 'yes' : 'no'}`);
    lines.push(`  Last run: ${entry.lastRunAt || 'not recorded'}`);
    if (entry.lastRunReport) {
      lines.push(`  Run summary: ${entry.lastRunReport.summary}`);
      lines.push(`  Network impact: ${entry.lastRunReport.networkImpact}`);
      lines.push(`  File impact: ${entry.lastRunReport.fileImpact}`);
    }
    lines.push(`  Last dry run: ${entry.lastDryRunAt || 'not recorded'}`);
    if (entry.lastDryRunReport) {
      lines.push(`  Executed: ${entry.lastDryRunReport.executed ? 'yes' : 'no'}`);
      lines.push(`  Network impact: ${entry.lastDryRunReport.networkImpact}`);
      lines.push(`  File impact: ${entry.lastDryRunReport.fileImpact}`);
    }
  });

  return lines.join('\n');
};

export const getAutomationRecipeState = () => ({
  ...defaultState,
  ...readStoredState(),
});

export const getAutomationRecipeInputs = () => readStoredInputs();

export const setAutomationRecipeInput = (id, input) => {
  if (!automationRecipes.some((recipe) => recipe.id === id)) {
    return getAutomationRecipeInputs();
  }

  const inputs = getAutomationRecipeInputs();
  const nextInputs = {
    ...inputs,
    [id]: {
      ...(inputs[id] || {}),
      libraryPath: normalizeText(input?.libraryPath),
    },
  };

  return writeRecipeInputs(nextInputs);
};

export const setAutomationRecipeEnabled = (id, enabled) => {
  if (!automationRecipes.some((recipe) => recipe.id === id)) {
    return getAutomationRecipeState();
  }

  const state = getAutomationRecipeState();
  const recipeState = state[id] ?? {};
  const nextState = {
    ...state,
    [id]: {
      ...recipeState,
      enabled,
    },
  };

  return writeRecipeState(nextState);
};

export const setAutomationRecipeDryRun = (id, timestamp = new Date().toISOString()) => {
  const state = getAutomationRecipeState();
  const recipeState = state[id] ?? {};
  const recipe = automationRecipes.find((item) => item.id === id);
  if (!recipe) return state;
  const normalizedTimestamp = normalizeText(timestamp);
  const nextState = {
    ...state,
    [id]: {
      ...recipeState,
      lastDryRunAt: normalizedTimestamp,
      lastDryRunReport: recipe
        ? buildAutomationDryRunReport(recipe, normalizedTimestamp)
        : null,
    },
  };

  return writeRecipeState(nextState);
};

export const setAutomationRecipeExecution = (
  id,
  report,
  timestamp = new Date().toISOString(),
) => {
  const state = getAutomationRecipeState();
  const recipeState = state[id] ?? {};
  if (!automationRecipes.some((recipe) => recipe.id === id)) return state;
  const normalizedTimestamp = normalizeText(timestamp);
  const nextState = {
    ...state,
    [id]: {
      ...recipeState,
      lastRunAt: normalizedTimestamp,
      lastRunReport: normalizeReport(report),
    },
  };

  return writeRecipeState(nextState);
};

export const automationRecipeStorageKey = storageKey;
export const automationRecipeInputStorageKey = inputStorageKey;
