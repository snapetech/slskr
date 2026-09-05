// <copyright file="bridge.test.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import api from './api';
import * as bridge from './bridge';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
  },
}));

describe('bridge', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('returns bridge config from the response body', async () => {
    api.get.mockResolvedValue({ data: { enabled: true, port: 2242 } });

    const result = await bridge.getConfig();

    expect(api.get).toHaveBeenCalledWith('/bridge/admin/config');
    expect(result).toEqual({ enabled: true, port: 2242 });
  });

  it('returns dashboard data from the response body', async () => {
    api.get.mockResolvedValue({ data: { health: { status: 'ok' } } });

    const result = await bridge.getDashboard();

    expect(api.get).toHaveBeenCalledWith('/bridge/admin/dashboard');
    expect(result).toEqual({ health: { status: 'ok' } });
  });

  it('returns an empty client list when the response has no clients', async () => {
    api.get.mockResolvedValue({ data: { clients: [] } });

    const result = await bridge.getClients();

    expect(result).toEqual([]);
  });

  it.each([
    ['configuration', bridge.getConfig],
    ['dashboard', bridge.getDashboard],
    ['stats', bridge.getStats],
    ['status', bridge.getStatus],
    ['clients', bridge.getClients],
  ])('rejects malformed %s responses', async (resource, request) => {
    api.get.mockResolvedValue({ data: resource === 'clients' ? {} : [] });

    await expect(request()).rejects.toThrow(
      resource === 'clients'
        ? 'Bridge API returned an invalid client list response'
        : `Bridge API returned an invalid ${resource} response`,
    );
  });
});
