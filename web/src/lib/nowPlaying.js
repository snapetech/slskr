import api from './api';

export const getNowPlaying = async () => {
  const response = await api.get('/nowplaying');
  if (response.status === 204 || response.data == null || response.data === '') {
    return null;
  }
  if (
    typeof response.data !== 'object' ||
    Array.isArray(response.data)
  ) {
    throw new Error('Now-playing API returned an invalid state response');
  }
  return response.data;
};

export const setNowPlaying = async ({ artist, title, album } = {}) => {
  return api.put('/nowplaying', { artist, title, album });
};

export const clearNowPlaying = async () => {
  return api.delete('/nowplaying');
};
