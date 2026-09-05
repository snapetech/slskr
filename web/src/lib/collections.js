// Collections & ShareGroups API client

import api from './api';
import { fetchWithoutRedirects, readJsonResponse } from './http';
import { encodePathSegment } from './pathEncoding';

const requireArrayResponse = (response, resource) => {
  if (!Array.isArray(response?.data)) {
    throw new Error(`Collections API returned an invalid ${resource} response`);
  }

  return response;
};

const requireObjectResponse = (response, resource) => {
  if (
    !response?.data ||
    typeof response.data !== 'object' ||
    Array.isArray(response.data)
  ) {
    throw new Error(`Collections API returned an invalid ${resource} response`);
  }

  return response;
};

const requireLibrarySearchResponse = (response, resource) => {
  if (
    !response?.data ||
    typeof response.data !== 'object' ||
    Array.isArray(response.data) ||
    !Array.isArray(response.data.items)
  ) {
    throw new Error(`Collections API returned an invalid ${resource} response`);
  }

  return response;
};

const requireLibraryBrowserResponse = (response) => {
  const data = response?.data;
  if (
    !data ||
    typeof data !== 'object' ||
    Array.isArray(data) ||
    !Array.isArray(data.breadcrumbs) ||
    !Array.isArray(data.directories) ||
    !Array.isArray(data.files) ||
    typeof data.hasMore !== 'boolean'
  ) {
    throw new Error(
      'Collections API returned an invalid library browser response',
    );
  }

  return response;
};

// ShareGroups
export const getShareGroups = async () =>
  requireArrayResponse(await api.get('/sharegroups'), 'share groups');
export const getShareGroup = async (id) =>
  requireObjectResponse(
    await api.get(`/sharegroups/${encodePathSegment(id)}`),
    'share group',
  );
export const createShareGroup = (data) => api.post('/sharegroups', data);
export const updateShareGroup = (id, data) =>
  api.put(`/sharegroups/${encodePathSegment(id)}`, data);
export const deleteShareGroup = (id) =>
  api.delete(`/sharegroups/${encodePathSegment(id)}`);
export const getShareGroupMembers = async (id, detailed = false) =>
  requireArrayResponse(
    await api.get(
      `/sharegroups/${encodePathSegment(id)}/members${detailed ? '?detailed=true' : ''}`,
    ),
    'share group members',
  );
export const addShareGroupMember = (id, data) =>
  api.post(`/sharegroups/${encodePathSegment(id)}/members`, data);
export const removeShareGroupMember = (id, userId) =>
  api.delete(
    `/sharegroups/${encodePathSegment(id)}/members/${encodePathSegment(userId)}`,
  );

// Collections
export const getCollections = async () =>
  requireArrayResponse(await api.get('/collections'), 'collections');
export const getCollection = async (id) =>
  requireObjectResponse(
    await api.get(`/collections/${encodePathSegment(id)}`),
    'collection',
  );
export const createCollection = (data) => api.post('/collections', data);
export const updateCollection = (id, data) =>
  api.put(`/collections/${encodePathSegment(id)}`, data);
export const deleteCollection = (id) =>
  api.delete(`/collections/${encodePathSegment(id)}`);
export const getCollectionItems = async (id) =>
  requireArrayResponse(
    await api.get(`/collections/${encodePathSegment(id)}/items`),
    'collection items',
  );
export const addCollectionItem = (id, data) =>
  api.post(`/collections/${encodePathSegment(id)}/items`, data);
export const updateCollectionItem = (itemId, data) =>
  api.put(`/collections/items/${encodePathSegment(itemId)}`, data);
export const removeCollectionItem = (itemId) =>
  api.delete(`/collections/items/${encodePathSegment(itemId)}`);
export const reorderCollectionItems = (id, itemIds) =>
  api.put(`/collections/${encodePathSegment(id)}/items/reorder`, { itemIds });

// Share Grants (Shares)
export const getShares = async () =>
  requireArrayResponse(await api.get('/share-grants'), 'shares');
// Shares announced to this node by another node's owner (see
// /api/v0/share-grants/announce) — distinct from getShares(), which lists
// grants this node itself owns.
export const getIncomingShares = async () =>
  requireArrayResponse(await api.get('/share-grants/incoming'), 'incoming shares');
export const getShare = async (id) =>
  requireObjectResponse(
    await api.get(`/share-grants/${encodePathSegment(id)}`),
    'share',
  );
export const getSharesByCollection = async (collectionId) =>
  requireArrayResponse(
    await api.get(`/share-grants/by-collection/${encodeURIComponent(collectionId)}`),
    'collection shares',
  );
export const createShare = (data) => api.post('/share-grants', data);
export const updateShare = (id, data) =>
  api.put(`/share-grants/${encodePathSegment(id)}`, data);
export const deleteShare = (id) =>
  api.delete(`/share-grants/${encodePathSegment(id)}`);
