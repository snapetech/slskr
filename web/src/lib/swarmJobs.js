// <copyright file="swarmJobs.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

const MAX_SWARM_JOB_RECORDS = 512;

const numberOr = (value, fallback = 0) => {
  const number = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(number) ? number : fallback;
};

const firstNumber = (...values) => {
  for (const value of values) {
    if (value !== undefined && value !== null) {
      const number = Number(value);
      if (Number.isFinite(number)) return number;
    }
  }

  return 0;
};

/**
 * Normalize the native, legacy, and transfer projections returned by the
 * multisource jobs endpoint into the fields consumed by the web UI.
 *
 * @param {object} job - Raw daemon job projection.
 * @returns {object | null} Normalized job or null for an invalid record.
 */
export const normalizeSwarmJob = (job) => {
  if (!job || typeof job !== 'object' || Array.isArray(job)) return null;

  const jobId = job.jobId ?? job.id;
  if (jobId === undefined || jobId === null || String(jobId).trim() === '') {
    return null;
  }

  const completedChunks = firstNumber(job.completedChunks);
  const totalChunks = firstNumber(job.totalChunks);
  const rawProgress =
    job.progressPercent ?? job.percentComplete ?? job.progress;
  const progressPercent =
    rawProgress === undefined || rawProgress === null
      ? totalChunks > 0
        ? (completedChunks * 100) / totalChunks
        : 0
      : numberOr(rawProgress);
  const sources = Array.isArray(job.sources) ? job.sources : [];

  return {
    ...job,
    activeSources: firstNumber(
      job.activeSources,
      job.activeWorkers,
      sources.length,
    ),
    downloadedBytes: firstNumber(
      job.downloadedBytes,
      job.bytesDownloaded,
      job.bytesTransferred,
    ),
    jobId,
    progressPercent: Math.min(100, Math.max(0, progressPercent)),
    totalBytes: firstNumber(job.totalBytes, job.fileSize, job.size),
  };
};

/**
 * Accept either the native `{ jobs, count }` envelope or a raw array while
 * bounding and filtering records before they reach React rendering code.
 *
 * @param {object | Array} value - Raw endpoint response.
 * @returns {Array<object>} Normalized jobs.
 */
export const normalizeSwarmJobList = (value) => {
  const jobs = Array.isArray(value)
    ? value
    : value && typeof value === 'object' && Array.isArray(value.jobs)
      ? value.jobs
      : [];

  return jobs
    .slice(0, MAX_SWARM_JOB_RECORDS)
    .map(normalizeSwarmJob)
    .filter((job) => job !== null);
};
