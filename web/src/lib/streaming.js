import api from './api';
import { urlBase } from '../config';

export const createStreamTicket = async (contentId) => {
  const response = await api.post(
    `/streams/${encodeURIComponent(contentId)}/ticket`,
  );
  return response.data?.ticket || '';
};

export const buildTicketedStreamUrl = (contentId, ticket) =>
  `${urlBase}/api/v0/streams/${encodeURIComponent(contentId)}?ticket=${encodeURIComponent(ticket)}`;

export const createShareStreamTicket = async (contentId, shareToken) => {
  const response = await api.post(
    `/streams/${encodeURIComponent(contentId)}/share-ticket`,
    undefined,
    { headers: { 'X-Share-Token': shareToken } },
  );
  return response.data?.ticket || '';
};

export const buildDirectStreamUrl = (contentId) =>
  `${urlBase}/api/v0/streams/${encodeURIComponent(contentId)}`;
