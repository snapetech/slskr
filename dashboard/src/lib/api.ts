export function requestHeaders(apiKey: string | null, body?: BodyInit | null): Headers {
  const headers = new Headers();
  if (apiKey) headers.set('Authorization', `Bearer ${apiKey}`);
  if (body != null) headers.set('Content-Type', 'application/json');
  return headers;
}

export const MAX_HTTP_RESPONSE_BYTES = 8 * 1024 * 1024;
export const MAX_HTTP_ERROR_BYTES = 64 * 1024;

export async function readResponseText(
  response: Response,
  maximum: number = MAX_HTTP_RESPONSE_BYTES,
): Promise<string> {
  const declaredLength = response.headers.get('content-length');
  if (declaredLength !== null) {
    const length = Number(declaredLength);
    if (Number.isFinite(length) && length > maximum) {
      await response.body?.cancel();
      throw new Error(`HTTP response body exceeds ${maximum} bytes`);
    }
  }

  const reader = response.body?.getReader();
  if (!reader) {
    const text = await response.text();
    if (new TextEncoder().encode(text).byteLength > maximum) {
      throw new Error(`HTTP response body exceeds ${maximum} bytes`);
    }
    return text;
  }

  const chunks: Uint8Array[] = [];
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
  return new TextDecoder().decode(body);
}

function responseMessage(status: number, statusText: string, body: string): string {
  if (body) {
    try {
      const payload = JSON.parse(body) as Record<string, unknown>;
      const detail = payload.detail ?? payload.error ?? payload.reason ?? payload.title;
      if (typeof detail === 'string' && detail.trim()) return detail;
    } catch {
      // Keep the HTTP status when the server did not return JSON.
    }
  }

  return `HTTP ${status}${statusText ? `: ${statusText}` : ''}`;
}

export async function requestJson<T>(
  url: string,
  apiKey: string | null,
  init: RequestInit = {},
): Promise<T> {
  const headers = requestHeaders(apiKey, init.body);
  new Headers(init.headers).forEach((value, key) => headers.set(key, value));

  const response = await fetch(url, {
    ...init,
    // Do not allow an API key-bearing request to follow a redirect to a
    // different origin. Callers cannot opt back into redirect following.
    redirect: 'error',
    headers,
  });
  const body = await readResponseText(
    response,
    response.ok ? MAX_HTTP_RESPONSE_BYTES : MAX_HTTP_ERROR_BYTES,
  );

  if (!response.ok) {
    throw new Error(responseMessage(response.status, response.statusText, body));
  }
  if (!body.trim()) return undefined as T;

  try {
    return JSON.parse(body) as T;
  } catch {
    throw new Error('The server returned invalid JSON');
  }
}

export function isAbortError(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'name' in error &&
    error.name === 'AbortError'
  );
}
