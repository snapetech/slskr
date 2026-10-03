// Empty success responses used by the browser audit, checked by real API adapters.
export const auditResponseContract = (pathname) => {
  switch (pathname) {
    case '/conversations/activity/unacknowledged':
      return false;
    case '/rooms/activity':
      return {};
    case '/hashdb/metadata-processing':
      return { active: [], history: [] };
    case '/hashdb/stats':
      return { currentSeqId: 0, totalHashEntries: 0 };
    case '/jobs':
      return { jobs: [], total: 0, limit: 20, offset: 0 };
    case '/autoreplace':
      return { enabled: false };
    case '/backfill/stats':
      return { isActive: false, isRunning: false };
    case '/bridge/admin/config':
      return {
        enabled: false,
        max_clients: 10,
        port: 2242,
        require_auth: true,
        soulfind_path: 'soulfind',
      };
    case '/bridge/admin/dashboard':
      return {
        connectedClients: [],
        health: { isHealthy: false, version: '' },
        meshBenefits: {},
        stats: {
          currentConnections: 0,
          totalBytesProxied: 0,
          totalDownloads: 0,
          totalSearches: 0,
        },
      };
    case '/security/dashboard':
      return {};
    case '/source-providers':
      return {
        acquisitionPlanningEnabled: false,
        profilePolicies: [],
        providers: [],
      };
    case '/swarm/analytics/performance':
      return {
        totalDownloads: 0,
        successRate: 0,
        averageSpeedBytesPerSecond: 0,
        averageDurationSeconds: 0,
        totalBytesDownloaded: 0,
        totalChunksCompleted: 0,
        chunkSuccessRate: 0,
      };
    case '/swarm/analytics/efficiency':
      return { chunkUtilization: 0, peerUtilization: 0, redundancyFactor: 0 };
    case '/swarm/analytics/trends':
      return {};
    default:
      return undefined;
  }
};
