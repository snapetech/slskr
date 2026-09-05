// <copyright file="security.test.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import api from './api';
import * as security from './security';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    put: vi.fn(),
  },
}));

describe('security', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('uses the relative security dashboard path', async () => {
    api.get.mockResolvedValue({ data: { ok: true } });

    await security.getDashboard();

    expect(api.get).toHaveBeenCalledWith('/security/dashboard');
  });

  it('uses the relative reputation path without double-prefixing', async () => {
    api.get.mockResolvedValue({ data: { score: 10 } });

    await security.getReputation('alice');

    expect(api.get).toHaveBeenCalledWith('/security/reputation/alice');
  });

  it.each([
    ['dashboard', security.getDashboard, {}],
    ['events', security.getEvents, []],
    ['bans', security.getBans, { bans: [] }],
    ['reputation', security.getReputation, {}],
    ['suspicious peers', security.getSuspiciousPeers, []],
    ['trusted peers', security.getTrustedPeers, []],
    ['scanners', security.getScanners, []],
    ['threats', security.getThreats, []],
    ['canary statistics', security.getCanaryStats, {}],
    ['disclosure', security.getDisclosure, {}],
    ['network statistics', security.getNetworkStats, {}],
    ['top connectors', security.getTopConnectors, []],
    ['anomalies', security.getAnomalies, []],
    ['adversarial settings', security.getAdversarialSettings, {}],
    ['adversarial statistics', security.getAdversarialStats, {}],
    ['transport status', security.getTransportStatus, {}],
    ['transport collection', security.getAllTransportStatuses, {}],
    ['Tor status', security.getTorStatus, {}],
  ])('returns a validated %s response', async (_, helper, data) => {
    api.get.mockResolvedValue({ data });
    const promise =
      helper === security.getReputation || helper === security.getDisclosure
        ? helper('alice')
        : helper();
    await expect(promise).resolves.toEqual(data);
  });

  it.each([
    ['dashboard', security.getDashboard],
    ['events', security.getEvents],
    ['bans', security.getBans],
    ['reputation', security.getReputation],
    ['suspicious peers', security.getSuspiciousPeers],
    ['trusted peers', security.getTrustedPeers],
    ['scanners', security.getScanners],
    ['threats', security.getThreats],
    ['canary statistics', security.getCanaryStats],
    ['disclosure', security.getDisclosure],
    ['network statistics', security.getNetworkStats],
    ['top connectors', security.getTopConnectors],
    ['anomalies', security.getAnomalies],
    ['adversarial settings', security.getAdversarialSettings],
    ['adversarial statistics', security.getAdversarialStats],
    ['transport status', security.getTransportStatus],
    ['transport collection', security.getAllTransportStatuses],
    ['Tor status', security.getTorStatus],
  ])('rejects malformed %s responses', async (_, helper) => {
    api.get.mockResolvedValue({ data: 'malformed' });
    const promise =
      helper === security.getReputation || helper === security.getDisclosure
        ? helper('alice')
        : helper();
    await expect(promise).rejects.toThrow('Security API returned an invalid');
  });
});
