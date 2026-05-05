import api from './api';

export const getSpotifyStatus = async () =>
  (await api.get('/integrations/spotify/status')).data;

export const startSpotifyAuthorization = async () => {
  const data = (await api.post('/integrations/spotify/authorize')).data;
  if (data?.authorization_url) {
    window.location.assign(data.authorization_url);
  }
  return data;
};

export const disconnectSpotify = async () => {
  await api.delete('/integrations/spotify');
};
