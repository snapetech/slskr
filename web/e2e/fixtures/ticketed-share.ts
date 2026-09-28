import { expect, type APIRequestContext } from '@playwright/test';
import type { NodeCfg } from '../env';

export const SINTEL_CONTENT_ID = 'sha256:93df4e31869e879354a5055a8549fb9335df0f5f73f4963c9bf0db2c888bb704';

/** Resolve the recipient's incoming grant and exchange its bearer token once. */
export async function incomingMovieStream({ request, recipient, recipientToken, owner, title }: {
  request: APIRequestContext;
  recipient: NodeCfg;
  recipientToken: string;
  owner: NodeCfg;
  title: string;
}): Promise<string> {
  const incoming = await request.get(`${recipient.baseUrl}/api/v0/share-grants/incoming`, {
    headers: { Authorization: `Bearer ${recipientToken}` },
  });
  expect(incoming.status()).toBe(200);
  const shares = await incoming.json();
  const share = shares.find((entry: { collectionTitle?: string }) => entry.collectionTitle === title);
  expect(share).toBeTruthy();
  expect(share.ownerEndpoint).toBe(owner.baseUrl);
  const manifest = await request.get(`${owner.baseUrl}/api/v0/share-grants/${encodeURIComponent(share.shareGrantId)}/manifest`, {
    headers: { 'X-Share-Token': share.token },
  });
  expect(manifest.status()).toBe(200);
  const body = await manifest.json();
  expect(body.items.some((item: { contentId?: string }) => item.contentId === SINTEL_CONTENT_ID)).toBe(true);
  const ticket = await request.post(`${owner.baseUrl}/api/v0/streams/${encodeURIComponent(SINTEL_CONTENT_ID)}/share-ticket`, {
    headers: { 'X-Share-Token': share.token },
  });
  expect(ticket.status()).toBe(200);
  const issued = await ticket.json();
  expect(typeof issued.ticket).toBe('string');
  const streamUrl = `${owner.baseUrl}/api/v0/streams/${encodeURIComponent(SINTEL_CONTENT_ID)}?ticket=${encodeURIComponent(issued.ticket)}`;
  expect(new URL(streamUrl).searchParams.has('token')).toBe(false);
  return streamUrl;
}
