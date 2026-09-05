import {
  getLocalStorageItem,
  removeLocalStorageItem,
  setLocalStorageItem,
} from './storage';
import { readBoundedJson } from './persistedJson';

export const meshEvidencePolicyStorageKey = 'slskr.meshEvidencePolicy';

const maxMeshPolicyStorageCharacters = 64 * 1024;
const maxEvidenceReviewInputCharacters = 1 * 1024 * 1024;
const maxEvidenceReviewEntries = 500;
const maxEvidenceTextCharacters = 2_048;

export const inboundTrustTiers = [
  {
    description: 'Ignore all inbound mesh evidence until explicitly enabled.',
    id: 'disabled',
    label: 'Disabled',
  },
  {
    description: 'Apply evidence only from directly trusted peers and contacts.',
    id: 'trusted',
    label: 'Trusted peers',
  },
  {
    description: 'Apply trusted peer and realm-curated evidence with provenance.',
    id: 'realm',
    label: 'Trusted realms',
  },
];

export const outboundEvidenceTypes = [
  {
    description: 'Signed statements that a known hash verified locally.',
    id: 'hashVerification',
    label: 'Hash verification',
  },
  {
    description: 'Signed release-completeness observations without file paths.',
    id: 'releaseCompleteness',
    label: 'Release completeness',
  },
  {
    description: 'Signed warnings for suspicious fake-lossless candidates.',
    id: 'fakeLosslessWarning',
    label: 'Fake-lossless warnings',
  },
  {
    description: 'Signed metadata corrections reviewed by this operator.',
    id: 'metadataCorrection',
    label: 'Metadata corrections',
  },
  {
    description: 'Realm-curated subject index entries, never raw library dumps.',
    id: 'realmSubjectIndex',
    label: 'Realm subject indexes',
  },
];

export const defaultMeshEvidencePolicy = {
  evidenceReview: {
    minimumConfidence: 0.7,
    minimumWitnesses: 2,
  },
  inboundTrustTier: 'disabled',
  outbound: outboundEvidenceTypes.reduce((state, evidenceType) => {
    state[evidenceType.id] = false;
    return state;
  }, {}),
  provenanceRequired: true,
  updatedAt: null,
};

const sanitizePolicy = (policy = {}) => {
  const inboundTrustTier = inboundTrustTiers.some(
    (tier) => tier.id === policy.inboundTrustTier,
  )
    ? policy.inboundTrustTier
    : defaultMeshEvidencePolicy.inboundTrustTier;

  const outbound = outboundEvidenceTypes.reduce((state, evidenceType) => {
    state[evidenceType.id] = policy.outbound?.[evidenceType.id] === true;
    return state;
  }, {});

  return {
    evidenceReview: {
      minimumConfidence: Number.isFinite(policy.evidenceReview?.minimumConfidence)
        ? Math.min(Math.max(policy.evidenceReview.minimumConfidence, 0), 1)
        : defaultMeshEvidencePolicy.evidenceReview.minimumConfidence,
      minimumWitnesses: Number.isFinite(policy.evidenceReview?.minimumWitnesses)
        ? Math.max(Math.round(policy.evidenceReview.minimumWitnesses), 1)
        : defaultMeshEvidencePolicy.evidenceReview.minimumWitnesses,
    },
    inboundTrustTier,
    outbound,
    provenanceRequired: true,
    updatedAt:
      typeof policy.updatedAt === 'string'
        ? policy.updatedAt.trim().slice(0, maxEvidenceTextCharacters) || null
        : null,
  };
};

export const getMeshEvidencePolicy = () => {
  const stored = readBoundedJson(
    getLocalStorageItem,
    meshEvidencePolicyStorageKey,
    null,
    maxMeshPolicyStorageCharacters,
  );
  return stored && typeof stored === 'object' && !Array.isArray(stored)
    ? sanitizePolicy(stored)
    : defaultMeshEvidencePolicy;
};

export const saveMeshEvidencePolicy = (policy) => {
  const sanitized = sanitizePolicy({
    ...policy,
    updatedAt: new Date().toISOString(),
  });

  setLocalStorageItem(meshEvidencePolicyStorageKey, JSON.stringify(sanitized));
  return sanitized;
};

export const setMeshEvidenceInboundTrustTier = (inboundTrustTier) =>
  saveMeshEvidencePolicy({
    ...getMeshEvidencePolicy(),
    inboundTrustTier,
  });

export const setMeshEvidenceOutboundEnabled = (evidenceTypeId, enabled) => {
  const policy = getMeshEvidencePolicy();
  return saveMeshEvidencePolicy({
    ...policy,
    outbound: {
      ...policy.outbound,
      [evidenceTypeId]: enabled === true,
    },
  });
};

export const resetMeshEvidencePolicy = () => {
  removeLocalStorageItem(meshEvidencePolicyStorageKey);
  return defaultMeshEvidencePolicy;
};

export const getMeshEvidencePolicySummary = (policy = getMeshEvidencePolicy()) => {
  const enabledOutbound = outboundEvidenceTypes.filter(
    (evidenceType) => policy.outbound[evidenceType.id],
  );
  const inboundTier = inboundTrustTiers.find(
    (tier) => tier.id === policy.inboundTrustTier,
  );

  return {
    enabledOutbound,
    inboundEnabled: policy.inboundTrustTier !== 'disabled',
    inboundTier,
    outboundEnabled: enabledOutbound.length > 0,
    privateByDefault:
      policy.inboundTrustTier === 'disabled' && enabledOutbound.length === 0,
  };
};

