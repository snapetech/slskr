import { describe, expect, it } from 'vitest';

import { parseMonitoringMetrics } from './prometheus';

describe('parseMonitoringMetrics', () => {
  it('projects the counters used by the monitoring page', () => {
    const metrics = parseMonitoringMetrics(`
# HELP slskr_shares_files Number of shared files
slskr_shares_files 42
slskr_transfers{state="total"} 7
slskr_transfers{state="active"} 2
slskr_events_total 19
`);

    expect(metrics).toEqual({
      sharesFiles: 42,
      transfersTotal: 7,
      transfersActive: 2,
      eventsTotal: 19,
    });
  });

  it('defaults missing and invalid samples to zero', () => {
    expect(parseMonitoringMetrics('slskr_shares_files NaN\nmalformed')).toEqual({
      sharesFiles: 0,
      transfersTotal: 0,
      transfersActive: 0,
      eventsTotal: 0,
    });
  });
});
