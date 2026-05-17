import api from './api';

export const getState = async () => {
  return (await api.get('/server')).data;
};

export const connect = (credentials) => {
  return api.put('/server', credentials || {});
};

export const disconnect = ({
  message = 'client disconnected from web UI',
} = {}) => {
  return api.delete('/server', { data: JSON.stringify(message) });
};
