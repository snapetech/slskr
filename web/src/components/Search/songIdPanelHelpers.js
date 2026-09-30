const getSyntheticVerdictColor = (verdict) => {
  switch (verdict) {
    case 'strong_suspicion':
      return 'orange';
    case 'moderate_suspicion':
      return 'yellow';
    case 'mixed_or_inconclusive':
      return 'grey';
    case 'low_signal':
      return 'green';
    default:
      return undefined;
  }
};

const getIdentityVerdictColor = (verdict) => {
  switch (verdict) {
    case 'recognized_cataloged_track':
      return 'green';
    case 'candidate_match_found':
      return 'blue';
    case 'likely_ai_or_channel_original':
      return 'orange';
    case 'needs_manual_review':
      return 'yellow';
    default:
      return undefined;
  }
};

const formatPercent = (value) => Math.round((value || 0) * 100);

const getStatusColor = (status) => {
  switch (status) {
    case 'completed':
      return 'green';
    case 'failed':
      return 'red';
    case 'running':
      return 'blue';
    case 'queued':
      return 'grey';
    default:
      return undefined;
  }
};

const getRunTitle = (item) =>
  item?.metadata?.title ||
  item?.query ||
  item?.summary ||
  item?.sourceType ||
  'SongID run';

const getTrackKey = (candidate) =>
  `${(candidate?.artist || '').trim().toLowerCase()}::${(candidate?.title || '').trim().toLowerCase()}`;

const getDedupedTracks = (tracks) => {
  if (!Array.isArray(tracks)) {
    return [];
  }

  const map = new Map();
  tracks.forEach((candidate) => {
    const key = getTrackKey(candidate);
    const current = map.get(key);
    if (!current || (candidate.actionScore || 0) > (current.actionScore || 0)) {
      map.set(key, {
        ...candidate,
        duplicateCount: (current?.duplicateCount || 0) + 1,
      });
    } else {
      current.duplicateCount = (current.duplicateCount || 1) + 1;
    }
  });

  return Array.from(map.values()).sort(
    (left, right) => (right.actionScore || 0) - (left.actionScore || 0),
  );
};

const getOptionKey = (option) => [
  option?.actionKind || '',
  option?.scope || '',
  option?.mode || '',
  option?.targetId || '',
  option?.searchText || '',
  Array.isArray(option?.searchTexts) ? option.searchTexts.join('|') : '',
].join('::');

const getDedupedOptions = (options) => {
  if (!Array.isArray(options)) {
    return [];
  }

  const map = new Map();
  options.forEach((option) => {
    const key = getOptionKey(option);
    const current = map.get(key);
    if (!current || (option.overallScore || 0) > (current.overallScore || 0)) {
      map.set(key, {
        ...option,
        duplicateCount: (current?.duplicateCount || 0) + 1,
      });
    } else {
      current.duplicateCount = (current.duplicateCount || 1) + 1;
    }
  });

  return Array.from(map.values()).sort(
    (left, right) => (right.overallScore || 0) - (left.overallScore || 0),
  );
};

const getPlanKey = (plan) => [
  plan?.kind || '',
  plan?.targetId || '',
  plan?.profile || '',
  plan?.searchText || '',
  plan?.title || '',
].join('::');

const getDedupedPlans = (plans) => {
  if (!Array.isArray(plans)) {
    return [];
  }

  const map = new Map();
  plans.forEach((plan) => {
    const key = getPlanKey(plan);
    const current = map.get(key);
    if (!current || (plan.actionScore || 0) > (current.actionScore || 0)) {
      map.set(key, {
        ...plan,
        duplicateCount: (current?.duplicateCount || 0) + 1,
      });
    } else {
      current.duplicateCount = (current.duplicateCount || 1) + 1;
    }
  });

  return Array.from(map.values()).sort(
    (left, right) => (right.actionScore || 0) - (left.actionScore || 0),
  );
};

const getUniqueTopActions = (actions, limit) => {
  const seen = new Set();
  return actions.filter((action) => {
    const key = (action.label || '').toLowerCase();
    if (seen.has(key)) {
      return false;
    }

    seen.add(key);
    return true;
  }).slice(0, limit);
};

const hasScorecardSignal = (scorecard) => scorecard && [
  'clipCount',
  'acoustIdHitCount',
  'rawAcoustIdHitCount',
  'songRecHitCount',
  'songRecDistinctMatchCount',
  'transcriptCount',
  'ocrCount',
  'commentFindingCount',
  'timestampHintCount',
  'chapterHintCount',
  'playlistRequestCount',
  'aiCommentMentionCount',
  'panakoHitCount',
  'audfprintHitCount',
  'corpusMatchCount',
  'provenanceSignalCount',
  'aiArtifactClipCount',
].some((key) => (scorecard[key] || 0) > 0);

const detailStyle = {
  borderTop: '1px solid rgba(34, 36, 38, 0.15)',
  marginTop: '1em',
  paddingTop: '0.75em',
};

const detailSummaryStyle = {
  cursor: 'pointer',
  fontWeight: 600,
};


export {
  getSyntheticVerdictColor,
  getIdentityVerdictColor,
  formatPercent,
  getStatusColor,
  getRunTitle,
  getDedupedTracks,
  getDedupedOptions,
  getDedupedPlans,
  getUniqueTopActions,
  hasScorecardSignal,
  detailStyle,
  detailSummaryStyle,
};
