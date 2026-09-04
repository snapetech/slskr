import { getKpiMetrics } from '../../../lib/telemetry';
import { toDisplayError } from '../../../lib/errors';
import { LoaderSegment } from '../../Shared';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Divider,
  Grid,
  Header,
  Icon,
  Message,
  Segment,
  Statistic,
  Table,
} from 'semantic-ui-react';

const formatBytes = (bytes) => {
  const numericBytes = Number(bytes);
  if (!Number.isFinite(numericBytes) || numericBytes <= 0) return '0 B';
  const k = 1_024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(
    sizes.length - 1,
    Math.floor(Math.log(numericBytes) / Math.log(k)),
  );
  return `${(numericBytes / k ** i).toFixed(1)} ${sizes[i]}`;
};

const formatNumber = (value) => {
  if (value === undefined || value === null) return '—';
  const numericValue = Number(value);
  if (!Number.isFinite(numericValue)) return '—';
  if (numericValue >= 1_000_000) return `${(numericValue / 1_000_000).toFixed(1)}M`;
  if (numericValue >= 1_000) return `${(numericValue / 1_000).toFixed(1)}K`;
  return Number.parseFloat(numericValue.toFixed(2)).toString();
};

// Extract the first sample value from a metric, or null
const sampleValue = (metric) => {
  if (!metric) return null;
  if (Array.isArray(metric.samples) && metric.samples.length > 0) {
    return metric.samples[0]?.value ?? null;
  }
  return null;
};

const KPI_GROUPS = [
  {
    key: 'transfers',
    title: 'Transfers',
    icon: 'exchange',
    metrics: [
      { key: 'slskr_uploads_total', label: 'Uploads Total', format: formatNumber },
      { key: 'slskr_downloads_total', label: 'Downloads Total', format: formatNumber },
      { key: 'slskr_uploads_active', label: 'Uploads Active', format: formatNumber },
      { key: 'slskr_downloads_active', label: 'Downloads Active', format: formatNumber },
      { key: 'slskr_uploads_queued', label: 'Uploads Queued', format: formatNumber },
      { key: 'slskr_downloads_queued', label: 'Downloads Queued', format: formatNumber },
    ],
  },
  {
    key: 'search',
    title: 'Search',
    icon: 'search',
    metrics: [
      { key: 'slskr_searches_incoming_requests_total', label: 'Incoming Requests', format: formatNumber },
      { key: 'slskr_searches_incoming_requests_dropped_total', label: 'Dropped Requests', format: formatNumber },
      { key: 'slskr_searches_outgoing_total', label: 'Outgoing Searches', format: formatNumber },
    ],
  },
  {
    key: 'process',
    title: 'Process',
    icon: 'microchip',
    metrics: [
      { key: 'process_working_set_bytes', label: 'Working Set', format: formatBytes },
      { key: 'dotnet_total_memory_bytes', label: 'Managed Memory', format: formatBytes },
      { key: 'process_cpu_seconds_total', label: 'CPU Seconds', format: formatNumber },
    ],
  },
  {
    key: 'network',
    title: 'Network',
    icon: 'wifi',
    metrics: [
      { key: 'microsoft_aspnetcore_server_kestrel_current_connections', label: 'Kestrel Connections', format: formatNumber },
      { key: 'system_net_sockets_connections_established_total', label: 'Sockets Established', format: formatNumber },
    ],
  },
];

const MetricGroup = ({ group, metrics }) => {
  const normalizedMetrics = metrics && typeof metrics === 'object' ? metrics : {};
  const items = group.metrics
    .map(({ key, label, format }) => ({
      label,
      value: sampleValue(normalizedMetrics[key]),
      format,
    }))
    .filter(({ value }) => value !== null);

  if (items.length === 0) return null;

  return (
    <Segment>
      <Header size="small">
        <Icon name={group.icon} />
        {group.title}
      </Header>
      <Statistic.Group size="mini">
        {items.map(({ label, value, format }) => (
          <Statistic key={label}>
            <Statistic.Value>{format(value)}</Statistic.Value>
            <Statistic.Label>{label}</Statistic.Label>
          </Statistic>
        ))}
      </Statistic.Group>
    </Segment>
  );
};

