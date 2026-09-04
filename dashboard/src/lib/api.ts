export function requestHeaders(apiKey: string | null, body?: BodyInit | null): Headers {
  const headers = new Headers();
  if (apiKey) headers.set('Authorization', `Bearer ${apiKey}`);
  if (body != null) headers.set('Content-Type', 'application/json');
  return headers;
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
    headers,
  });
  const body = await response.text();

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
