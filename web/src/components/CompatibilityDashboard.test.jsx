import '@testing-library/jest-dom';
import CompatibilityDashboard from './CompatibilityDashboard';
import React from 'react';
import { MemoryRouter } from 'react-router-dom';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const reportMocks = vi.hoisted(() => ({
  getExceptionPareto: vi.fn(),
  getExceptions: vi.fn(),
  getHistogram: vi.fn(),
  getLeaderboard: vi.fn(),
  getSummary: vi.fn(),
  getTopDirectories: vi.fn(),
}));

const searchCreate = vi.hoisted(() => vi.fn());

vi.mock('../lib/reports', () => reportMocks);
vi.mock('../lib/searches', () => ({ create: searchCreate }));
vi.mock('react-toastify', () => ({ toast: { error: vi.fn(), info: vi.fn() } }));

describe('CompatibilityDashboard', () => {
  beforeEach(() => {
    reportMocks.getSummary.mockResolvedValue({
      Download: { Succeeded: { count: 2, distinctUsers: 1, totalBytes: 2_048 } },
      Upload: { Succeeded: { count: 1, distinctUsers: 1, totalBytes: 1_024 } },
    });
    reportMocks.getHistogram.mockResolvedValue({
      '2026-08-18T00:00:00Z': {
        Download: { Succeeded: { averageSpeed: 1_024, count: 2, totalBytes: 2_048 } },
        Upload: { Succeeded: { averageSpeed: 512, count: 1, totalBytes: 1_024 } },
      },
    });
    reportMocks.getLeaderboard.mockResolvedValue([]);
    reportMocks.getTopDirectories.mockResolvedValue([]);
    reportMocks.getExceptionPareto.mockResolvedValue([]);
    reportMocks.getExceptions.mockResolvedValue([]);
    searchCreate.mockResolvedValue({});
  });

  it('renders the frozen dashboard workflow and changes history range', async () => {
    render(
      <MemoryRouter>
        <CompatibilityDashboard server={{ isConnected: true }} />
      </MemoryRouter>,
    );

    expect(await screen.findByText('History')).toBeInTheDocument();
    expect(screen.getByText('Downloaded · 2 files')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '7d' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Users' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Content' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Errors' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '7d' }));
    await waitFor(() => {
      expect(reportMocks.getHistogram).toHaveBeenCalledWith(
        expect.objectContaining({ buckets: 84 }),
      );
    });
  });

  it('starts a search from the target-style search bar', async () => {
    render(
      <MemoryRouter>
        <CompatibilityDashboard server={{ isConnected: true }} />
      </MemoryRouter>,
    );

    const input = await screen.findByPlaceholderText('Search phrase');
    fireEvent.change(input, { target: { value: 'ambient' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => {
      expect(searchCreate).toHaveBeenCalledWith(
        expect.objectContaining({ searchText: 'ambient' }),
      );
    });
  });
});
