import api from './api';

const requireRecord = (data, resource) => {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    throw new Error(`Transfer reports API returned an invalid ${resource} response`);
  }
  return data;
};

const requireArray = (data, resource) => {
  if (!Array.isArray(data)) {
    throw new Error(`Transfer reports API returned an invalid ${resource} response`);
  }
  return data;
};

export const getSummary = async ({
  start,
  end,
  direction,
  username = null,
} = {}) => {
  const parameters = new URLSearchParams();

  if (start) parameters.append('start', start.toISOString());
  if (end) parameters.append('end', end.toISOString());
  if (direction) parameters.append('direction', direction);
  if (username) parameters.append('username', username);

  return requireRecord(
    (await api.get(`/telemetry/reports/transfers/summary?${parameters}`)).data,
    'summary',
  );
};

export const getHistogram = async ({
  start,
  end,
  buckets,
  direction,
  interval,
  username = null,
} = {}) => {
  const parameters = new URLSearchParams();

  if (start) parameters.append('start', start.toISOString());
  if (end) parameters.append('end', end.toISOString());

  let histogramInterval = Number(interval);
  if (!Number.isFinite(histogramInterval) || histogramInterval < 5) {
    const startTime = start?.getTime();
    const endTime = end?.getTime();
    const requestedBuckets = Number(buckets);

    if (
      Number.isFinite(startTime) &&
      Number.isFinite(endTime) &&
      endTime > startTime &&
      Number.isFinite(requestedBuckets) &&
      requestedBuckets > 0
    ) {
      histogramInterval = Math.ceil(
        ((endTime - startTime) / 60_000) / requestedBuckets,
      );
    }
  }

  if (Number.isFinite(histogramInterval)) {
    parameters.append('interval', Math.max(5, Math.ceil(histogramInterval)));
  }
  if (direction) parameters.append('direction', direction);
  if (username) parameters.append('username', username);

  return requireRecord(
    (await api.get(`/telemetry/reports/transfers/histogram?${parameters}`)).data,
    'histogram',
  );
};

export const getLeaderboard = async ({
  direction,
  start,
  end,
  sortBy = 'Count',
  sortOrder = 'DESC',
  limit = 10,
  offset = 0,
} = {}) => {
  const parameters = new URLSearchParams();

  if (direction) parameters.append('direction', direction);
  if (start) parameters.append('start', start.toISOString());
  if (end) parameters.append('end', end.toISOString());

  parameters.append('sortBy', sortBy);
  parameters.append('sortOrder', sortOrder);
  parameters.append('limit', limit);
  parameters.append('offset', offset);

  return requireArray(
    (
      await api.get(`/telemetry/reports/transfers/leaderboard?${parameters}`)
    ).data,
    'leaderboard',
  );
};

export const getTopDirectories = async ({
  start,
  end,
  username = null,
  limit = 10,
  offset = 0,
} = {}) => {
  const parameters = new URLSearchParams();

  if (start) parameters.append('start', start.toISOString());
  if (end) parameters.append('end', end.toISOString());
  if (username) parameters.append('username', username);

  parameters.append('limit', limit);
  parameters.append('offset', offset);

  return requireArray(
    (
      await api.get(`/telemetry/reports/transfers/directories?${parameters}`)
    ).data,
    'directory',
  );
};

export const getExceptions = async ({
  direction,
  start,
  end,
  username,
  sortOrder = 'DESC',
  limit = 10,
  offset = 0,
} = {}) => {
  const parameters = new URLSearchParams();

  if (direction) parameters.append('direction', direction);
  if (start) parameters.append('start', start.toISOString());
  if (end) parameters.append('end', end.toISOString());
  if (username) parameters.append('username', username);

  parameters.append('sortOrder', sortOrder);
  parameters.append('limit', limit);
  parameters.append('offset', offset);

  return requireArray(
    (
      await api.get(`/telemetry/reports/transfers/exceptions?${parameters}`)
    ).data,
    'exception',
  );
};

export const getExceptionPareto = async ({
  direction,
  start,
  end,
  username = null,
  limit = 10,
  offset = 0,
} = {}) => {
  const parameters = new URLSearchParams();

  if (direction) parameters.append('direction', direction);
  if (start) parameters.append('start', start.toISOString());
  if (end) parameters.append('end', end.toISOString());
  if (username) parameters.append('username', username);

  parameters.append('limit', limit);
  parameters.append('offset', offset);

  return requireArray(
    (
      await api.get(
        `/telemetry/reports/transfers/exceptions/pareto?${parameters}`,
      )
    ).data,
    'exception pareto',
  );
};
