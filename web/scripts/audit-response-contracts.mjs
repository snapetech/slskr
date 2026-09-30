// Empty success responses used by the browser audit, checked by real API adapters.
export const auditResponseContract = (pathname) => {
  switch (pathname) {
    case '/conversations/activity/unacknowledged':
      return false;
    case '/rooms/activity':
      return {};
    case '/hashdb/metadata-processing':
      return { active: [], history: [] };
    case '/jobs':
      return { jobs: [], total: 0, limit: 20, offset: 0 };
    case '/autoreplace':
      return { enabled: false };
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
