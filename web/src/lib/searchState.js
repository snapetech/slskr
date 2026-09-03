const normalizeState = (value) => String(value ?? '').trim().toLowerCase();

const stateTokens = (search = {}) =>
  [search.status, search.state]
    .flatMap((value) => normalizeState(value).split(','))
    .map((value) => value.trim())
    .filter(Boolean);

export const getSearchStateKind = (search = {}) => {
  const tokens = stateTokens(search);

  if (tokens.some((token) => ['failed', 'errored', 'error'].includes(token))) {
    return 'failed';
  }

  if (tokens.some((token) => ['cancelled', 'canceled', 'expired'].includes(token))) {
    return 'cancelled';
  }

  if (
    search.isComplete === true ||
    tokens.some((token) =>
      [
        'completed',
        'complete',
        'timedout',
        'timed out',
        'responselimitreached',
        'response limit reached',
        'filelimitreached',
        'file limit reached',
      ].includes(token),
    )
  ) {
    return 'completed';
  }

  if (
    tokens.some((token) =>
      [
        'none',
        'queued',
        'requested',
        'inprogress',
        'in progress',
        'active',
        'pending',
      ].includes(token),
    )
  ) {
    return 'active';
  }

  return 'unknown';
};

export const isSearchComplete = (search = {}) =>
  ['completed', 'cancelled', 'failed'].includes(getSearchStateKind(search));

export const parseSearchDate = (value) => {
  if (value === null || value === undefined || value === '') {
    return null;
  }

  if (value instanceof Date) {
    return Number.isNaN(value.getTime()) ? null : value;
  }

  const text = String(value).trim();
  const numeric = /^[+-]?(?:\d+\.?\d*|\.\d+)$/u.test(text)
    ? Number(text)
    : Number.NaN;

  if (Number.isFinite(numeric)) {
    const milliseconds = Math.abs(numeric) < 100_000_000_000
      ? numeric * 1000
      : numeric;
    const date = new Date(milliseconds);
    return Number.isNaN(date.getTime()) ? null : date;
  }

  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
};

export const searchDateValue = (value) =>
  parseSearchDate(value)?.getTime() ?? 0;

export const formatSearchTime = (value) =>
  parseSearchDate(value)?.toLocaleTimeString() ?? '-';
