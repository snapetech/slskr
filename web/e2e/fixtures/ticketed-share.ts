import { expect, type APIRequestContext } from '@playwright/test';
import type { NodeCfg } from '../env';
import { connect } from 'node:net';
import { connect as connectTls } from 'node:tls';

/** Pause a real response socket so its large movie body cannot drain. */
export async function holdMovieStream(streamUrl: string): Promise<{ status: number; close: () => Promise<void> }> {
  const url = new URL(streamUrl);
  if (url.username || url.password || !['http:', 'https:'].includes(url.protocol) || !url.searchParams.has('ticket') || url.searchParams.has('token')) {
    throw new Error('Expected a ticketed HTTP movie stream');
  }
  const port = Number(url.port || (url.protocol === 'https:' ? 443 : 80));
  const socket = url.protocol === 'https:'
    ? connectTls({ host: url.hostname, port, servername: url.hostname })
    : connect({ host: url.hostname, port });
  const close = async () => {
    if (socket.closed) return;
    const closed = new Promise<void>((resolve) => socket.once('close', resolve));
    socket.destroy();
    await closed;
  };
  try {
    const status = await new Promise<number>((resolve, reject) => {
      let received = Buffer.alloc(0);
      const timer = setTimeout(() => { reject(new Error('Movie stream headers timed out')); socket.destroy(); }, 10_000);
      socket.once('error', reject);
      socket.once('close', () => { clearTimeout(timer); reject(new Error('Movie stream closed before headers')); });
      socket.once(url.protocol === 'https:' ? 'secureConnect' : 'connect', () => {
        socket.write(`GET ${url.pathname}${url.search} HTTP/1.1\r\nHost: ${url.host}\r\nRange: bytes=0-\r\nConnection: close\r\n\r\n`);
      });
      socket.on('data', (chunk: Buffer) => {
        received = Buffer.concat([received, chunk]);
        const headerEnd = received.indexOf('\r\n\r\n');
        if (headerEnd > 65_536) {
          reject(new Error('Movie response headers exceed the fixture budget'));
          socket.destroy();
        } else if (headerEnd >= 0) {
          clearTimeout(timer);
          socket.pause();
          const match = /^HTTP\/1\.1 (\d{3}) /u.exec(received.toString('ascii', 0, headerEnd));
          if (match) resolve(Number(match[1])); else reject(new Error('Invalid movie response status'));
        } else if (received.length > 65_536) {
          reject(new Error('Movie response headers exceed the fixture budget'));
          socket.destroy();
        }
      });
    });
    return { status, close };
  } catch (error) {
    await close();
    throw error;
  }
}

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
