const urlBase = (window.urlBase === '/' ? '' : window.urlBase) || '';
const developmentPort =
  window.port ?? (import.meta.env.VITE_SLSKR_PORT || 5_030);
const rootUrl =
  import.meta.env.PROD
    ? urlBase
    : import.meta.env.VITE_USE_ABSOLUTE_API_URL === 'true'
      ? `http://localhost:${developmentPort}${urlBase}`
      : urlBase;
const apiBaseUrl = `${rootUrl}/api/v0`;
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
  rootUrl,
  tokenKey,
  tokenPassthroughValue,
  urlBase,
};
