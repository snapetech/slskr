import '@testing-library/jest-dom';
import { getKpiMetrics } from '../../../lib/telemetry';
import Metrics from './index';
import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../lib/telemetry', () => ({
  getKpiMetrics: vi.fn(),
}));

vi.mock('../../Shared', () => ({
  LoaderSegment: () => <div>Loading metrics...</div>,
}));

describe('Metrics', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('retains the last successful snapshot when a refresh fails', async () => {
    getKpiMetrics
      .mockResolvedValueOnce({
        slskr_uploads_total: {
          help: 'Uploads total',
          samples: [{ value: 3 }],
          type: 'counter',
        },
      })
      .mockRejectedValueOnce(new Error('metrics service unavailable'));

    render(<Metrics />);

    expect(await screen.findByText('Uploads Total')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Updated/ }));

    expect(await screen.findByTestId('metrics-load-error')).toHaveTextContent(
      'metrics service unavailable',
    );
    expect(screen.getByText('Uploads Total')).toBeInTheDocument();
    expect(screen.getAllByText('3')).not.toHaveLength(0);
    await waitFor(() => expect(getKpiMetrics).toHaveBeenCalledTimes(2));
  });
});
