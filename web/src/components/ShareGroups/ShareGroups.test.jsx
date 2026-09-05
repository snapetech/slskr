import * as collectionsAPI from '../../lib/collections';
import * as identityAPI from '../../lib/identity';
import ShareGroups from './ShareGroups';
import { render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/collections', () => ({
  getShareGroups: vi.fn(),
}));

vi.mock('../../lib/identity', () => ({
  getContacts: vi.fn(),
}));

describe('ShareGroups', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    identityAPI.getContacts.mockResolvedValue({ data: [] });
  });

  it('shows authorization failures instead of an empty group state', async () => {
    collectionsAPI.getShareGroups.mockRejectedValue(new Error('forbidden'));

    render(<ShareGroups />);

    await waitFor(() => {
      expect(screen.getByText(/forbidden/i)).toBeInTheDocument();
    });
    expect(screen.queryByText('No share groups yet')).not.toBeInTheDocument();
  });
});
