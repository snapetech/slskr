// Browser fetch defaults to following redirects. API requests carry bearer,
// CSRF, or delegated share credentials, so callers must opt into fail-closed
// redirect handling explicitly.
export const fetchWithoutRedirects = (input, init = {}) =>
  fetch(input, { ...init, redirect: 'error' });
