import api from './api';

const isRecord = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

export const getAll = async () => {
  const data = (await api.get('/destinations')).data;
  if (!Array.isArray(data) || data.some((destination) => !isRecord(destination))) {
    throw new Error('Destinations API returned an invalid list response');
  }
  return data;
};

export const getDefault = async () => {
  const data = (await api.get('/destinations/default')).data;
  if (!isRecord(data)) {
    throw new Error('Destinations API returned an invalid default response');
  }
  return data;
};

export const validate = async (path) => {
  const data = (await api.post('/destinations/validate', { path })).data;
  if (!isRecord(data)) {
    throw new Error('Destinations API returned an invalid validation response');
  }
  return data;
};
