import api from './api';

const isRecord = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

const validateMessages = (messages, label) => {
  if (messages === undefined) return;
  if (!Array.isArray(messages) || messages.some((message) => !isRecord(message))) {
    throw new Error(`Chat API returned an invalid ${label} messages response`);
  }
};

const validateConversation = (conversation, label) => {
  if (!isRecord(conversation)) {
    throw new Error(`Chat API returned an invalid ${label} conversation response`);
  }
  if (
    conversation.isActive !== undefined &&
    typeof conversation.isActive !== 'boolean'
  ) {
    throw new Error(`Chat API returned an invalid ${label} conversation response`);
  }
  if (
    conversation.hasUnAcknowledgedMessages !== undefined &&
    typeof conversation.hasUnAcknowledgedMessages !== 'boolean'
  ) {
    throw new Error(`Chat API returned an invalid ${label} conversation response`);
  }
  if (
    conversation.unAcknowledgedMessageCount !== undefined &&
    (!Number.isSafeInteger(conversation.unAcknowledgedMessageCount) ||
      conversation.unAcknowledgedMessageCount < 0)
  ) {
    throw new Error(`Chat API returned an invalid ${label} conversation response`);
  }
  validateMessages(conversation.messages, label);
  return conversation;
};

export const getAll = async ({ unAcknowledgedOnly = false } = {}) => {
  const query = unAcknowledgedOnly ? '?unAcknowledgedOnly=true' : '';
  const data = (await api.get(`/conversations${query}`)).data;
  if (!Array.isArray(data)) {
    throw new Error('Chat API returned an invalid conversations response');
  }
  data.forEach((conversation) => validateConversation(conversation, 'list'));
  return data;
};

export const hasUnAcknowledgedMessages = async () => {
  const data = (await api.get('/conversations/activity/unacknowledged')).data;
  if (typeof data !== 'boolean') {
    throw new Error('Chat API returned an invalid unread activity response');
  }
  return data;
};

export const get = async ({ username, since = null }) => {
  const query = since == null ? '' : `?since=${encodeURIComponent(since)}`;
  const data = (
    await api.get(`/conversations/${encodeURIComponent(username)}${query}`)
  ).data;
  return validateConversation(data, 'detail');
};

export const acknowledge = ({ username }) => {
  return api.put(`/conversations/${encodeURIComponent(username)}`);
};

export const send = ({ username, message }) => {
  return api.post(
    `/conversations/${encodeURIComponent(username)}`,
    JSON.stringify(message),
  );
};

export const sendBatch = ({ message, usernames }) => {
  return api.post('/conversations/batch', {
    message,
    usernames,
  });
};

export const remove = ({ username }) => {
  return api.delete(`/conversations/${encodeURIComponent(username)}`);
};
