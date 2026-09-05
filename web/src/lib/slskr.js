import api from './api';
import { toDisplayError } from './errors';
import { encodePathSegment } from './pathEncoding';
import { normalizeSwarmJobList } from './swarmJobs';

const isRecord = (value) =>
  value !== null && typeof value === 'object' && !Array.isArray(value);

// Older daemon profiles may not expose every optional endpoint. Preserve the
// compatibility fallback for that case, but surface real outages and auth
// failures so the UI cannot report fabricated empty state.
const safeGet = async (endpoint, fallback = null) => {
  try {
    const response = await api.get(endpoint);
    return response.data;
  } catch (error) {
    if (error?.response?.status === 404) {
      console.debug(
        `Endpoint ${endpoint} is unavailable on this daemon profile`,
      );
      return fallback;
    }

    throw error;
  }
};

const MAX_PEER_RECORDS = 512;
const normalizePeerList = (value) => {
  const peers = Array.isArray(value)
    ? value
    : value && typeof value === 'object' && Array.isArray(value.peers)
      ? value.peers
      : [];

  return peers
    .filter((peer) => peer && typeof peer === 'object' && !Array.isArray(peer))
    .slice(0, MAX_PEER_RECORDS);
};

// Capabilities API
export const getCapabilities = async () => {
  return safeGet('/capabilities', { features: [] });
};

export const getDiscoveredPeers = async () => {
  return normalizePeerList(await safeGet('/capabilities/peers', { peers: [] }));
};

// HashDatabase API
export const getHashDatabaseStats = async () => {
  return safeGet('/hashdb/stats', { currentSeqId: 0, totalHashEntries: 0 });
};

export const getHashDatabaseEntries = async (limit = 100, offset = 0) => {
  const parameters = new URLSearchParams({
    limit: String(limit),
    offset: String(offset),
  });
  return safeGet(`/hashdb/entries?${parameters.toString()}`, {
    entries: [],
  });
};

export const getMetadataProcessingStatus = async (limit = 50) => {
  const parameters = new URLSearchParams({ limit: String(limit) });
  return safeGet(`/hashdb/metadata-processing?${parameters.toString()}`, {
    active: [],
    history: [],
  });
};

// Mesh API
export const getMeshStats = async () => {
  return safeGet('/mesh/stats', {
    currentSeqId: 0,
    isSyncing: false,
    knownMeshPeers: 0,
  });
};

export const getMeshPeers = async () => {
  return normalizePeerList(await safeGet('/mesh/peers', { peers: [] }));
};

export const triggerMeshSync = async (username) => {
  try {
    return (await api.post(`/mesh/sync/${encodePathSegment(username)}`)).data;
  } catch (error) {
    return { error: toDisplayError(error, 'Sync failed'), success: false };
  }
};

// Backfill API
export const getBackfillStats = async () => {
  return safeGet('/backfill/stats', { isActive: false, isRunning: false });
};

export const getBackfillCandidates = async (limit = 50) => {
  const parameters = new URLSearchParams({ limit: String(limit) });
  return safeGet(`/backfill/candidates?${parameters.toString()}`, {
    candidates: [],
  });
};

export const backfillFromSearchHistory = async (options = {}) => {
  try {
    const searchParameters = new URLSearchParams();

    if (options.batchSize) {
      searchParameters.append('batchSize', options.batchSize);
    }

    if (options.reset) {
      searchParameters.append('reset', 'true');
    }

    const query = searchParameters.toString()
      ? `?${searchParameters.toString()}`
      : '';

    return (await api.post(`/hashdb/backfill/from-history${query}`)).data;
  } catch (error) {
    return { error: toDisplayError(error, 'Backfill failed'), success: false };
  }
};

// MultiSource API
export const getActiveSwarmJobs = async () => {
  return normalizeSwarmJobList(await safeGet('/multisource/jobs', { jobs: [] }));
};

export const getSwarmJob = async (jobId) => {
  return safeGet(`/multisource/jobs/${encodePathSegment(jobId)}`, null);
};

// DHT API
export const getDhtStatus = async () => {
  const dht = await safeGet('/dht/status', {
    dhtNodeCount: 0,
    isLanOnly: false,
    isBeaconCapable: false,
    isDhtRunning: false,
    verifiedBeaconCount: 0,
  });
  const normalizedDht = isRecord(dht) ? dht : {};

  return {
    ...normalizedDht,
    isLanOnly: normalizedDht.isLanOnly ?? normalizedDht.lanOnly ?? false,
  };
};

// Combined stats fetch for dashboard
export const getSlskrStats = async () => {
  // Each helper preserves the compatibility default for a genuinely absent
  // optional endpoint. Promise.all must still reject real transport, auth, and
  // server failures so callers cannot mistake an outage for an empty runtime.
  const [capabilities, hashDatabase, mesh, backfill, swarmJobs, dht] =
    await Promise.all([
      getCapabilities(),
      getHashDatabaseStats(),
      getMeshStats(),
      getBackfillStats(),
      getActiveSwarmJobs(),
      getDhtStatus(),
    ]);

  const rawHashDatabase = isRecord(hashDatabase) ? hashDatabase : {};
  const normalizedHashDatabase = {
    ...rawHashDatabase,
    currentSeqId: rawHashDatabase.currentSeqId ?? 0,
    // Map backend field names to frontend expectations
    totalEntries:
      rawHashDatabase.totalHashEntries ?? rawHashDatabase.totalEntries ?? 0,
  };

  const rawMesh = isRecord(mesh) ? mesh : {};
  const normalizedMesh = {
    ...rawMesh,
    // Map backend field names to frontend expectations
    connectedPeerCount:
      rawMesh.knownMeshPeers ?? rawMesh.connectedPeerCount ?? 0,
    isSyncing: rawMesh.isSyncing ?? false,
    localSeqId: rawMesh.currentSeqId ?? rawMesh.localSeqId ?? 0,
    warnings: Array.isArray(rawMesh.warnings) ? rawMesh.warnings : [],
  };

  const rawBackfill = isRecord(backfill) ? backfill : {};
  const normalizedBackfill = {
    ...rawBackfill,
    isActive: rawBackfill.isActive ?? rawBackfill.isRunning ?? false,
  };

  return {
    backfill: normalizedBackfill,
    capabilities: isRecord(capabilities) ? capabilities : null,
    dht: isRecord(dht) ? dht : null,
    hashDb: normalizedHashDatabase,
    mesh: normalizedMesh,
    swarmJobs: Array.isArray(swarmJobs) ? swarmJobs : [],
  };
};
