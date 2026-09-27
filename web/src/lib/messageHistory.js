const CURSOR_FIELDS = ['createdAtMs', 'created_at_ms', 'timestampUnixMs'];

const asRecord = (value) =>
  value && typeof value === 'object' && !Array.isArray(value) ? value : null;

const firstValue = (message, fields) => {
  for (const field of fields) {
    if (message[field] !== undefined && message[field] !== null) {
      return message[field];
    }
  }
  return undefined;
};

/**
 * Return the server's millisecond cursor when a message exposes one.
 * Human-readable `timestamp` fields are intentionally excluded because older
 * profiles use both seconds and formatted strings for that field.
 */
export const messageCursor = (message) => {
  const record = asRecord(message);
  if (!record) return null;

  const value = Number(firstValue(record, CURSOR_FIELDS));
  return Number.isSafeInteger(value) && value >= 0 ? value : null;
};

export const historyCursor = (messages) => {
  let cursor = null;
  for (const message of Array.isArray(messages) ? messages : []) {
    const value = messageCursor(message);
    if (value !== null && (cursor === null || value > cursor)) {
      cursor = value;
    }
  }
  return cursor;
};

const messageIdentity = (message) => {
  const record = asRecord(message) || {};
  const id = firstValue(record, ['messageId', 'message_id', 'id']);
  if (id !== undefined && id !== null && String(id) !== '') {
    return `id:${String(id)}`;
  }

  const cursor = messageCursor(record);
  const sender = firstValue(record, [
    'username',
    'senderPeerId',
    'sender',
    'direction',
  ]);
  const body = firstValue(record, ['message', 'body']) ?? '';
  return `fallback:${cursor ?? 'unknown'}:${String(sender ?? '')}:${String(body)}`;
};

export const mergeMessageHistory = (
  previousMessages,
  incomingMessages,
  maximumMessages,
) => {
  const previous = Array.isArray(previousMessages) ? previousMessages : [];
  const incoming = Array.isArray(incomingMessages) ? incomingMessages : [];
  const maximum = Number.isSafeInteger(maximumMessages) && maximumMessages > 0
    ? maximumMessages
    : 500;
  const byIdentity = new Map();

  for (const message of [...previous, ...incoming]) {
    if (!asRecord(message)) continue;
    byIdentity.set(messageIdentity(message), message);
  }

  return [...byIdentity.values()].slice(-maximum);
};

export const messageKey = messageIdentity;

export const isAbortError = (error, signal) =>
  signal?.aborted ||
  error?.name === 'AbortError' ||
  error?.code === 'ERR_CANCELED';
