// Browser fetch defaults to following redirects. API requests carry bearer,
// CSRF, or delegated share credentials, so callers must opt into fail-closed
// redirect handling explicitly.
export const fetchWithoutRedirects = (input, init = {}) =>
  fetch(input, { ...init, redirect: 'error' });

export const MAX_JSON_RESPONSE_BYTES = 8 * 1024 * 1024;

/**
 * Parse a JSON response without allowing a remote endpoint to allocate an
 * unbounded string or object before the caller can validate it.
 */
export const readJsonResponse = async (
  response,
  maximum = MAX_JSON_RESPONSE_BYTES,
) => {
  const declaredLength = response.headers?.get('content-length');
  if (declaredLength !== null && declaredLength !== undefined) {
    const length = Number(declaredLength);
    if (Number.isFinite(length) && length > maximum) {
      await response.body?.cancel?.();
      throw new Error(`HTTP response body exceeds ${maximum} bytes`);
    }
  }

  const reader = response.body?.getReader?.();
  if (!reader) {
    const text = await response.text();
    if (!text.trim()) return undefined;
    if (new TextEncoder().encode(text).byteLength > maximum) {
      throw new Error(`HTTP response body exceeds ${maximum} bytes`);
    }
    return JSON.parse(text);
  }

  const chunks = [];
  let length = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    length += value.byteLength;
    if (length > maximum) {
      await reader.cancel();
      throw new Error(`HTTP response body exceeds ${maximum} bytes`);
    }
    chunks.push(value);
  }

  const body = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    body.set(chunk, offset);
    offset += chunk.byteLength;
  }
  const text = new TextDecoder().decode(body);
  return text.trim() ? JSON.parse(text) : undefined;
};