const trustedInboundTiers = {
  disabled: new Set(),
  realm: new Set(['trusted', 'realm']),
  trusted: new Set(['trusted']),
};

const normalizeEvidenceEntries = (entries) => {
  if (Array.isArray(entries)) return entries.slice(0, maxEvidenceReviewEntries);
  if (Array.isArray(entries?.evidence)) {
    return entries.evidence.slice(0, maxEvidenceReviewEntries);
  }
  if (Array.isArray(entries?.items)) {
    return entries.items.slice(0, maxEvidenceReviewEntries);
  }
  return [];
};

const normalizeEvidenceText = (value, fallback = '') =>
  typeof value === 'string' || typeof value === 'number'
    ? String(value).trim().slice(0, maxEvidenceTextCharacters)
    : fallback;

const normalizeEvidenceEntry = (entry = {}) => {
  const source = entry && typeof entry === 'object' && !Array.isArray(entry)
    ? entry
    : {};
  const type = normalizeEvidenceText(source.type, 'unknown');
  const subject = normalizeEvidenceText(source.subject, 'unknown');
  const confidence = Number(source.confidence);
  const witnessCount = Number(source.witnessCount);

  return {
    confidence: Number.isFinite(confidence) ? Math.min(Math.max(confidence, 0), 1) : 0,
    containsExactHoldings: source.containsExactHoldings === true,
    containsPath: source.containsPath === true,
    containsRawListeningHistory: source.containsRawListeningHistory === true,
    id: normalizeEvidenceText(source.id, `${type}:${subject}`),
    provenance: {
      peerId: normalizeEvidenceText(source.provenance?.peerId || source.peerId),
      realmId: normalizeEvidenceText(source.provenance?.realmId || source.realmId),
      signature: normalizeEvidenceText(source.provenance?.signature || source.signature),
      trustTier: normalizeEvidenceText(
        source.provenance?.trustTier || source.trustTier,
        'untrusted',
      ),
    },
    subject,
    type,
    witnessCount: Number.isFinite(witnessCount) ? Math.max(Math.floor(witnessCount), 0) : 0,
  };
};

export const evaluateMeshEvidenceEntries = (
  entries,
  policy = getMeshEvidencePolicy(),
) => {
  const sanitizedPolicy = sanitizePolicy(policy);
  const trustedTiers = trustedInboundTiers[sanitizedPolicy.inboundTrustTier];
  const normalized = normalizeEvidenceEntries(entries).map(normalizeEvidenceEntry);

  const results = normalized.map((entry) => {
    const reasons = [];

    if (sanitizedPolicy.inboundTrustTier === 'disabled') {
      reasons.push('inbound mesh evidence disabled');
    }

    if (!entry.provenance.signature || !entry.provenance.peerId) {
      reasons.push('missing signed provenance');
    }

    if (!trustedTiers.has(entry.provenance.trustTier)) {
      reasons.push(`untrusted provenance tier: ${entry.provenance.trustTier}`);
    }

    if (entry.confidence < sanitizedPolicy.evidenceReview.minimumConfidence) {
      reasons.push(`confidence below ${sanitizedPolicy.evidenceReview.minimumConfidence}`);
    }

    if (entry.witnessCount < sanitizedPolicy.evidenceReview.minimumWitnesses) {
      reasons.push(`witness count below ${sanitizedPolicy.evidenceReview.minimumWitnesses}`);
    }

    if (entry.containsPath) {
      reasons.push('contains raw path data');
    }

    if (entry.containsExactHoldings) {
      reasons.push('contains exact local holdings');
    }

    if (entry.containsRawListeningHistory) {
      reasons.push('contains raw listening history');
    }

    return {
      ...entry,
      accepted: reasons.length === 0,
      reasons,
    };
  });

  const accepted = results.filter((entry) => entry.accepted);

  return {
    accepted,
    rejected: results.filter((entry) => !entry.accepted),
    results,
    summary: {
      accepted: accepted.length,
      rejected: results.length - accepted.length,
      total: results.length,
    },
  };
};

export const parseMeshEvidenceReviewInput = (value) => {
  if (typeof value !== 'string' || !value.trim()) return [];
  if (value.length > maxEvidenceReviewInputCharacters) return [];

  const parsed = JSON.parse(value);
  return normalizeEvidenceEntries(parsed);
};

export const formatMeshEvidenceReviewReport = (review) => {
  const lines = [
    'slskr mesh evidence review',
    `Total: ${review.summary.total}`,
    `Accepted: ${review.summary.accepted}`,
    `Rejected: ${review.summary.rejected}`,
    '',
  ];

  review.results.forEach((entry) => {
    lines.push(
      `[${entry.accepted ? 'ACCEPT' : 'REJECT'}] ${entry.type} ${entry.subject}`,
    );
    lines.push(`Confidence: ${entry.confidence}`);
    lines.push(`Witnesses: ${entry.witnessCount}`);
    lines.push(`Provenance: ${entry.provenance.trustTier} ${entry.provenance.peerId}`);
    if (entry.reasons.length > 0) {
      lines.push(`Reasons: ${entry.reasons.join('; ')}`);
    }
    lines.push('');
  });

  return lines.join('\n').trim();
};
