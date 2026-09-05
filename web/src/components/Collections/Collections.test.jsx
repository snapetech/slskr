import * as collectionsAPI from '../../lib/collections';
import Collections from './Collections';
import { render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/collections', () => ({
  getCollections: vi.fn(),
  getShareGroups: vi.fn(),
}));

describe('Collections', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    collectionsAPI.getShareGroups.mockResolvedValue({ data: [] });
  });

  it('shows authentication failures instead of an empty collection state', async () => {
    collectionsAPI.getCollections.mockRejectedValue(new Error('not authorized'));

    render(<Collections />);

    await waitFor(() => {
      expect(screen.getByText(/not authorized/i)).toBeInTheDocument();
    });
    expect(screen.getByText('No collections yet')).toBeInTheDocument();
  });
});
