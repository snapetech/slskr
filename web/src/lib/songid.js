import api from './api';
import { createSongIdHubConnection } from './hubFactory';

const requireArrayResponse = (value) => {
  if (!Array.isArray(value)) {
    throw new Error('SongID API returned an invalid run list response');
  }

  return value;
};

export const createRun = async (source) => {
  const response = await api.post('/songid/runs', { source });
  return response.data;
};

export const getRun = async (id) => {
  const response = await api.get(`/songid/runs/${encodeURIComponent(id)}`);
  return response.data;
};

export const getForensicMatrix = async (id) => {
  const response = await api.get(`/songid/runs/${encodeURIComponent(id)}/forensic-matrix`);
  return response.data;
};

export const getRuns = async (limit = 10) => {
  const parameters = new URLSearchParams({ limit: String(limit) });
  const response = await api.get(`/songid/runs?${parameters.toString()}`);
  return requireArrayResponse(response.data);
};

export const createHub = () => createSongIdHubConnection();
