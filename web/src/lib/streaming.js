import api from './api';
import { apiBaseUrl, urlBase } from '../config';

const requireTicket = (data, label) => {
  if (
    !data ||
    typeof data !== 'object' ||
    Array.isArray(data) ||
    typeof data.ticket !== 'string' ||
    data.ticket.trim() === ''
  ) {
    throw new Error(`Streaming API returned an invalid ${label} response`);
  }
  return data.ticket;
};

const requirePeerTicket = (data) => {
  if (
    !data ||
    typeof data !== 'object' ||
    Array.isArray(data) ||
    typeof data.ticket !== 'string' ||
    data.ticket.trim() === '' ||
    typeof data.streamUrl !== 'string' ||
    data.streamUrl.trim() === ''
  ) {
    throw new Error('Streaming API returned an invalid peer ticket response');
  }
  return data;
};

export const createStreamTicket = async (contentId) => {
  const response = await api.post(
    `/streams/${encodeURIComponent(contentId)}/ticket`,
  );
  return requireTicket(response.data, 'stream ticket');
};

export const buildTicketedStreamUrl = (contentId, ticket) =>
  `${apiBaseUrl}/streams/${encodeURIComponent(contentId)}?ticket=${encodeURIComponent(ticket)}`;

// Exchanges a share token for a short-lived, content-bound stream ticket. The share token is sent in
// the X-Share-Token header (never the URL) so it stays out of browser history, proxy logs, and access
// logs; the returned opaque ticket is safe to place in the stream URL.
export const createShareStreamTicket = async (contentId, shareToken) => {
  const response = await api.post(
    `/streams/${encodeURIComponent(contentId)}/share-ticket`,
    undefined,
    { headers: { 'X-Share-Token': shareToken } },
  );
  return requireTicket(response.data, 'share stream ticket');
};

export const buildDirectStreamUrl = (contentId) =>
  `${apiBaseUrl}/streams/${encodeURIComponent(contentId)}`;

export const createPeerStreamTicket = async ({ username, filename, size }) => {
  const response = await api.post('/peer-streams/tickets', {
    username,
    filename,
    size,
  });
  return requirePeerTicket(response.data);
};

export const buildPeerStreamUrl = (streamUrl) => {
  if (!streamUrl) return '';
  if (/^https?:\/\//i.test(streamUrl)) return streamUrl;
  return `${urlBase}${streamUrl.startsWith('/') ? streamUrl : `/${streamUrl}`}`;
};
