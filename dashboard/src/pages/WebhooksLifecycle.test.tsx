import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Webhooks from './Webhooks';
import { requestJson } from '../lib/api';

vi.mock('../lib/api', () => ({
  apiEndpoint: (_base: string, path: string) => path,
  isAbortError: () => false,
  requestJson: vi.fn(),
}));

const webhook = {
  id: 'hook/one',
  url: 'https://hooks.example.test/notify',
  events: ['search.created'],
  active: true,
  created_at: 123,
  last_triggered: 456,
};

function renderPage() {
  return render(<Webhooks apiUrl="https://api.example.test" apiKey="test-key" />);
}

describe('webhook request lifecycle', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('announces loading and accepts the object response shape', async () => {
    let resolve!: (value: unknown) => void;
    vi.mocked(requestJson).mockImplementation(() => new Promise((done) => { resolve = done; }));
    renderPage();
    expect(screen.getByRole('status').textContent).toContain('Loading webhooks');
    resolve({ webhooks: [webhook] });
    expect(await screen.findByText(webhook.url)).toBeTruthy();
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.getByRole('button', { name: `Delete webhook ${webhook.url}` })).toBeTruthy();
  });

  it.each([
    { webhooks: 'invalid' },
    [{ ...webhook, events: [17] }],
    [{ ...webhook, created_at: Number.NaN }],
    [null],
  ])('announces malformed responses without inventing valid records (%j)', async (data) => {
    vi.mocked(requestJson).mockResolvedValue(data);
    renderPage();
    expect((await screen.findByRole('alert')).textContent).toContain('invalid webhook response');
    expect(screen.queryByText(webhook.url)).toBeNull();
  });

  it('creates a webhook with the selected events and refreshes the list', async () => {
    vi.mocked(requestJson)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce({})
      .mockResolvedValueOnce({ webhooks: [webhook] });
    const user = userEvent.setup();
    renderPage();
    await screen.findByText('No webhooks configured');
    await user.click(screen.getByRole('button', { name: 'New Webhook' }));
    await user.type(screen.getByLabelText('URL'), webhook.url);
    await user.click(screen.getByRole('checkbox', { name: 'search.created' }));
    expect((screen.getByRole('button', { name: 'Create' }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole('checkbox', { name: 'transfer.completed' }));
    await user.click(screen.getByRole('button', { name: 'Create' }));
    expect(await screen.findByText(webhook.url)).toBeTruthy();
    expect(requestJson).toHaveBeenNthCalledWith(2, '/api/admin/webhooks', 'test-key', {
      method: 'POST',
      body: JSON.stringify({ url: webhook.url, events: ['transfer.completed'] }),
      signal: expect.any(AbortSignal),
    });
    expect(screen.queryByText('Create Webhook')).toBeNull();
  });

  it('does not submit an empty URL and can cancel the editor', async () => {
    vi.mocked(requestJson).mockResolvedValue([]);
    const user = userEvent.setup();
    renderPage();
    await screen.findByText('No webhooks configured');
    await user.click(screen.getByRole('button', { name: 'New Webhook' }));
    await user.click(screen.getByRole('button', { name: 'Create' }));
    expect(requestJson).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByText('Create Webhook')).toBeNull();
  });

  it('requires delete confirmation and encodes the selected identifier', async () => {
    vi.mocked(requestJson)
      .mockResolvedValueOnce([webhook])
      .mockResolvedValueOnce({})
      .mockResolvedValueOnce([]);
    const confirm = vi.spyOn(window, 'confirm').mockReturnValueOnce(false).mockReturnValueOnce(true);
    const user = userEvent.setup();
    renderPage();
    await screen.findByText(webhook.url);
    await user.click(screen.getByRole('button', { name: `Delete webhook ${webhook.url}` }));
    expect(confirm).toHaveBeenCalledTimes(1);
    expect(requestJson).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole('button', { name: `Delete webhook ${webhook.url}` }));
    expect(await screen.findByText('No webhooks configured')).toBeTruthy();
    expect(requestJson).toHaveBeenNthCalledWith(2, '/api/admin/webhooks/hook%2Fone', 'test-key', {
      method: 'DELETE',
      signal: expect.any(AbortSignal),
    });
  });

  it('sends a selected test webhook and surfaces request failure', async () => {
    vi.mocked(requestJson)
      .mockResolvedValueOnce([webhook])
      .mockResolvedValueOnce({})
      .mockRejectedValueOnce(new Error('Delivery failed'));
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {});
    const user = userEvent.setup();
    renderPage();
    await screen.findByText(webhook.url);
    const button = screen.getByRole('button', { name: `Send test webhook to ${webhook.url}` });
    await user.click(button);
    await waitFor(() => expect(alert).toHaveBeenCalledWith('Test webhook sent!'));
    expect(requestJson).toHaveBeenNthCalledWith(2, '/api/admin/webhooks/hook%2Fone/test', 'test-key', {
      method: 'POST',
      signal: expect.any(AbortSignal),
    });
    await user.click(button);
    expect((await screen.findByRole('alert')).textContent).toContain('Delivery failed');
    expect(screen.getByText(webhook.url)).toBeTruthy();
  });

  it('reports create and delete failures while preserving the configured row', async () => {
    vi.mocked(requestJson)
      .mockResolvedValueOnce([webhook])
      .mockRejectedValueOnce(new Error('Create failed'))
      .mockRejectedValueOnce(new Error('Delete failed'));
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    const user = userEvent.setup();
    renderPage();
    await screen.findByText(webhook.url);
    await user.click(screen.getByRole('button', { name: 'New Webhook' }));
    await user.type(screen.getByLabelText('URL'), 'https://other.example.test/webhook');
    await user.click(screen.getByRole('button', { name: 'Create' }));
    expect((await screen.findByRole('alert')).textContent).toContain('Create failed');
    await user.click(screen.getByRole('button', { name: `Delete webhook ${webhook.url}` }));
    expect((await screen.findByRole('alert')).textContent).toContain('Delete failed');
    expect(screen.getByText(webhook.url)).toBeTruthy();
  });
});
