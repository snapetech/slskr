import api from './api';

const requireRecord = (data) => {
  if (!data || typeof data !== 'object' || Array.isArray(data)) {
    throw new Error('Server API returned an invalid state response');
  }
  return data;
};

export const getState = async () => {
  return requireRecord((await api.get('/server')).data);
};

export const connect = (credentials) => {
  return api.put('/server', credentials || {});
};

export const disconnect = ({
  message = 'client disconnected from web UI',
} = {}) => {
  return api.delete('/server', { data: JSON.stringify(message) });
};
