import { encodePathSegment } from './pathEncoding';

const parseIndex = (value) => {
  if (typeof value !== 'number' && typeof value !== 'string') {
    return null;
  }

  if (typeof value === 'string' && value.trim() === '') {
    return null;
  }

  const index = Number(value);
  return Number.isSafeInteger(index) && index >= 0 ? index : null;
};

const filesFor = (response, key) =>
  Array.isArray(response?.[key]) ? response[key] : [];

const findFileIndex = (files, filename) =>
  files.findIndex((file) => file?.filename === filename);

// Search responses are grouped by peer for display, while action routes use
// the stable flat SearchRecord result index. Older payloads lack resultIndex,
// so retain a safe response/file fallback for compatibility.
export const getSearchResultItemId = ({
  file,
  response = {},
  responseIndex = 0,
} = {}) => {
  const resultIndex = parseIndex(file?.resultIndex ?? file?.result_index);
  if (resultIndex !== null) {
    return `${resultIndex}:0`;
  }

  const files = filesFor(response, 'files');
  const lockedFiles = filesFor(response, 'lockedFiles');
  const preferredFiles = file?.locked ? lockedFiles : files;
  const preferredIndex = findFileIndex(preferredFiles, file?.filename);

  if (preferredIndex >= 0) {
    const offset = file?.locked ? files.length : 0;
    return `${parseIndex(responseIndex) ?? 0}:${offset + preferredIndex}`;
  }

  const fallbackIndex = findFileIndex(
    file?.locked ? files : lockedFiles,
    file?.filename,
  );
  if (fallbackIndex < 0) {
    return null;
  }

  const offset = file?.locked ? 0 : files.length;
  return `${parseIndex(responseIndex) ?? 0}:${offset + fallbackIndex}`;
};

export const buildSearchResultActionPath = (searchId, itemId, action) =>
  `/searches/${encodePathSegment(searchId)}/items/${encodePathSegment(itemId)}/${action}`;
