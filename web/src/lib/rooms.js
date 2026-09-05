import api from './api';

const requireArrayResponse = (value, resource) => {
  if (!Array.isArray(value)) {
    throw new Error(`Rooms API returned an invalid ${resource} response`);
  }

  return value;
};

const requireObjectResponse = (value, resource) => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`Rooms API returned an invalid ${resource} response`);
  }

  return value;
};

export const getAvailable = async () => {
  const response = (await api.get('/rooms/available')).data;

  return requireArrayResponse(response, 'available rooms');
};

export const getJoined = async () => {
  const response = (await api.get('/rooms/joined')).data;

  return requireArrayResponse(response, 'joined rooms');
};

export const getActivity = async () => {
  const response = (await api.get('/rooms/activity')).data;

  requireObjectResponse(response, 'room activity');

  return Object.fromEntries(
    Object.entries(response)
      .map(([roomName, timestamp]) => [roomName, Number(timestamp)])
      .filter(
        ([roomName, timestamp]) =>
          roomName.length > 0 && Number.isFinite(timestamp) && timestamp > 0,
      ),
  );
};

export const getMessages = async ({ roomName, since = null }) => {
  const query =
    since == null ? '' : `?since=${encodeURIComponent(String(since))}`;
  const response = (
    await api.get(
      `/rooms/joined/${encodeURIComponent(roomName)}/messages${query}`,
    )
  ).data;

  return requireArrayResponse(response, 'room messages');
};

export const getUsers = async ({ roomName }) => {
  const response = (
    await api.get(`/rooms/joined/${encodeURIComponent(roomName)}/users`)
  ).data;

  return requireArrayResponse(response, 'room users');
};

export const join = async ({ roomName }) => {
  return api.post('/rooms/joined', JSON.stringify(roomName));
};

export const leave = async ({ roomName }) => {
  return api.delete(`/rooms/joined/${encodeURIComponent(roomName)}`);
};

export const sendMessage = async ({ roomName, message }) => {
  return api.post(
    `/rooms/joined/${encodeURIComponent(roomName)}/messages`,
    JSON.stringify(message),
  );
};

export const setTicker = async ({ roomName, message }) => {
  return api.post(
    `/rooms/joined/${encodeURIComponent(roomName)}/ticker`,
    JSON.stringify(message),
  );
};

export const addRoomMember = async ({ roomName, username }) => {
  return api.post(
    `/rooms/joined/${encodeURIComponent(roomName)}/members`,
    JSON.stringify(username),
  );
};
