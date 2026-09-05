import api from './api';

export const list = async ({ offset = 0, limit = 50, kind, topic, q } = {}) => {
  const parameters = new URLSearchParams({
    offset: String(offset),
    limit: String(limit),
  });

  for (const [name, value] of Object.entries({ kind, topic, q })) {
    if (typeof value === 'string' && value.trim()) {
      parameters.set(name, value.trim());
    }
  }

  const response = await api.get(`/events?${parameters.toString()}`);

  const events = Array.isArray(response.data) ? response.data : [];
  const totalCount = response.headers['x-total-count'];

  return { events, totalCount };
};

export const raiseEvent = async ({ type, disambiguator = '' }) => {
  return api.post(
    `/events/${encodeURIComponent(type)}`,
    JSON.stringify(disambiguator),
  );
};
