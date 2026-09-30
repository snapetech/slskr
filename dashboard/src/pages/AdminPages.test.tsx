import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ApiKeys from './ApiKeys';
import Configuration from './Configuration';
import Webhooks from './Webhooks';
import { requestJson } from '../lib/api';

vi.mock('../lib/api', () => ({
  apiEndpoint: (_base: string, path: string) => path,
  isAbortError: () => false,
  requestJson: vi.fn(),
}));

describe('administrative pages', () => {
  afterEach(cleanup);
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the API authentication state without exposing a token', async () => {
    vi.mocked(requestJson).mockResolvedValue({
      keys: [],
      mode: 'static',
      reason: 'Configured by the daemon',
    });

    render(<ApiKeys apiUrl="https://example.test" apiKey="secret" />);

    expect(await screen.findByText('Static token authentication')).toBeTruthy();
    expect(screen.getByText('Configured by the daemon')).toBeTruthy();
    expect(screen.queryByText('secret')).toBeNull();
  });

  it('loads configuration and sends the edited download filter', async () => {
    vi.mocked(requestJson)
      .mockResolvedValueOnce({ autoreplace_enabled: false })
      .mockResolvedValueOnce({ exclude: ['old'], maxTerms: 10, maxTermLength: 32 })
      .mockResolvedValueOnce({});
    const user = userEvent.setup();

    render(<Configuration apiUrl="https://example.test" apiKey="secret" />);

    const input = await screen.findByDisplayValue('old');
    await user.clear(input);
    await user.type(input, 'new');
    await user.click(screen.getByRole('button', { name: 'Save download filter' }));

    expect(requestJson).toHaveBeenLastCalledWith('/api/config/download-filter', 'secret', expect.objectContaining({
      method: 'PUT',
      body: JSON.stringify({ exclude: ['new'] }),
    }));
    expect(await screen.findByText('Download filter saved.')).toBeTruthy();
  });

  it('aborts an in-flight configuration write when the page unmounts', async () => {
    let writeSignal: AbortSignal | undefined;
    vi.mocked(requestJson).mockImplementation((url, _key, init) => {
      if (url === '/api/config/preferences') {
        return Promise.resolve({ autoreplace_enabled: false });
      }
      if (url === '/api/config/download-filter' && init?.method !== 'PUT') {
        return Promise.resolve({ exclude: ['old'], maxTerms: 10, maxTermLength: 32 });
      }
      if (url === '/api/config/download-filter') {
        writeSignal = init?.signal ?? undefined;
        return new Promise((_resolve, reject) => {
          writeSignal?.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')));
        });
      }
      return Promise.reject(new Error(`Unexpected request: ${url}`));
    });
    const user = userEvent.setup();
    const view = render(<Configuration apiUrl="https://example.test" apiKey="secret" />);

    await screen.findByDisplayValue('old');
    await user.click(screen.getByRole('button', { name: 'Save download filter' }));
    await waitFor(() => expect(writeSignal).toBeDefined());
    view.unmount();

    expect(writeSignal?.aborted).toBe(true);
  });

  it('renders the empty webhook state and exposes the create action', async () => {
    vi.mocked(requestJson).mockResolvedValue([]);

    render(<Webhooks apiUrl="https://example.test" apiKey="secret" />);

    expect(await screen.findByText('No webhooks configured')).toBeTruthy();
    expect(screen.getByRole('button', { name: /new webhook/i })).toBeTruthy();
  });

  it('aborts an in-flight webhook mutation when the page unmounts', async () => {
    let writeSignal: AbortSignal | undefined;
    vi.mocked(requestJson).mockImplementation((url, _key, init) => {
      if (url === '/api/admin/webhooks' && init?.method === 'POST') {
        writeSignal = init.signal ?? undefined;
        return new Promise((_resolve, reject) => {
          writeSignal?.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')));
        });
      }
      return Promise.resolve([]);
    });
    const user = userEvent.setup();
    const view = render(<Webhooks apiUrl="https://example.test" apiKey="secret" />);

    await screen.findByText('No webhooks configured');
    await user.click(screen.getByRole('button', { name: /new webhook/i }));
    await user.type(screen.getByLabelText('URL'), 'https://hooks.example.test/notify');
    await user.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => expect(writeSignal).toBeDefined());
    view.unmount();

    expect(writeSignal?.aborted).toBe(true);
  });
});