export const createShareToken = (id, expiresInSeconds) =>
  api.post(`/share-grants/${encodeURIComponent(id)}/token`, { expiresInSeconds });
export const getShareManifest = (id, token) => {
  const url = `/share-grants/${encodeURIComponent(id)}/manifest`;
  return token
    ? api.get(url, { headers: { 'X-Share-Token': token } })
    : api.get(url);
};

export const backfillShare = (id) =>
  api.post(`/share-grants/${encodePathSegment(id)}/backfill`);

// The backend stores grant permissions as a single comma/whitespace-separated
// string (e.g. "download,stream"), not boolean fields.
export const shareGrantAllows = (permissions, token) =>
  String(permissions || '')
    .split(/[,\s]+/)
    .some((candidate) => candidate.toLowerCase() === token);

// A share announced to this node lives on another node entirely — the
// grant, its manifest, and its stream tickets all belong to the owner's own
// API, reachable only via its ownerEndpoint and the share token issued for
// it. The shared axios client always injects this node's own session
// Bearer token and reloads the app on any 401, neither of which applies to
// a cross-node share-token request, so these go through plain fetch.
const remoteShareUrl = (ownerEndpoint, path) => {
  let base;
  try {
    base = new URL(ownerEndpoint);
  } catch {
    throw new Error('Remote share owner endpoint is invalid');
  }

  if (
    !['http:', 'https:'].includes(base.protocol) ||
    base.username ||
    base.password
  ) {
    throw new Error('Remote share owner endpoint must be an HTTP(S) URL without credentials');
  }

  const prefix = base.pathname.replace(/\/+$/u, '');
  return new URL(`${prefix}${path}`, base.origin).toString();
};

const shareFetch = async (ownerEndpoint, path, { method = 'GET', token, body } = {}) => {
  const response = await fetchWithoutRedirects(remoteShareUrl(ownerEndpoint, path), {
    body: body === undefined ? undefined : JSON.stringify(body),
    headers: {
      ...(token ? { 'X-Share-Token': token } : {}),
      ...(body === undefined ? {} : { 'Content-Type': 'application/json' }),
    },
    method,
    redirect: 'error',
  });
  let data = null;
  try {
    data = await readJsonResponse(response);
  } catch (error) {
    if (response.ok || String(error?.message || '').includes('exceeds')) {
      if (!response.ok) throw error;
      throw new Error('Remote share endpoint returned invalid JSON', { cause: error });
    }
  }
  if (!response.ok) {
    const error = new Error(data?.error || `Request failed: ${response.status}`);
    error.response = { data, status: response.status };
    throw error;
  }
  return data;
};

export const fetchRemoteShareManifest = (ownerEndpoint, shareGrantId, token) =>
  shareFetch(ownerEndpoint, `/api/v0/share-grants/${encodeURIComponent(shareGrantId)}/manifest`, {
    token,
  });

export const remoteBackfillShare = (ownerEndpoint, shareGrantId, token) =>
  shareFetch(ownerEndpoint, `/api/v0/share-grants/${encodeURIComponent(shareGrantId)}/backfill`, {
    body: {},
    method: 'POST',
    token,
  });

export const createRemoteShareStreamTicket = (ownerEndpoint, contentId, token) =>
  shareFetch(ownerEndpoint, `/api/v0/streams/${encodeURIComponent(contentId)}/share-ticket`, {
    method: 'POST',
    token,
  }).then((data) => data?.ticket);

export const buildRemoteShareStreamUrl = (ownerEndpoint, contentId, ticket) =>
  `${remoteShareUrl(ownerEndpoint, `/api/v0/streams/${encodeURIComponent(contentId)}`)}?ticket=${encodeURIComponent(ticket)}`;

// Library Items (for Collections picker)
// Note: api baseURL already includes /api/v0, so use relative path
export const searchLibraryItems = async (query, kinds, limit = 100) => {
  const parameters = new URLSearchParams();
  if (query) parameters.append('query', query);
  if (kinds) parameters.append('kinds', kinds);
  parameters.append('limit', limit.toString());
  return requireLibrarySearchResponse(
    await api.get(`library/items?${parameters.toString()}`),
    'library search',
  );
};

export const browseLibraryItems = async ({
  kinds = 'Audio',
  limit = 100,
  offset = 0,
  path = '',
  query = '',
} = {}) => {
  const parameters = new URLSearchParams();
  if (path) parameters.append('path', path);
  if (query) parameters.append('query', query);
  if (kinds) parameters.append('kinds', kinds);
  parameters.append('limit', limit.toString());
  parameters.append('offset', offset.toString());
  return requireLibraryBrowserResponse(
    await api.get(`library/items/browser?${parameters.toString()}`),
  );
};

export const getLibraryItem = async (contentId) =>
  requireObjectResponse(
    await api.get(`library/items/${encodeURIComponent(contentId)}`),
    'library item',
  );
