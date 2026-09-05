import api from './api';

export const getState = async () => {
  return (await api.get('/application')).data;
};

export const restart = async () => {
  return api.put('/application');
};

export const shutdown = async () => {
  return api.delete('/application');
};

export const getVersion = async ({ forceCheck = false }) => {
  const parameters = new URLSearchParams({ forceCheck: String(forceCheck) });
  return (await api.get(`/application/version/latest?${parameters}`)).data;
};

export const getBuild = async ({ checkForUpdates = true } = {}) => {
  const parameters = new URLSearchParams({
    checkForUpdates: String(checkForUpdates),
  });
  return (await api.get(`/application/build?${parameters}`)).data;
};
