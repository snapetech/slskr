import '@testing-library/jest-dom';
import { createLogsHubConnection } from '../../../lib/hubFactory';
import { getLogs, updateLogLevel } from '../../../lib/options';
import Logs from './index';
import { cleanup, render, screen } from '@testing-library/react';
import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../lib/hubFactory', () => ({
  createLogsHubConnection: vi.fn(),
}));

vi.mock('../../../lib/options', () => ({
  getLogs: vi.fn(),
  updateLogLevel: vi.fn(),
}));

describe('Logs', () => {
  beforeEach(() => {
    getLogs.mockRejectedValue({
      response: { data: { message: 'Log endpoint unavailable' } },
    });
    updateLogLevel.mockResolvedValue({ level: 'Information' });
    createLogsHubConnection.mockReturnValue({
      on: vi.fn(),
      onclose: vi.fn(),
      onreconnected: vi.fn(),
      onreconnecting: vi.fn(),
      start: vi.fn().mockResolvedValue(undefined),
      stop: vi.fn(),
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('surfaces initial log-load failures instead of showing an empty success state', async () => {
    render(<Logs />);

    expect(await screen.findByTestId('logs-error')).toHaveTextContent(
      'Log endpoint unavailable',
    );
    expect(screen.getByText('No logs are available from the server')).toBeInTheDocument();
    expect(screen.queryByText('No logs match the selected filter')).not.toBeInTheDocument();
  });
});
