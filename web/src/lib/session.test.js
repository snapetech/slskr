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

  it('omits Authorization when token passthrough is enabled', () => {
    session.enablePassthrough();

    expect(session.authHeaders()).toEqual({});
    expect(session.isLoggedIn()).toBe(false);
  });

  it('ignores legacy tokens left in persistent browser storage', () => {
    localStorage.setItem('slskr-token', 'persistent-token');

    expect(session.authHeaders()).toEqual({});
    expect(session.isLoggedIn()).toBe(false);
  });

  it('adds CSRF only when requested for direct fetch mutations', () => {
    expect(session.authHeaders()).toEqual({});
    expect(session.authHeaders({ csrf: true })).toEqual({
      'X-CSRF-TOKEN': 'csrf-token',
    });
  });

  it('logs in with a username and password, storing the server-issued token', async () => {
    api.post.mockResolvedValue({
      data: {
        name: 'user',
        token: 'server-issued-jwt',
        tokenType: 'Bearer',
      },
    });

    await expect(
      session.login({ password: 'user-password', rememberMe: false, username: 'user' }),
    ).resolves.toBe('server-issued-jwt');

    // POST /api/v0/session expects { username, password } in the body, and
    // returns a signed JWT to use for subsequent requests — there is no
    // Authorization header on this call, and the client trusts the token
    // the server issues rather than reusing the password as a token.
    expect(api.post).toHaveBeenCalledWith('/session', {
      password: 'user-password',
      username: 'user',
    });
    expect(sessionStorage.getItem('slskr-token')).toBe('server-issued-jwt');
  });

  it('remembers the session in persistent storage when rememberMe is set', async () => {
    api.post.mockResolvedValue({
      data: { token: 'server-issued-jwt' },
    });

    await session.login({ password: 'user-password', rememberMe: true, username: 'user' });

    expect(localStorage.getItem('slskr-token')).toBe('server-issued-jwt');
    expect(sessionStorage.getItem('slskr-token')).toBeNull();
  });

  it('rethrows network session-check errors without masking them', async () => {
    setToken(sessionStorage, 'jwt-token');
    const error = new Error('network down');
    api.get.mockRejectedValue(error);

    await expect(session.check()).rejects.toThrow('network down');
    expect(sessionStorage.getItem('slskr-token')).toBe('jwt-token');
  });

  it('checks the session when authentication is disabled', async () => {
    session.enablePassthrough();
    api.get.mockResolvedValue({ data: { state: 'disconnected' } });

    await expect(session.check()).resolves.toBe(true);
    expect(api.get).toHaveBeenCalledWith('/session');
  });
});
