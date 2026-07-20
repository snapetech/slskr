import api from './api';

export const getInfo = ({ quietUnavailable = false, username }) => {
  const query = quietUnavailable ? '?quietUnavailable=true' : '';
  return api.get(`/users/${encodeURIComponent(username)}/info${query}`);
};

export const getStatus = ({ username }) => {
  return api.get(`/users/${encodeURIComponent(username)}/status`);
};

export const getEndpoint = ({ username }) => {
  return api.get(`/users/${encodeURIComponent(username)}/endpoint`);
};

export const browse = async ({ username }) => {
  const encodedUsername = encodeURIComponent(username);
  const statusPath = `/users/${encodedUsername}/browse/status`;
  const browsePath = `/users/${encodedUsername}/browse`;
  const current = (await api.get(statusPath)).data;

  if (current?.isComplete) {
    return (await api.get(browsePath)).data;
  }

  await api.post(`/users/${encodedUsername}/browse/request`);

  for (let attempt = 0; attempt < 240; attempt += 1) {
    const status = (await api.get(statusPath)).data;
    const state = String(status?.state || '').toLowerCase();
    if (status?.isComplete) {
      return (await api.get(browsePath)).data;
    }
    if (state === 'failed' || state === 'cancelled') {
      throw new Error(status?.reason || 'The remote peer is unavailable');
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error('Browse timed out while waiting for the remote peer');
};

export const getBrowseStatus = ({ username }) => {
  return api.get(`/users/${encodeURIComponent(username)}/browse/status`);
};

export const getDirectoryContents = async ({ username, directory }) => {
  return (
    await api.post(`/users/${encodeURIComponent(username)}/directory`, {
      directory,
    })
  ).data;
};

export const getGroup = ({ username }) => {
  return api.get(`/users/${encodeURIComponent(username)}/group`);
};

export const getGroups = async ({ usernames }) => {
  const params = new URLSearchParams();
  for (const username of usernames) {
    params.append('usernames', username);
  }
  const response = (await api.get(`/users/groups?${params.toString()}`)).data;
  return response && typeof response === 'object' && !Array.isArray(response)
    ? response
    : {};
};
