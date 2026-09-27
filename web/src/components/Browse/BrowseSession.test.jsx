import BrowseSession, {
  getBrowseErrorMessage,
  normalizeDirectories,
} from './BrowseSession';
import { render, screen } from '@testing-library/react';

describe('BrowseSession controls', () => {
  it('exposes an accessible browse action', () => {
    render(<BrowseSession />);

    expect(
      screen.getByRole('button', { name: 'Browse user files' }),
    ).toBeInTheDocument();
  });
});

describe('BrowseSession errors', () => {
  it('uses bounded text returned by the daemon', () => {
    expect(
      getBrowseErrorMessage({
        response: { data: 'Unable to browse user; the remote peer is unavailable' },
      }),
    ).toBe('Unable to browse user; the remote peer is unavailable');
  });

  it('uses structured API details and ordinary errors', () => {
    expect(
      getBrowseErrorMessage({ response: { data: { detail: 'Reconnect first' } } }),
    ).toBe('Reconnect first');
    expect(getBrowseErrorMessage(new Error('Browse timed out'))).toBe(
      'Browse timed out',
    );
  });

  it('does not pass structured error objects into React children', () => {
    expect(
      getBrowseErrorMessage({
        response: { data: { detail: { reason: 'nested' }, message: 'Safe text' } },
      }),
    ).toBe('Safe text');
    expect(
      getBrowseErrorMessage({ response: { data: { detail: { reason: 'nested' } } } }),
    ).toBe('Browse failed');
  });
});

describe('BrowseSession tree bounds', () => {
  it('normalizes malformed records and bounds deep, wide, and file-heavy trees', () => {
    const deepTree = { name: 'Depth 0' };
    let current = deepTree;
    for (let depth = 1; depth <= 80; depth += 1) {
      current.children = [{ name: `Depth ${depth}` }];
      current = current.children[0];
    }

    const normalized = normalizeDirectories([
      null,
      42,
      { children: [{ name: 'missing parent' }] },
      {
        name: 'Wide root',
        fileCount: 'not a number',
        files: [
          ...Array.from({ length: 2_005 }, (_, index) => ({
            filename: `track-${index}.mp3`,
            size: index,
          })),
          { filename: '' },
        ],
        children: Array.from({ length: 10_005 }, (_, index) => ({
          name: `Child ${index}`,
        })),
      },
    ]);

    const normalizedDeepTree = normalizeDirectories([deepTree]);

    expect(normalized).toHaveLength(1);
    expect(normalized[0].fileCount).toBe(0);
    expect(normalized[0].files).toHaveLength(2_000);
    expect(normalized[0].children).toHaveLength(9_998);
    expect(normalizedDeepTree).toHaveLength(1);
    expect(normalizedDeepTree[0].children).toHaveLength(1);
    expect(normalizedDeepTree[0].children[0].children).toHaveLength(1);
    expect(normalizedDeepTree[0].children[0].children[0].children).toHaveLength(1);
    const depth = (node) => 1 + (node.children?.[0] ? depth(node.children[0]) : 0);
    expect(depth(normalizedDeepTree[0])).toBe(65);
    expect(normalized[0].children.every(Boolean)).toBe(true);
  });

  it('builds a wide parent-indexed tree without losing children', () => {
    const session = new BrowseSession({});
    const directories = [
      { name: 'Library', fileCount: 0 },
      ...Array.from({ length: 1_500 }, (_, index) => ({
        name: `Library\\Album ${index}`,
        fileCount: 1,
      })),
    ];

    const tree = session.getDirectoryTree({ directories, separator: '\\' });

    expect(tree).toHaveLength(1);
    expect(tree[0].name).toBe('Library');
    expect(tree[0].children).toHaveLength(1_500);
    expect(tree[0].children[1_499].name).toBe('Library\\Album 1499');
  });
});
