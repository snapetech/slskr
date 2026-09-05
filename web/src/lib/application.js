import api from './api';

const requireRecord = (data, resource) => {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    throw new Error(`Application API returned an invalid ${resource} response`);
  }
  return data;
};

export const getState = async () => {
  return requireRecord((await api.get('/application')).data, 'state');
};

export const restart = async () => {
  return api.put('/application');
};

export const shutdown = async () => {
  return api.delete('/application');
};

export const getVersion = async ({ forceCheck = false } = {}) => {
  const parameters = new URLSearchParams({ forceCheck: String(forceCheck) });
  return requireRecord(
    (await api.get(`/application/version/latest?${parameters}`)).data,
    'version',
  );
};

export const getBuild = async ({ checkForUpdates = true } = {}) => {
  const parameters = new URLSearchParams({
    checkForUpdates: String(checkForUpdates),
  });
  return requireRecord(
    (await api.get(`/application/build?${parameters}`)).data,
    'build',
  );
};
