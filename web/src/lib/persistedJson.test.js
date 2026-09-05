import {
  maxPersistedJsonCharacters,
  readBoundedJson,
  writeBoundedList,
  writeBoundedObject,
} from './persistedJson';

describe('persistedJson', () => {
  it('rejects oversized JSON before parsing', () => {
    expect(
      readBoundedJson(
        () => 'x'.repeat(maxPersistedJsonCharacters + 1),
        'state',
        [],
      ),
    ).toEqual([]);
  });

  it('keeps the newest list entries within item and byte limits', () => {
    let saved = '';
    const result = writeBoundedList(
      (_key, value) => {
        saved = value;
      },
      'state',
      [{ id: 1 }, { id: 2 }, { id: 3 }],
      { maxItems: 2, maxCharacters: 100 },
    );

    expect(result).toEqual([{ id: 1 }, { id: 2 }]);
    expect(JSON.parse(saved)).toEqual(result);
  });

  it('keeps the newest object entries within the entry limit', () => {
    let saved = '';
    const result = writeBoundedObject(
      (_key, value) => {
        saved = value;
      },
      'state',
      { first: 1, second: 2, third: 3 },
      { maxEntries: 2 },
    );

    expect(result).toEqual({ second: 2, third: 3 });
    expect(JSON.parse(saved)).toEqual(result);
  });
});
