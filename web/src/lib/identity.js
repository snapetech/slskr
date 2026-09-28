// Identity & Friends API client

import api from './api';
import { encodePathSegment } from './pathEncoding';

// Profile API
export const getMyProfile = () => api.get('/profile/me');
export const updateMyProfile = (data) => api.put('/profile/me', data);
export const getProfile = (peerId) =>
  api.get(`/profile/${encodeURIComponent(peerId)}`);
export const createInvite = (data) => api.post('/profile/invite', data);

// Contacts API
export const getContacts = () => api.get('/contacts');
export const getContact = (id) =>
  api.get(`/contacts/${encodePathSegment(id)}`);
export function inviteContactUsername(inviteLink) {
  if (typeof inviteLink !== 'string' || inviteLink.length > 65536) {
    throw new Error('Invalid invite link');
  }
  const match = /^slskdn:\/\/invite\/([A-Za-z0-9_-]+)$/u.exec(inviteLink);
  if (!match) throw new Error('Invalid invite link');
  let invite;
  try {
    const bytes = Uint8Array.from(atob(match[1].replaceAll('-', '+').replaceAll('_', '/')), (c) => c.charCodeAt(0));
    invite = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    throw new Error('Invalid invite payload');
  }
  const username = invite?.Profile?.DisplayName;
  const expiry = Date.parse(invite?.ExpiresAt);
  if (invite?.InviteVersion !== 1 || typeof username !== 'string'
      || !username.trim() || username.length > 255 || /[\u0000-\u001f\u007f]/u.test(username)
      || !Number.isFinite(expiry) || expiry <= Date.now()) {
    throw new Error('Invalid or expired invite');
  }
  // An invite supplies an address-book username; it grants no authenticated
  // identity, verified status, transport credentials, or share permission.
  return username;
}

export const addContactFromInvite = (data) =>
  api.post('/contacts/from-invite', {
    ...data,
    username: inviteContactUsername(data?.inviteLink),
  });
export const addContactFromDiscovery = (data) =>
  api.post('/contacts/from-discovery', data);
export const updateContact = (id, data) =>
  api.put(`/contacts/${encodePathSegment(id)}`, data);
export const deleteContact = (id) =>
  api.delete(`/contacts/${encodePathSegment(id)}`);
export const getNearby = () => api.get('/contacts/nearby');
