import * as collectionsAPI from '../../lib/collections';
import SharedWithMe from './SharedWithMe';
import { render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/collections', () => ({
  getIncomingShares: vi.fn(),
  shareGrantAllows: vi.fn(() => false),
}));

describe('SharedWithMe', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows authentication failures instead of an empty incoming-share state', async () => {
    collectionsAPI.getIncomingShares.mockRejectedValue(new Error('session expired'));

    render(<SharedWithMe />);

    await waitFor(() => {
      expect(screen.getByText(/session expired/i)).toBeInTheDocument();
    });
    expect(screen.getByText('No shares yet')).toBeInTheDocument();
  });
});