const SlskrMetricsTable = ({ metrics }) => {
  const slskrMetrics = Object.entries(
    metrics && typeof metrics === 'object' ? metrics : {},
  )
    .filter(([key]) => key.startsWith('slskr_'))
    .sort(([a], [b]) => a.localeCompare(b));

  if (slskrMetrics.length === 0) return null;

  return (
    <Segment>
      <Header size="small">
        <Icon name="table" />
        All slskr Metrics
      </Header>
      <Table
        compact
        size="small"
        striped
      >
        <Table.Header>
          <Table.Row>
            <Table.HeaderCell>Metric</Table.HeaderCell>
            <Table.HeaderCell>Type</Table.HeaderCell>
            <Table.HeaderCell>Value</Table.HeaderCell>
            <Table.HeaderCell>Help</Table.HeaderCell>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {slskrMetrics.map(([key, metric]) => (
            <Table.Row key={key}>
              <Table.Cell>
                <code style={{ fontSize: '0.85em' }}>{key}</code>
              </Table.Cell>
              <Table.Cell>{metric.type}</Table.Cell>
              <Table.Cell>{formatNumber(sampleValue(metric))}</Table.Cell>
              <Table.Cell style={{ color: 'grey', fontSize: '0.9em' }}>{metric.help}</Table.Cell>
            </Table.Row>
          ))}
        </Table.Body>
      </Table>
    </Segment>
  );
};

const Metrics = () => {
  const [metrics, setMetrics] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [lastUpdated, setLastUpdated] = useState(null);
  const mountedRef = useMountedRef();
  const requestIdRef = useRef(0);
  const inFlightRef = useRef(false);

  const fetchMetrics = useCallback(async () => {
    if (!mountedRef.current || inFlightRef.current) return;
    const requestId = ++requestIdRef.current;
    inFlightRef.current = true;
    setLoading(true);
    setError(null);
    try {
      const data = await getKpiMetrics();
      if (
        !mountedRef.current ||
        requestIdRef.current !== requestId
      ) {
        return;
      }
      setMetrics(data && typeof data === 'object' ? data : {});
      setLastUpdated(new Date());
    } catch (err) {
      if (
        mountedRef.current &&
        requestIdRef.current === requestId
      ) {
        setError(toDisplayError(err, 'Failed to load metrics'));
      }
    } finally {
      if (
        mountedRef.current &&
        requestIdRef.current === requestId
      ) {
        setLoading(false);
      }
      inFlightRef.current = false;
    }
  }, [mountedRef]);

  useEffect(() => {
    void fetchMetrics();
  }, [fetchMetrics]);

  if (loading && !metrics) return <LoaderSegment />;

  if (error) {
    return (
      <Message
        error
        header="Failed to load metrics"
        content={error}
      />
    );
  }

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1em' }}>
        <Header size="medium">
          <Icon name="chart bar" />
          Prometheus Metrics
        </Header>
        <span style={{ color: 'grey', fontSize: '0.9em', cursor: 'pointer' }} onClick={fetchMetrics}>
          <Icon name="refresh" />
          {lastUpdated ? `Updated ${lastUpdated.toLocaleTimeString()}` : 'Refresh'}
        </span>
      </div>

      <Grid stackable>
        {KPI_GROUPS.map((group) => (
          <Grid.Column key={group.key} width={8}>
            <MetricGroup
              group={group}
              metrics={metrics}
            />
          </Grid.Column>
        ))}
      </Grid>

      <Divider />

      <SlskrMetricsTable metrics={metrics} />
    </div>
  );
};

export default Metrics;
