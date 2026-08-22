import * as applicationAPI from './application';
import * as libraryHealthAPI from './libraryHealth';
import * as sharesAPI from './shares';
import * as slskrAPI from './slskr';

const summarizeSettledChecks = (label, checks) => {
  const failed = checks.filter((check) => check.status === 'rejected').length;
  const completed = checks.length - failed;

  return {
    failed,
    started: 1,
    summary: `${label} completed ${completed}/${checks.length} read-only checks; ${failed} failed.`,
  };
};

const readOnlySnapshotChecks = () => [
  applicationAPI.getState(),
  sharesAPI.getAll(),
  slskrAPI.getSlskrStats(),
];

export const executeLocalDiagnostics = async () => {
  const checks = await Promise.allSettled(readOnlySnapshotChecks());
  return summarizeSettledChecks('Local diagnostics', checks);
};

export const executeStaleCacheReminders = async () => {
  const checks = await Promise.allSettled([
    sharesAPI.getAll(),
    libraryHealthAPI.getIssues({ limit: 100 }),
  ]);
  return summarizeSettledChecks('Share and library reminders', checks);
};

export const executeDashboardRefresh = async () => {
  const checks = await Promise.allSettled(readOnlySnapshotChecks());
  return summarizeSettledChecks('Dashboard refresh', checks);
};

export const automationActions = {
  'dashboard-refresh': executeDashboardRefresh,
  'local-diagnostics': executeLocalDiagnostics,
  'stale-cache-reminders': executeStaleCacheReminders,
};

export const executeAutomationAction = async (recipeId) => {
  const action = automationActions[recipeId];
  if (!action) {
    throw new Error(`No automation action is available for ${recipeId}.`);
  }

  return action();
};
