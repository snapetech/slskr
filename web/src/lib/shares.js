import api from './api';

const requireObjectResponse = (value, resource) => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`Shares API returned an invalid ${resource} response`);
  }

  return value;
};

const requireArrayResponse = (value, resource) => {
  if (!Array.isArray(value)) {
    throw new Error(`Shares API returned an invalid ${resource} response`);
  }

  return value;
};

export const getAll = async () => {
  const data = (await api.get('/shares')).data;
  return requireObjectResponse(data, 'share list');
};

export const get = async ({ id } = {}) => {
  if (!id) throw new Error('unable to get share: id is missing');
  const data = (await api.get(`/shares/${encodeURIComponent(id)}`)).data;
  return requireObjectResponse(data, 'share');
};

export const browseAll = async () => {
  const data = (await api.get('/shares/contents')).data;
  return requireArrayResponse(data, 'share contents');
};

export const browse = async ({ id } = {}) => {
  if (!id) throw new Error('unable to get share contents: id is missing');
  const data = (
    await api.get(`/shares/${encodeURIComponent(id)}/contents`)
  ).data;
  return requireArrayResponse(data, 'share contents');
};

export const rescan = async () => {
  return (await api.put('/shares')).data;
};

export const cancel = async () => {
  return (await api.delete('/shares')).data;
};
