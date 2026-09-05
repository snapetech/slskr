import * as collectionsAPI from '../../lib/collections';
import Collections from './Collections';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/collections', () => ({
  getCollectionItems: vi.fn(),
  getCollections: vi.fn(),
  getSharesByCollection: vi.fn(),
  getShareGroups: vi.fn(),
}));

describe('Collections', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    collectionsAPI.getShareGroups.mockResolvedValue({ data: [] });
    collectionsAPI.getCollectionItems.mockResolvedValue({ data: [] });
    collectionsAPI.getSharesByCollection.mockResolvedValue({ data: [] });
  });

  it('shows authentication failures instead of an empty collection state', async () => {
    collectionsAPI.getCollections.mockRejectedValue(new Error('not authorized'));

    render(<Collections />);

    await waitFor(() => {
      expect(screen.getByTestId('collections-load-error')).toHaveTextContent(
        'not authorized',
      );
    });
    expect(screen.getByTestId('collections-load-error')).toHaveTextContent(
      'Collections unavailable',
    );
    expect(screen.queryByText('No collections yet')).not.toBeInTheDocument();
  });

  it('reports collection-item failures instead of showing an empty success state', async () => {
    collectionsAPI.getCollections.mockResolvedValue({
      data: [{ id: 'collection-1', title: 'Test collection', type: 'Playlist' }],
    });
    collectionsAPI.getCollectionItems.mockRejectedValue(
      new Error('Collection item service unavailable'),
    );

    render(<Collections />);

    fireEvent.click(await screen.findByTestId('collection-row-Test collection'));

    expect(await screen.findByTestId('collection-items-load-error')).toHaveTextContent(
      'Collection item service unavailable',
    );
    expect(screen.getByText('Collection items unavailable')).toBeInTheDocument();
    expect(screen.queryByText('No items in this collection yet.')).not.toBeInTheDocument();
  });

  it('reports collection-share failures instead of showing an empty success state', async () => {
    collectionsAPI.getCollections.mockResolvedValue({
      data: [{ id: 'collection-1', title: 'Test collection', type: 'Playlist' }],
    });
    collectionsAPI.getSharesByCollection.mockRejectedValue(
      new Error('Collection share service unavailable'),
    );

    render(<Collections />);

    fireEvent.click(await screen.findByTestId('collection-row-Test collection'));

    expect(await screen.findByTestId('collection-shares-load-error')).toHaveTextContent(
      'Collection share service unavailable',
    );
    expect(screen.getByText('Collection shares unavailable')).toBeInTheDocument();
    expect(screen.queryByText('No shares yet.')).not.toBeInTheDocument();
  });
});
