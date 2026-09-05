import api from './api';

const asObject = (value) =>
  value && typeof value === 'object' && !Array.isArray(value) ? value : {};
const isPlainObject = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);
const maxSavedDiscoveryGraphBranches = 12;
const maxSavedDiscoveryGraphTextCharacters = 2_048;
const maxSavedDiscoveryGraphStorageCharacters = 64 * 1024;
const savedDiscoveryGraphRequestKeys = [
  'album',
  'artist',
  'artistId',
  'compareLabel',
  'compareNodeId',
  'recordingId',
  'releaseId',
  'scope',
  'songIdRunId',
  'title',
];

const normalizeSavedDiscoveryGraphText = (value) =>
  typeof value === 'string' || typeof value === 'number'
    ? String(value).trim().slice(0, maxSavedDiscoveryGraphTextCharacters)
    : '';

const normalizeSavedDiscoveryGraphRequest = (request) => {
  if (!isPlainObject(request)) return null;

  const normalized = Object.fromEntries(
    savedDiscoveryGraphRequestKeys
      .map((key) => [key, normalizeSavedDiscoveryGraphText(request[key])])
      .filter(([, value]) => value),
  );
  return Object.keys(normalized).length > 0 ? normalized : null;
};

export const buildDiscoveryGraph = async (request) => {
  const response = await api.post('/discovery-graph', request);
  if (
    !response.data ||
    typeof response.data !== 'object' ||
    Array.isArray(response.data)
  ) {
    throw new Error('Discovery graph API returned an invalid graph response');
  }
  return response.data;
};

export const toQueryString = (request = {}) => {
  const parameters = new URLSearchParams();

  Object.entries(request).forEach(([key, value]) => {
    if (value !== undefined && value !== null && `${value}`.trim() !== '') {
      parameters.set(key, value);
    }
  });

  return parameters.toString();
};

export const fromQueryString = (search = '') => {
  const parameters = new URLSearchParams(search.startsWith('?') ? search : `?${search}`);
  return {
    album: parameters.get('album') || undefined,
    artist: parameters.get('artist') || undefined,
    artistId: parameters.get('artistId') || undefined,
    compareLabel: parameters.get('compareLabel') || undefined,
    compareNodeId: parameters.get('compareNodeId') || undefined,
    recordingId: parameters.get('recordingId') || undefined,
    releaseId: parameters.get('releaseId') || undefined,
    scope: parameters.get('scope') || 'songid_run',
    songIdRunId: parameters.get('songIdRunId') || undefined,
    title: parameters.get('title') || undefined,
  };
};

export const normalizeSavedDiscoveryGraphBranch = (branch) => {
  if (!isPlainObject(branch)) return null;

  const id = normalizeSavedDiscoveryGraphText(branch.id);
  const title = normalizeSavedDiscoveryGraphText(branch.title);
  if (!id || !title) return null;

  const normalized = { id, title };
  const savedAt = normalizeSavedDiscoveryGraphText(branch.savedAt);
  const seedNodeId = normalizeSavedDiscoveryGraphText(branch.seedNodeId);
  const request = normalizeSavedDiscoveryGraphRequest(branch.request);
  if (savedAt) normalized.savedAt = savedAt;
  if (seedNodeId) normalized.seedNodeId = seedNodeId;
  if (request) normalized.request = request;
  return normalized;
};

export const parseSavedDiscoveryGraphBranches = (raw) => {
  if (typeof raw !== 'string' || raw.length > maxSavedDiscoveryGraphStorageCharacters) {
    return [];
  }

  try {
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed)
      ? parsed
          .slice(0, maxSavedDiscoveryGraphBranches)
          .map(normalizeSavedDiscoveryGraphBranch)
          .filter(Boolean)
      : [];
  } catch {
    return [];
  }
};

export const getSavedDiscoveryGraphLimits = () => ({
  maxBranches: maxSavedDiscoveryGraphBranches,
  maxCharacters: maxSavedDiscoveryGraphStorageCharacters,
});

export const getDiscoveryGraphNodes = (graph) =>
  Array.isArray(graph?.nodes) ? graph.nodes.filter(isPlainObject) : [];

export const getDiscoveryGraphEdges = (graph) =>
  Array.isArray(graph?.edges) ? graph.edges.filter(isPlainObject) : [];

export const getVisibleDiscoveryGraph = ({
  edgeTypes = [],
  graph,
  maxDepth = 99,
  minNodeWeight = 0,
} = {}) => {
  const nodes = getDiscoveryGraphNodes(graph);
  const visibleNodes = nodes.filter(
    (node) => (node.depth || 0) <= maxDepth && (node.weight || 0) >= minNodeWeight,
  );
  const visibleNodeIds = new Set(visibleNodes.map((node) => node.nodeId));
  const visibleEdges = getDiscoveryGraphEdges(graph).filter(
    (edge) =>
      visibleNodeIds.has(edge.sourceNodeId) &&
      visibleNodeIds.has(edge.targetNodeId) &&
      (edgeTypes.length === 0 || edgeTypes.includes(edge.edgeType)),
  );

  return {
    edgeTypeCounts: summarizeCounts(visibleEdges, 'edgeType'),
    nodeTypeCounts: summarizeCounts(visibleNodes, 'nodeType'),
    visibleEdges,
    visibleNodes,
  };
};

