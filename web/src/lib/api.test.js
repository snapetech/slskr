// <copyright file="api.test.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import { apiBaseUrl } from '../config';
import api, {
  buildApiUrl,
  getCsrfTokenFromCookieString,
  reloadAfterUnauthorized,
} from './api';
import { setToken } from './token';

describe('api csrf token selection', () => {
  afterEach(() => {
    delete window.port;
  });

  it('prefers the current port scoped csrf token', () => {
    const token = getCsrfTokenFromCookieString(
      'XSRF-TOKEN-5031=https-token; XSRF-TOKEN-5030=http-token',
      '5030',
    );

    expect(token).toBe('http-token');
  });

  it('falls back to the legacy csrf token name', () => {
    const token = getCsrfTokenFromCookieString('XSRF-TOKEN=legacy-token', '');

    expect(token).toBe('legacy-token');
  });

  it('ignores the antiforgery cookie token name', () => {
    const token = getCsrfTokenFromCookieString(
      'XSRF-COOKIE-5030=cookie-token; XSRF-TOKEN-5030=request-token',
      '5030',
    );

    expect(token).toBe('request-token');
  });

  it('falls back to the only port scoped token when the browser url has no port', () => {
    const token = getCsrfTokenFromCookieString(
      'XSRF-TOKEN-5030=request-token',
      '',
    );

    expect(token).toBe('request-token');
  });

  it('uses the injected backend port by default before the browser url port', () => {
    window.port = '5030';

    const token = getCsrfTokenFromCookieString(
      'XSRF-TOKEN-5030=request-token; XSRF-TOKEN-443=proxy-token',
    );

    expect(token).toBe('request-token');
  });

  it('rejects ambiguous port-scoped cookies and ignores malformed entries', () => {
    expect(
      getCsrfTokenFromCookieString(
        '=invalid; XSRF-TOKEN-5030=first; XSRF-TOKEN-5031=second',
        '',
      ),
    ).toBeNull();
  });

  it('builds URLs from relative API paths and rejects paths with an API prefix', () => {
    expect(buildApiUrl('/users')).toBe(`${apiBaseUrl}/users`);
    expect(buildApiUrl('users')).toBe(`${apiBaseUrl}/users`);
    expect(() => buildApiUrl('/api/users')).toThrow('Do not include /api prefix');
    expect(() => buildApiUrl('api/v0/users')).toThrow('Do not include /api prefix');
  });

  it('skips browser navigation reload in tests', () => {
    const location = {
      reload: vi.fn(),
    };

    expect(reloadAfterUnauthorized(location, 'test')).toBe(false);
    expect(location.reload).not.toHaveBeenCalled();
  });

  it('reloads the page outside the test environment', () => {
    const location = {
      reload: vi.fn(),
    };

    expect(reloadAfterUnauthorized(location, 'production')).toBe(true);
    expect(location.reload).toHaveBeenCalledWith();
  });
});

describe('api transport limits', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('uses a bounded fetch transport with redirects disabled', async () => {
    const originalBaseUrl = api.defaults.baseURL;
    api.defaults.baseURL = 'http://localhost:3000/api/v0';
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(null, {
        headers: { 'Content-Length': String(8 * 1024 * 1024 + 1) },
        status: 200,
      }),
    );

    try {
      await expect(api.get('/oversized')).rejects.toThrow('maxContentLength');
      expect(fetchMock).toHaveBeenCalledOnce();
      expect(fetchMock.mock.calls[0][0].redirect).toBe('manual');
    } finally {
      api.defaults.baseURL = originalBaseUrl;
    }
  });

  it('adds a session bearer token and JSON content type to requests', async () => {
    const originalBaseUrl = api.defaults.baseURL;
    api.defaults.baseURL = 'http://localhost:3000/api/v0';
    setToken(sessionStorage, 'request-jwt');
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('{}', { status: 200 }),
    );

    try {
      await api.get('/users');
      const headers = new Headers(fetchMock.mock.calls[0][0].headers);
      expect(headers.get('Authorization')).toBe('Bearer request-jwt');
      expect(headers.get('Content-Type')).toBe('application/json');
    } finally {
      api.defaults.baseURL = originalBaseUrl;
      sessionStorage.clear();
    }
  });

  it('omits passthrough authorization and sends cookie CSRF on mutations', async () => {
    const originalBaseUrl = api.defaults.baseURL;
    const cookieDescriptor = Object.getOwnPropertyDescriptor(document, 'cookie');
    api.defaults.baseURL = 'http://localhost:3000/api/v0';
    setToken(sessionStorage, 'n/a');
    Object.defineProperty(document, 'cookie', {
      configurable: true,
      value: 'XSRF-TOKEN=csrf-token',
      writable: true,
    });
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('{}', { status: 200 }),
    );

    try {
      await api.post('/users', { name: 'user' });
      const headers = new Headers(fetchMock.mock.calls[0][0].headers);
      expect(headers.has('Authorization')).toBe(false);
      expect(headers.get('X-CSRF-TOKEN')).toBe('csrf-token');
    } finally {
      api.defaults.baseURL = originalBaseUrl;
      sessionStorage.clear();
      if (cookieDescriptor) {
        Object.defineProperty(document, 'cookie', cookieDescriptor);
      } else {
        delete document.cookie;
      }
    }
  });

  it.each(['/session', '/server', '/application'])(
    'preserves authentication state for exempt 401 route %s', async (path) => {
      const originalBaseUrl = api.defaults.baseURL;
      api.defaults.baseURL = 'http://localhost:3000/api/v0';
      setToken(sessionStorage, 'request-jwt');
      vi.spyOn(globalThis, 'fetch').mockResolvedValue(
        new Response('{}', { status: 401 }),
      );

      try {
        await expect(api.get(path)).rejects.toMatchObject({
          response: { status: 401 },
        });
        expect(sessionStorage.getItem('slskr-token')).toBe('request-jwt');
      } finally {
        api.defaults.baseURL = originalBaseUrl;
        sessionStorage.clear();
      }
    },
  );

  it('clears session credentials and logs empty-body responses on other 401s', async () => {
    const originalBaseUrl = api.defaults.baseURL;
    api.defaults.baseURL = 'http://localhost:3000/api/v0';
    setToken(sessionStorage, 'request-jwt');
    const logError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'debug').mockImplementation(() => {});
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(null, {
        headers: { 'Content-Length': '0' },
        status: 401,
      }),
    );

    try {
      await expect(api.get('/protected')).rejects.toMatchObject({
        response: { status: 401 },
      });
      expect(sessionStorage.getItem('slskr-token')).toBeNull();
      expect(logError).toHaveBeenCalledWith(
        expect.stringContaining('HTTP 401 with empty body'),
      );
    } finally {
      api.defaults.baseURL = originalBaseUrl;
      sessionStorage.clear();
    }
  });

  it('preserves network errors without a response payload', async () => {
    const originalBaseUrl = api.defaults.baseURL;
    api.defaults.baseURL = 'http://localhost:3000/api/v0';
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new TypeError('offline'));

    try {
      await expect(api.get('/users')).rejects.toThrow('offline');
    } finally {
      api.defaults.baseURL = originalBaseUrl;
    }
  });
});
