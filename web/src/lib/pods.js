import { urlBase } from '../config';
import { fetchWithoutRedirects, readJsonResponse } from './http';
import * as session from './session';
import { encodePathSegment } from './pathEncoding';

const baseUrl = `${urlBase}/api/v0/pods`;
const discoveryBaseUrl = `${urlBase}/api/v0/podcore/discovery`;
const asArray = (value) => (Array.isArray(value) ? value : []);

export const list = async () => {
  const response = await fetchWithoutRedirects(baseUrl, {
    headers: session.authHeaders(),
  });

  if (!response.ok) {
    throw new Error(`Failed to list pods: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const get = async (podId) => {
  const response = await fetchWithoutRedirects(`${baseUrl}/${encodePathSegment(podId)}`, {
    headers: session.authHeaders(),
  });

  if (!response.ok) {
    throw new Error(`Failed to get pod: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const create = async (pod, requestingPeerId = 'local-peer') => {
  const response = await fetchWithoutRedirects(baseUrl, {
    body: JSON.stringify({ pod, requestingPeerId }),
    headers: {
      ...session.authHeaders({ csrf: true }),
      'Content-Type': 'application/json',
    },
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error(`Failed to create pod: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const update = async (podId, pod, requestingPeerId = 'local-peer') => {
  const response = await fetchWithoutRedirects(`${baseUrl}/${encodePathSegment(podId)}`, {
    body: JSON.stringify({ pod, requestingPeerId }),
    headers: {
      ...session.authHeaders({ csrf: true }),
      'Content-Type': 'application/json',
    },
    method: 'PUT',
  });

  if (!response.ok) {
    throw new Error(`Failed to update pod: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const getMembers = async (podId) => {
  const response = await fetchWithoutRedirects(`${baseUrl}/${encodePathSegment(podId)}/members`, {
    headers: session.authHeaders(),
  });

  if (!response.ok) {
    throw new Error(`Failed to get pod members: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const join = async (podId, peerId) => {
  const response = await fetchWithoutRedirects(`${baseUrl}/${encodePathSegment(podId)}/join`, {
    body: JSON.stringify({ peerId }),
    headers: {
      ...session.authHeaders({ csrf: true }),
      'Content-Type': 'application/json',
    },
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error(`Failed to join pod: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const leave = async (podId, peerId) => {
  const response = await fetchWithoutRedirects(`${baseUrl}/${encodePathSegment(podId)}/leave`, {
    body: JSON.stringify({ peerId }),
    headers: {
      ...session.authHeaders({ csrf: true }),
      'Content-Type': 'application/json',
    },
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error(`Failed to leave pod: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const getMessages = async (podId, channelId, since = null) => {
  const parameters = since
    ? `?since=${encodeURIComponent(String(since))}`
    : '';
  const response = await fetchWithoutRedirects(
    `${baseUrl}/${encodePathSegment(podId)}/channels/${encodePathSegment(channelId)}/messages${parameters}`,
    {
      headers: session.authHeaders(),
    },
  );

  if (!response.ok) {
    throw new Error(`Failed to get messages: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const sendMessage = async (
  podId,
  channelId,
  body,
  senderPeerId,
  signature = null,
) => {
  const response = await fetchWithoutRedirects(
    `${baseUrl}/${encodePathSegment(podId)}/channels/${encodePathSegment(channelId)}/messages`,
    {
      body: JSON.stringify({ body, senderPeerId, signature }),
      headers: {
        ...session.authHeaders({ csrf: true }),
        'Content-Type': 'application/json',
      },
      method: 'POST',
    },
  );

  if (!response.ok) {
    throw new Error(`Failed to send message: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const bindRoom = async (
  podId,
  channelId,
  roomName,
  mode = 'readonly',
) => {
  const response = await fetchWithoutRedirects(
    `${baseUrl}/${encodePathSegment(podId)}/channels/${encodePathSegment(channelId)}/bind`,
    {
      body: JSON.stringify({ mode, roomName }),
      headers: {
        ...session.authHeaders({ csrf: true }),
        'Content-Type': 'application/json',
      },
      method: 'POST',
    },
  );

  if (!response.ok) {
    throw new Error(`Failed to bind room: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

export const unbindRoom = async (podId, channelId) => {
  const response = await fetchWithoutRedirects(
    `${baseUrl}/${encodePathSegment(podId)}/channels/${encodePathSegment(channelId)}/unbind`,
    {
      headers: session.authHeaders({ csrf: true }),
      method: 'POST',
    },
  );

  if (!response.ok) {
    throw new Error(`Failed to unbind room: ${response.statusText}`);
  }

  return readJsonResponse(response);
};

const readDiscovery = async (response) => {
  if (!response.ok) {
    throw new Error(`Failed to discover pods: ${response.statusText}`);
  }

  const result = await readJsonResponse(response);
  return asArray(result?.pods ?? result?.Pods);
};

export const discoverAll = async (limit = 50) => {
  const response = await fetchWithoutRedirects(`${discoveryBaseUrl}/all?limit=${limit}`, {
    headers: session.authHeaders(),
  });

  return readDiscovery(response);
};

export const discoverByName = async (name) => {
  const response = await fetchWithoutRedirects(
    `${discoveryBaseUrl}/name/${encodeURIComponent(name)}`,
    { headers: session.authHeaders() },
  );

  return readDiscovery(response);
};

export const discoverByTag = async (tag) => {
  const response = await fetchWithoutRedirects(
    `${discoveryBaseUrl}/tag/${encodeURIComponent(tag)}`,
    { headers: session.authHeaders() },
  );

  return readDiscovery(response);
};

export const refreshDiscovery = async () => {
  const response = await fetchWithoutRedirects(`${discoveryBaseUrl}/refresh`, {
    headers: session.authHeaders({ csrf: true }),
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error(`Failed to refresh discovery: ${response.statusText}`);
  }

  return readJsonResponse(response);
};
