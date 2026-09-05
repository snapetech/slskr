import api from './api';
import { toDisplayError } from './errors';
import { encodePathSegment } from './pathEncoding';
import { normalizeSwarmJobList } from './swarmJobs';

const isRecord = (value) =>
  value !== null && typeof value === 'object' && !Array.isArray(value);

const requireRecord = (value, resource) => {
  if (!isRecord(value)) {
    throw new Error(`slskR API returned an invalid ${resource} response`);
  }

  return value;
};

const requireArrayField = (value, resource) => {
  if (!Array.isArray(value)) {
    throw new Error(`slskR API returned an invalid ${resource} response`);
  }

  return value;
};

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
    : isRecord(value) && Array.isArray(value.peers)
      ? value.peers
      : requireArrayField(value, 'peer list');

  return peers
    .filter((peer) => peer && typeof peer === 'object' && !Array.isArray(peer))
    .slice(0, MAX_PEER_RECORDS);
};

// Capabilities API
export const getCapabilities = async () => {
  return requireRecord(
    await safeGet('/capabilities', { features: [] }),
    'capabilities',
  );
};

export const getDiscoveredPeers = async () => {
  return normalizePeerList(await safeGet('/capabilities/peers', { peers: [] }));
};

// HashDatabase API
export const getHashDatabaseStats = async () => {
  return requireRecord(
    await safeGet('/hashdb/stats', { currentSeqId: 0, totalHashEntries: 0 }),
    'hash database stats',
  );
};

export const getHashDatabaseEntries = async (limit = 100, offset = 0) => {
  const parameters = new URLSearchParams({
    limit: String(limit),
    offset: String(offset),
  });
  const response = await safeGet(`/hashdb/entries?${parameters.toString()}`, {
    entries: [],
  });
  const entries = requireRecord(response, 'hash database entries');
  requireArrayField(entries.entries, 'hash database entry list');
  return entries;
};

export const getMetadataProcessingStatus = async (limit = 50) => {
  const parameters = new URLSearchParams({ limit: String(limit) });
  const response = await safeGet(`/hashdb/metadata-processing?${parameters.toString()}`, {
    active: [],
    history: [],
  });
  const status = requireRecord(response, 'metadata processing status');
  requireArrayField(status.active, 'metadata processing active list');
  requireArrayField(status.history, 'metadata processing history list');
  return status;
};

// Mesh API
export const getMeshStats = async () => {
  return requireRecord(
    await safeGet('/mesh/stats', {
      currentSeqId: 0,
      isSyncing: false,
      knownMeshPeers: 0,
    }),
    'mesh stats',
  );
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
  return requireRecord(
    await safeGet('/backfill/stats', { isActive: false, isRunning: false }),
    'backfill stats',
  );
};

export const getBackfillCandidates = async (limit = 50) => {
  const parameters = new URLSearchParams({ limit: String(limit) });
  const response = await safeGet(`/backfill/candidates?${parameters.toString()}`, {
    candidates: [],
  });
  const candidates = requireRecord(response, 'backfill candidates');
  requireArrayField(candidates.candidates, 'backfill candidate list');
  return candidates;
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
  const response = await safeGet('/multisource/jobs', { jobs: [] });
  if (!Array.isArray(response)) {
    const jobs = requireRecord(response, 'swarm jobs');
    requireArrayField(jobs.jobs, 'swarm job list');
  }
  return normalizeSwarmJobList(response);
};

export const getSwarmJob = async (jobId) => {
  const response = await safeGet(
    `/multisource/jobs/${encodePathSegment(jobId)}`,
    null,
  );
  return response === null ? null : requireRecord(response, 'swarm job');
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
  const normalizedDht = requireRecord(dht, 'DHT status');

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
