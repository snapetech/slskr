export interface MonitoringMetrics {
  sharesFiles: number;
  transfersTotal: number;
  transfersActive: number;
  eventsTotal: number;
}

const EMPTY_METRICS: MonitoringMetrics = {
  sharesFiles: 0,
  transfersTotal: 0,
  transfersActive: 0,
  eventsTotal: 0,
};

export function parseMonitoringMetrics(body: string): MonitoringMetrics {
  const samples = new Map<string, number>();

  for (const rawLine of body.split('\n')) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;

    const match = line.match(/^([A-Za-z_:][A-Za-z0-9_:]*)(\{[^}]*\})?\s+([^\s]+)(?:\s+\d+)?$/);
    if (!match) continue;

    const value = Number(match[3]);
    if (Number.isFinite(value)) {
      samples.set(`${match[1]}${match[2] ?? ''}`, value);
    }
  }

  return {
    ...EMPTY_METRICS,
    sharesFiles: samples.get('slskr_shares_files') ?? 0,
    transfersTotal: samples.get('slskr_transfers{state="total"}') ?? 0,
    transfersActive: samples.get('slskr_transfers{state="active"}') ?? 0,
    eventsTotal: samples.get('slskr_events_total') ?? 0,
  };
}
