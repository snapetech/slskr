export const normalizeUrlBase = (value) => {
  if (typeof value !== 'string') return '';
  const trimmed = value.trim();
  if (!trimmed || trimmed === '/') return '';
  if (
    !trimmed.startsWith('/') ||
    trimmed.includes('?') ||
    trimmed.includes('#') ||
    trimmed.split('/').some((segment) => segment === '..' || segment === '.')
  ) {
    return '';
  }
  const path = trimmed.replace(/^\/+|\/+$/gu, '');
  return path ? `/${path}` : '';
};

const urlBase = normalizeUrlBase(window.urlBase);
const developmentPort =
  window.port ?? (import.meta.env.VITE_SLSKR_PORT || 5_030);
const rootUrl =
  import.meta.env.PROD
    ? urlBase
    : import.meta.env.VITE_USE_ABSOLUTE_API_URL === 'true'
      ? `http://localhost:${developmentPort}${urlBase}`
      : urlBase;
const apiBaseUrl = `${rootUrl}/api/v0`;
const hubBaseUrl = `${rootUrl}/hub`;
const tokenKey = 'slskr-token';
const tokenPassthroughValue = 'n/a';
const activeChatKey = 'slskr-active-chat';
const activeRoomKey = 'slskr-active-room';
const activeUserInfoKey = 'slskr-active-user';

export {
  activeChatKey,
  activeRoomKey,
  activeUserInfoKey,
  apiBaseUrl,
  hubBaseUrl,
  rootUrl,
  tokenKey,
  tokenPassthroughValue,
  urlBase,
};
