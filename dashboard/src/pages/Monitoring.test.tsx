import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Monitoring from './Monitoring';
import { requestText } from '../lib/api';

vi.mock('../lib/api', async () => {
  const actual = await vi.importActual<typeof import('../lib/api')>('../lib/api');
  return { ...actual, requestText: vi.fn() };
});

describe('Monitoring page', () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('does not present zero metrics when the first request fails', async () => {
    vi.mocked(requestText).mockRejectedValue(new Error('metrics unavailable'));

    render(<Monitoring apiUrl="https://example.test" apiKey={null} />);

    await waitFor(() => expect(screen.getByText('metrics unavailable')).toBeTruthy());
    expect(screen.getAllByText('—')).toHaveLength(4);
    expect(screen.queryByText('0')).toBeNull();
  });

  it('keeps real zero samples as zero', async () => {
    vi.mocked(requestText).mockResolvedValue(
      'slskr_shares_files 0\nslskr_transfers{state="total"} 0\nslskr_transfers{state="active"} 0\nslskr_events_total 0\n',
    );

    render(<Monitoring apiUrl="https://example.test" apiKey={null} />);

    await waitFor(() => expect(screen.getAllByText('0')).toHaveLength(4));
    expect(screen.queryByText('—')).toBeNull();
  });
});
