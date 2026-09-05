import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Dashboard from './Dashboard';
import { useApi } from '../context/ApiContext';
import { useFetch } from '../hooks/useFetch';

vi.mock('../context/ApiContext', () => ({
  useApi: vi.fn(),
}));

vi.mock('../hooks/useFetch', () => ({
  useFetch: vi.fn(),
}));

describe('Dashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useApi).mockReturnValue({
      apiKey: null,
      apiUrl: 'https://example.test',
      isConnected: true,
      setApiKey: vi.fn(),
      setApiUrl: vi.fn(),
      setIsConnected: vi.fn(),
    });
  });

  it('keeps the last statistics visible when auto-refresh fails', () => {
    vi.mocked(useFetch).mockReturnValue({
      data: {
        searches: { total: 12 },
        session: { connected: true, state: 'ready' },
        transfers: { in_progress: 3 },
        users: { total: 8 },
      },
      error: new Error('stats refresh unavailable'),
      loading: false,
      refetch: vi.fn(),
    });

    render(<Dashboard />);

    expect(screen.getByRole('alert').textContent).toContain(
      'Showing the last successfully loaded values.',
    );
    expect(screen.getByText('12')).toBeTruthy();
    expect(screen.getByText('3')).toBeTruthy();
  });
});
