import '@testing-library/jest-dom';
import CompatibilityDashboard from './CompatibilityDashboard';
import React from 'react';
import { MemoryRouter, useLocation } from 'react-router-dom';
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
vi.mock('uuid', () => ({ v4: () => 'bridge/search' }));
vi.mock('react-toastify', () => ({ toast: { error: vi.fn(), info: vi.fn() } }));

const LocationProbe = () => {
  const location = useLocation();
  return <output data-testid="location-path">{location.pathname}</output>;
};

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

  it('retains the last successful report when a range refresh fails', async () => {
    reportMocks.getSummary.mockReset();
    reportMocks.getSummary
      .mockResolvedValueOnce({
        Download: { Succeeded: { count: 4, distinctUsers: 2, totalBytes: 4_096 } },
        Upload: { Succeeded: { count: 2, distinctUsers: 1, totalBytes: 2_048 } },
      })
      .mockRejectedValueOnce(new Error('summary service unavailable'));

    render(
      <MemoryRouter>
        <CompatibilityDashboard server={{ isConnected: true }} />
      </MemoryRouter>,
    );

    expect(await screen.findByText('Downloaded · 4 files')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '7d' }));

    await waitFor(() => {
      expect(screen.getByTestId('compatibility-summary-load-error')).toHaveTextContent(
        'summary service unavailable',
      );
      expect(screen.getByText('Downloaded · 4 files')).toBeInTheDocument();
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

  it('encodes search IDs before opening result routes', async () => {
    render(
      <MemoryRouter>
        <CompatibilityDashboard server={{ isConnected: true }} />
        <LocationProbe />
      </MemoryRouter>,
    );

    const input = await screen.findByPlaceholderText('Search phrase');
    fireEvent.change(input, { target: { value: 'ambient' } });
    fireEvent.click(screen.getByRole('button', { name: 'Search and open results' }));

    await waitFor(() => {
      expect(screen.getByTestId('location-path')).toHaveTextContent(
        '/searches/bridge%2Fsearch',
      );
    });
  });
});
