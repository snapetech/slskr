import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import { useFetch } from './hooks/useFetch';

vi.mock('./hooks/useFetch', () => ({
  useFetch: vi.fn(),
}));

describe('App connection health', () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
    vi.clearAllMocks();
  });

  it('keeps the application usable when a health refresh fails after success', async () => {
    vi.mocked(useFetch).mockReturnValue({
      data: { status: 'ok' },
      error: null,
      loading: false,
      refetch: vi.fn(),
    });
    const view = render(<App />);

    await waitFor(() => {
      expect(screen.queryByText('Connection Required')).toBeNull();
    });

    vi.mocked(useFetch).mockReturnValue({
      data: { status: 'ok' },
      error: new Error('API unavailable'),
      loading: false,
      refetch: vi.fn(),
    });
    view.rerender(<App />);

    expect(screen.queryByText('Connection Required')).toBeNull();
    expect(screen.getByText(/Connection degraded: API unavailable/)).toBeTruthy();
  });
});
