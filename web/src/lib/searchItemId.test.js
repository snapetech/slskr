import {
  buildSearchResultActionPath,
  getSearchResultItemId,
} from './searchItemId';

describe('getSearchResultItemId', () => {
  it('uses the stable backend result index for reordered responses', () => {
    expect(
      getSearchResultItemId({
        file: { filename: 'locked.flac', resultIndex: 7 },
        responseIndex: 0,
      }),
    ).toBe('7:0');
  });

  it('offsets locked files when handling legacy payloads', () => {
    const response = {
      files: [{ filename: 'open.flac' }],
      lockedFiles: [{ filename: 'locked.flac' }],
    };

    expect(
      getSearchResultItemId({
        file: { filename: 'locked.flac', locked: true },
        response,
        responseIndex: 3,
      }),
    ).toBe('3:1');
  });

  it('returns null when the selected file is not in the response', () => {
    expect(
      getSearchResultItemId({
        file: { filename: 'missing.flac' },
        response: { files: [] },
      }),
    ).toBeNull();
  });
});

describe('buildSearchResultActionPath', () => {
  it('encodes search and item identifiers as route segments', () => {
    expect(
      buildSearchResultActionPath('bridge/search', '0:3', 'download'),
    ).toBe('/searches/bridge%2Fsearch/items/0%3A3/download');
  });
});
