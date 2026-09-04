import { urlBase } from '../config';
import * as session from './session';
import { encodePathSegment } from './pathEncoding';

const baseUrl = `${urlBase}/api/v0/pods`;
const discoveryBaseUrl = `${urlBase}/api/v0/podcore/discovery`;

export const list = async () => {
  const response = await fetch(baseUrl, {
    headers: session.authHeaders(),
  });

  if (!response.ok) {
    throw new Error(`Failed to list pods: ${response.statusText}`);
  }

  return response.json();
};

export const get = async (podId) => {
  const response = await fetch(`${baseUrl}/${encodePathSegment(podId)}`, {
    headers: session.authHeaders(),
  });

  if (!response.ok) {
    throw new Error(`Failed to get pod: ${response.statusText}`);
  }

  return response.json();
};

export const create = async (pod, requestingPeerId = 'local-peer') => {
  const response = await fetch(baseUrl, {
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

  return response.json();
};

export const update = async (podId, pod, requestingPeerId = 'local-peer') => {
  const response = await fetch(`${baseUrl}/${encodePathSegment(podId)}`, {
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

  return response.json();
};

export const getMembers = async (podId) => {
  const response = await fetch(`${baseUrl}/${encodePathSegment(podId)}/members`, {
    headers: session.authHeaders(),
  });

  if (!response.ok) {
    throw new Error(`Failed to get pod members: ${response.statusText}`);
  }

  return response.json();
};

export const join = async (podId, peerId) => {
  const response = await fetch(`${baseUrl}/${encodePathSegment(podId)}/join`, {
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

  return response.json();
};

export const leave = async (podId, peerId) => {
  const response = await fetch(`${baseUrl}/${encodePathSegment(podId)}/leave`, {
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

  return response.json();
};

export const getMessages = async (podId, channelId, since = null) => {
  const parameters = since
    ? `?since=${encodeURIComponent(String(since))}`
    : '';
  const response = await fetch(
    `${baseUrl}/${encodePathSegment(podId)}/channels/${encodePathSegment(channelId)}/messages${parameters}`,
    {
      headers: session.authHeaders(),
    },
  );

  if (!response.ok) {
    throw new Error(`Failed to get messages: ${response.statusText}`);
  }

  return response.json();
};

export const sendMessage = async (
  podId,
  channelId,
  body,
  senderPeerId,
  signature = null,
) => {
  const response = await fetch(
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

  return response.json();
};

export const bindRoom = async (
  podId,
  channelId,
  roomName,
  mode = 'readonly',
) => {
  const response = await fetch(
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

  return response.json();
};

export const unbindRoom = async (podId, channelId) => {
  const response = await fetch(
    `${baseUrl}/${encodePathSegment(podId)}/channels/${encodePathSegment(channelId)}/unbind`,
    {
      headers: session.authHeaders({ csrf: true }),
      method: 'POST',
    },
  );

  if (!response.ok) {
    throw new Error(`Failed to unbind room: ${response.statusText}`);
  }

  return response.json();
};

const readDiscovery = async (response) => {
  if (!response.ok) {
    throw new Error(`Failed to discover pods: ${response.statusText}`);
  }

  const result = await response.json();
  return result?.pods || result?.Pods || [];
};

export const discoverAll = async (limit = 50) => {
  const response = await fetch(`${discoveryBaseUrl}/all?limit=${limit}`, {
    headers: session.authHeaders(),
  });

  return readDiscovery(response);
};

export const discoverByName = async (name) => {
  const response = await fetch(
    `${discoveryBaseUrl}/name/${encodeURIComponent(name)}`,
    { headers: session.authHeaders() },
  );

  return readDiscovery(response);
};

export const discoverByTag = async (tag) => {
  const response = await fetch(
    `${discoveryBaseUrl}/tag/${encodeURIComponent(tag)}`,
    { headers: session.authHeaders() },
  );

  return readDiscovery(response);
};

export const refreshDiscovery = async () => {
  const response = await fetch(`${discoveryBaseUrl}/refresh`, {
    headers: session.authHeaders({ csrf: true }),
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error(`Failed to refresh discovery: ${response.statusText}`);
  }

  return response.json();
};
