import api from './api';

const defaultLogLevels = ['Trace', 'Debug', 'Information', 'Warning', 'Error'];

export const getCurrent = async () => {
  return (await api.get('/options')).data;
};

export const applyOverlay = async (overlay) => {
  return (await api.patch('/options', overlay)).data;
};

export const getCurrentDebugView = async () => {
  return (await api.get('/options/debug')).data;
};

export const getLogs = async () => {
  const logs = (await api.get('/logs')).data;
  if (!Array.isArray(logs)) {
    return logs;
  }

  let levelInfo;
  try {
    levelInfo = (await api.get('/logs/level')).data || {};
  } catch {
    levelInfo = null;
  }

  return {
    entries: logs,
    level: levelInfo?.level || 'Information',
    levels: levelInfo?.levels || defaultLogLevels,
    limit: logs.length,
  };
};

export const updateLogLevel = async (level) => {
  return (await api.put('/logs/level', { level })).data;
};

export const getYaml = async () => {
  return (await api.get('/options/yaml')).data;
};

export const getYamlLocation = async () => {
  return (await api.get('/options/yaml/location')).data;
};

export const validateYaml = async ({ yaml }) => {
  return (await api.post('/options/yaml/validate', yaml)).data;
};

export const updateYaml = async ({ yaml }) => {
  return (await api.put('/options/yaml', yaml)).data;
};
