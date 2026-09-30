import {
  historyCursor,
  mergeMessageHistory,
  messageCursor,
  messageKey,
} from './messageHistory';

describe('message history helpers', () => {
  it('reads only explicit millisecond cursor fields', () => {
    expect(messageCursor({ createdAtMs: 10 })).toBe(10);
    expect(messageCursor({ timestampUnixMs: '12' })).toBe(12);
    expect(messageCursor({ timestamp: '12' })).toBeNull();
    expect(historyCursor([{ createdAtMs: 10 }, { createdAtMs: 15 }])).toBe(15);
  });

  it('replaces duplicate identities while preserving bounded history', () => {
    const previous = [
      { id: 'one', body: 'old', createdAtMs: 1 },
      { id: 'two', body: 'two', createdAtMs: 2 },
    ];
    const incoming = [
      { id: 'two', body: 'updated', createdAtMs: 2 },
      { id: 'three', body: 'three', createdAtMs: 3 },
    ];

    expect(mergeMessageHistory(previous, incoming, 2)).toEqual([
      { id: 'two', body: 'updated', createdAtMs: 2 },
      { id: 'three', body: 'three', createdAtMs: 3 },
    ]);
    expect(messageKey(incoming[0])).toBe(messageKey(previous[1]));
  });

  it('uses a content fallback identity for profiles without message ids', () => {
    const message = {
      body: 'hello',
      senderPeerId: 'peer',
      timestampUnixMs: 42,
    };
    expect(mergeMessageHistory([message], [message], 100)).toEqual([message]);
  });
});