export const buildDiscoveryGraphBranchPlan = ({
  edgeTypes = [],
  graph,
  maxDepth = 99,
  minNodeWeight = 0,
  pinnedNode = null,
} = {}) => {
  const { edgeTypeCounts, nodeTypeCounts, visibleEdges, visibleNodes } =
    getVisibleDiscoveryGraph({
      edgeTypes,
      graph,
      maxDepth,
      minNodeWeight,
    });
  const nodeMap = visibleNodes.reduce((accumulator, node) => {
    accumulator[node.nodeId] = node;
    return accumulator;
  }, {});
  const routeSuggestions = visibleEdges
    .map((edge) => ({
      edge,
      score:
        (edge.weight || 0) +
        ((nodeMap[edge.targetNodeId]?.weight || 0) * 0.5) -
        ((nodeMap[edge.targetNodeId]?.depth || 0) * 0.05),
      source: nodeMap[edge.sourceNodeId],
      target: nodeMap[edge.targetNodeId],
    }))
    .filter((route) => route.source && route.target)
    .sort((left, right) => right.score - left.score)
    .slice(0, 8);
  const searchQueries = visibleNodes
    .filter((node) => node.nodeType === 'track')
    .map((node) => node.label || '')
    .filter(Boolean)
    .slice(0, 10);

  return {
    activeEdgeTypes: edgeTypes,
    edgeTypeCounts,
    evidenceSummary: Array.isArray(graph?.evidenceSummary)
      ? graph.evidenceSummary
      : [],
    graphTitle: graph?.title || 'Discovery Graph',
    maxDepth,
    minNodeWeight,
    nodeTypeCounts,
    pinnedNode,
    routeSuggestions,
    searchQueries,
    seedNodeId: graph?.seedNodeId || '',
    summary: graph?.summary || '',
    visibleEdgeCount: visibleEdges.length,
    visibleNodeCount: visibleNodes.length,
  };
};

export const formatDiscoveryGraphBranchReport = (plan) => {
  const lines = [
    'slskr Discovery Graph branch review',
    `Graph: ${plan.graphTitle}`,
    `Seed: ${plan.seedNodeId || 'unknown'}`,
    `Visible nodes: ${plan.visibleNodeCount}`,
    `Visible edges: ${plan.visibleEdgeCount}`,
    `Depth: ${plan.maxDepth}`,
    `Minimum weight: ${Math.round((plan.minNodeWeight || 0) * 100)}`,
  ];

  if (plan.pinnedNode?.nodeId) {
    lines.push(`Pinned comparison: ${plan.pinnedNode.label || plan.pinnedNode.nodeId}`);
  }

  lines.push('', 'Node families:');
  appendCounts(lines, plan.nodeTypeCounts);
  lines.push('', 'Edge families:');
  appendCounts(lines, plan.edgeTypeCounts);

  if (plan.routeSuggestions.length > 0) {
    lines.push('', 'Suggested branch routes:');
    plan.routeSuggestions.forEach((route) => {
      lines.push(
        `- ${route.source.label} -> ${route.target.label} ` +
          `[${route.edge.edgeType}, ${Math.round((route.edge.weight || 0) * 100)}] ` +
          `${route.edge.reason || ''}`.trim(),
      );
    });
  }

  if (plan.evidenceSummary.length > 0) {
    lines.push('', 'Evidence lanes:');
    plan.evidenceSummary
      .slice()
      .sort((left, right) => (right.score || 0) - (left.score || 0))
      .forEach((lane) => {
        lines.push(
          `- ${lane.label || lane.lane}: ${Math.round((lane.score || 0) * 100)} ` +
            `(${lane.count || 0}) ${lane.summary || ''}`.trim(),
        );
      });
  }

  if (plan.searchQueries.length > 0) {
    lines.push('', 'Nearby search seeds:');
    plan.searchQueries.forEach((query) => lines.push(`- ${query}`));
  }

  return lines.join('\n');
};

const summarizeCounts = (items, property) =>
  items.reduce((accumulator, item) => {
    const key = item[property] || 'unknown';
    accumulator[key] = (accumulator[key] || 0) + 1;
    return accumulator;
  }, {});

const appendCounts = (lines, counts) => {
  const entries = Object.entries(asObject(counts));
  if (entries.length === 0) {
    lines.push('- none visible');
    return;
  }

  entries
    .sort(([left], [right]) => left.localeCompare(right))
    .forEach(([key, count]) => lines.push(`- ${key}: ${count}`));
};
