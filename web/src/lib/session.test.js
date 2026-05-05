// <copyright file="session.test.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import * as session from './session';
import api from './api';
import { setToken } from './token';

vi.mock('./api', async () => {
  const actual = await vi.importActual('./api');
  return {
    ...actual,
    default: {
      get: vi.fn(),
      post: vi.fn(),
    },
  };
});

describe('session', () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
    Object.defineProperty(document, 'cookie', {
      configurable: true,
      value: 'XSRF-TOKEN=csrf-token',
      writable: true,
    });
  });

  it('adds Authorization when a bearer token is stored', () => {
    setToken(sessionStorage, 'jwt-token');

    expect(session.authHeaders()).toEqual({
      Authorization: 'Bearer jwt-token',
    });
  });

  it('adds CSRF only when requested for direct fetch mutations', () => {
    expect(session.authHeaders()).toEqual({});
    expect(session.authHeaders({ csrf: true })).toEqual({
      'X-CSRF-TOKEN': 'csrf-token',
    });
  });

  it('verifies a user supplied token without accepting a token echo from the API', async () => {
    api.post.mockResolvedValue({
      data: {
        name: 'slskr',
        token: 'server-token-must-not-be-used',
        tokenConfigured: true,
      },
    });

    await expect(
      session.login({ username: 'user', password: 'user-token', rememberMe: false }),
    ).resolves.toBe('user-token');

    expect(api.post).toHaveBeenCalledWith(
      '/session',
      { username: 'user' },
      { headers: { Authorization: 'Bearer user-token' } },
    );
    expect(sessionStorage.getItem('slskr-token')).toBe('user-token');
    expect(sessionStorage.getItem('slskr-token')).not.toBe(
      'server-token-must-not-be-used',
    );
  });
});
