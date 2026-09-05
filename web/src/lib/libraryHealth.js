import api from './api';

export const startScan = (libraryPath) =>
  api.post('/library/health/scans', {
    includeSubdirectories: true,
    libraryPath,
  });

export const getScanStatus = (scanId) =>
  api.get(`/library/health/scans/${encodeURIComponent(scanId)}`);

export const getSummary = (libraryPath) =>
  api.get(
    `/library/health/summary?libraryPath=${encodeURIComponent(libraryPath)}`,
  );

export const getDashboard = (libraryPath, artistLimit = 10, issueLimit = 100) => {
  const parameters = new URLSearchParams({
    artistLimit: String(artistLimit),
    issueLimit: String(issueLimit),
    libraryPath,
  });
  return api.get(`/library/health/dashboard?${parameters.toString()}`);
};

export const getIssues = (filter = {}) => {
  const parameters = new URLSearchParams();
  if (filter.libraryPath) parameters.append('libraryPath', filter.libraryPath);
  if (filter.limit !== undefined && filter.limit !== null) {
    parameters.append('limit', String(filter.limit));
  }
  if (filter.offset !== undefined && filter.offset !== null) {
    parameters.append('offset', String(filter.offset));
  }
  return api.get(`/library/health/issues?${parameters.toString()}`);
};

export const getIssuesByType = (libraryPath = null) => {
  const parameters = libraryPath
    ? `?libraryPath=${encodeURIComponent(libraryPath)}`
    : '';
  return api.get(`/library/health/issues/by-type${parameters}`);
};

export const getIssuesByArtist = (limit = 20) => {
  const parameters = new URLSearchParams({ limit: String(limit) });
  return api.get(`/library/health/issues/by-artist?${parameters}`);
};

export const getIssuesByRelease = (limit = 20) => {
  const parameters = new URLSearchParams({ limit: String(limit) });
  return api.get(`/library/health/issues/by-release?${parameters}`);
};

export const updateIssueStatus = (issueId, status) =>
  api.patch(`/library/health/issues/${encodeURIComponent(issueId)}`, { status });

export const createRemediationJob = (issueIds) =>
  api.post(`/library/health/issues/fix`, { issueIds });

export default {
  createRemediationJob,
  getDashboard,
  getIssues,
  getIssuesByArtist,
  getIssuesByRelease,
  getIssuesByType,
  getScanStatus,
  getSummary,
  startScan,
  updateIssueStatus,
};
