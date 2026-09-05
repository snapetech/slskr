// <copyright file="api.test.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import api, { getCsrfTokenFromCookieString, reloadAfterUnauthorized } from './api';

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
});
