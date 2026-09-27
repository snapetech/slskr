import '@testing-library/jest-dom';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { axe } from 'jest-axe';
import { vi } from 'vitest';
import DirectoryTree from './DirectoryTree';

const treeFixture = [
  null,
  { children: [null, { name: 'Music\\Albums', fileCount: 2 }] },
  {
    children: [
      { name: 'Music\\Albums', fileCount: 2 },
      { name: 'Music\\Singles', fileCount: 1 },
    ],
    fileCount: 0,
    name: 'Music',
  },
  { name: 7 },
];

describe('DirectoryTree', () => {
  it('ignores malformed roots and exposes accessible expansion and selection controls', () => {
    render(
      <DirectoryTree
        onDownload={vi.fn()}
        onSelect={vi.fn()}
        tree={treeFixture}
      />,
    );

    expect(screen.getByText('Music')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Expand Music' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(screen.getByRole('checkbox', { name: 'Select Music' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Expand Music' }));

    expect(screen.getByText('Albums')).toBeInTheDocument();
    expect(screen.getByText('Singles')).toBeInTheDocument();
  });

  it('clears a stale selection when a same-sized replacement tree arrives', async () => {
    const onSelect = vi.fn();
    const { rerender } = render(
      <DirectoryTree
        onDownload={vi.fn()}
        onSelect={onSelect}
        tree={[{ name: 'Music', fileCount: 1 }]}
      />,
    );

    fireEvent.click(screen.getByRole('checkbox', { name: 'Select Music' }));
    expect(screen.getByRole('button', { name: 'Clear (1)' })).toBeInTheDocument();

    rerender(
      <DirectoryTree
        onDownload={vi.fn()}
        onSelect={onSelect}
        tree={[{ name: 'Books', fileCount: 1 }]}
      />,
    );

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: 'Clear (1)' })).not.toBeInTheDocument();
    });
    expect(screen.getByRole('checkbox', { name: 'Select Books' })).not.toBeChecked();
  });

  it('has no axe violations for expanded tree controls', async () => {
    const { container } = render(
      <DirectoryTree
        onDownload={vi.fn()}
        onSelect={vi.fn()}
        tree={treeFixture}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Expand Music' }));

    const results = await axe(container);
    expect(results.violations).toHaveLength(0);
  });
});
