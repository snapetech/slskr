import api from './api';

export const getMetrics = async () => {
  const data = (
    await api.get('/telemetry/metrics', { headers: { Accept: 'application/json' } })
  ).data;
  if (typeof data !== 'string') {
    throw new Error('Telemetry API returned an invalid metrics response');
  }
  return data;
};

export const getKpiMetrics = async () => {
  const data = (await api.get('/telemetry/metrics/kpi')).data;
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    throw new Error('Telemetry API returned an invalid KPI response');
  }
  return data;
};
