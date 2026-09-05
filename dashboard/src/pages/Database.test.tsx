import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Database from './Database';
import { requestJson } from '../lib/api';

vi.mock('../lib/api', async () => {
  const actual = await vi.importActual<typeof import('../lib/api')>('../lib/api');
  return { ...actual, requestJson: vi.fn() };
});

describe('Database page', () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('does not present zero database counts when the first request fails', async () => {
    vi.mocked(requestJson).mockRejectedValue(new Error('database unavailable'));

    render(<Database apiUrl="https://example.test" apiKey={null} />);

    await waitFor(() => expect(screen.getByText('database unavailable')).toBeTruthy());
    expect(screen.getAllByText('—')).toHaveLength(3);
    expect(screen.queryByText('0')).toBeNull();
  });

  it('renders actual zero counts from a successful response', async () => {
    vi.mocked(requestJson).mockResolvedValue({ searches: 0, transfers: 0, messages: 0 });

    render(<Database apiUrl="https://example.test" apiKey={null} />);

    await waitFor(() => expect(screen.getAllByText('0')).toHaveLength(3));
    expect(screen.queryByText('—')).toBeNull();
  });
});
