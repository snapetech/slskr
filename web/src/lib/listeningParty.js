import api from './api';
import { rootUrl } from '../config';

const requireObjectResponse = (value, resource) => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`Listening party API returned an invalid ${resource} response`);
  }

  return value;
};

const requireArrayResponse = (value, resource) => {
  if (!Array.isArray(value)) {
    throw new Error(`Listening party API returned an invalid ${resource} response`);
  }

  return value;
};

export const getPartyDirectory = async () => {
  const { data } = await api.get('/listening-party');
  return requireArrayResponse(data, 'party directory');
};

export const getPartyState = async (podId, channelId) => {
  const response = await api.get(
    `/listening-party/${encodeURIComponent(podId)}/${encodeURIComponent(channelId)}`,
    { validateStatus: (status) => status === 200 || status === 204 },
  );

  if (response.status === 204) return null;
  return requireObjectResponse(response.data, 'party state');
};

export const publishPartyState = async (podId, channelId, event) => {
  return (
    await api.post(
      `/listening-party/${encodeURIComponent(podId)}/${encodeURIComponent(channelId)}`,
      event,
    )
  ).data;
};

export const buildRadioStreamUrl = (party) => {
  if (!party?.streamPath) return null;
  return `${rootUrl}${party.streamPath}`;
};
