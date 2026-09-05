import api from './api';

const requireArrayResponse = (value) => {
  if (!Array.isArray(value)) {
    throw new Error('Download requests API returned an invalid request list response');
  }

  return value;
};

export const list = async ({ state } = {}) => {
  const params = state ? `?state=${encodeURIComponent(state)}` : '';
  const { data } = await api.get(`/downloads/requests${params}`);
  return requireArrayResponse(data);
};

export const get = async (id) => {
  const { data } = await api.get(`/downloads/requests/${encodeURIComponent(id)}`);
  return data;
};

export const rename = async (id, name) => {
  const { data } = await api.patch(`/downloads/requests/${encodeURIComponent(id)}/name`, { name });
  return data;
};

export const cancel = async (id) => {
  await api.post(`/downloads/requests/${encodeURIComponent(id)}/cancel`);
};
