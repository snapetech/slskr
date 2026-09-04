import './CompatibilityDashboard.css';
import { toDisplayError } from '../lib/errors';
import * as reports from '../lib/reports';
import * as searches from '../lib/searches';
import { formatBytes, formatDate, formatSpeed, formatWait, getFileName, truncate } from '../lib/util';
import LoaderSegment from './Shared/LoaderSegment';
import { useMountedRef } from '../lib/useMountedRef';
import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { toast } from 'react-toastify';
import {
  Button,
  ButtonGroup,
  Grid,
  Header,
  Icon,
  Input,
  Progress,
  Segment,
  Statistic,
  Message,
  Tab,
  Table,
} from 'semantic-ui-react';
import { v4 as uuidv4 } from 'uuid';

const HISTORY_RANGES = [
  { buckets: 24, days: 1, label: '24h' },
  { buckets: 84, days: 7, label: '7d' },
  { buckets: 60, days: 30, label: '30d' },
  { buckets: 90, days: 90, label: '90d' },
  { buckets: 90, days: 180, label: '180d' },
  { buckets: 100, days: 365, label: '1y' },
  { buckets: 100, days: null, label: 'All' },
];

const EMPTY_REPORT = {
  directories: [],
  exceptions: {
    download: { pareto: [], recent: [] },
    upload: { pareto: [], recent: [] },
  },
  histogram: {},
  leaderboard: { download: [], upload: [] },
  summary: {},
};

const isRecord = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

const asRows = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (row) => row && typeof row === 'object' && !Array.isArray(row),
  );

const normalizeReport = ({
  directories,
  downloadLeaderboard,
  downloadPareto,
  downloadRecent,
  histogram,
  summary,
  uploadLeaderboard,
  uploadPareto,
  uploadRecent,
}) => ({
  directories: asRows(directories),
  exceptions: {
    download: {
      pareto: asRows(downloadPareto),
      recent: asRows(downloadRecent),
    },
    upload: {
      pareto: asRows(uploadPareto),
      recent: asRows(uploadRecent),
    },
  },
  histogram: isRecord(histogram) ? histogram : {},
  leaderboard: {
    download: asRows(downloadLeaderboard),
    upload: asRows(uploadLeaderboard),
  },
  summary: isRecord(summary) ? summary : {},
});

const sumCounts = (directionData = {}) =>
  Object.values(isRecord(directionData) ? directionData : {}).reduce(
    (sum, state) => sum + (Number(state?.count) || 0),
    0,
  );

const sumBytes = (directionData = {}) =>
  Object.values(isRecord(directionData) ? directionData : {}).reduce(
    (sum, state) => sum + (Number(state?.totalBytes) || 0),
    0,
  );

const errorCount = (directionData = {}) =>
  (Number(directionData?.Errored?.count) || 0) +
  (Number(directionData?.Cancelled?.count) || 0) +
  (Number(directionData?.TimedOut?.count) || 0);

const formatBytesParts = (bytes) => {
  if (!bytes || bytes < 1) return { unit: 'B', value: '0' };
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const index = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1_024)),
  );
  return { unit: units[index], value: (bytes / 1_024 ** index).toFixed(1) };
};

const buildChartData = (histogram = {}) =>
  Object.entries(isRecord(histogram) ? histogram : {})
    .sort(([left], [right]) => new Date(left) - new Date(right))
    .map(([timestamp, directions]) => {
      const safeDirections = isRecord(directions) ? directions : {};
      const upload = isRecord(safeDirections.Upload) ? safeDirections.Upload : {};
      const download = isRecord(safeDirections.Download) ? safeDirections.Download : {};
      const uploadBytes = sumBytes(upload);
      const downloadBytes = sumBytes(download);
      const uploadCount = sumCounts(upload);
      const downloadCount = sumCounts(download);
      const uploadErrors = errorCount(upload);
      const downloadErrors = errorCount(download);

      return {
        downloadBytes,
        downloadCount,
        downloadErrorRate:
          downloadCount > 0 ? (downloadErrors / downloadCount) * 100 : 0,
        downloadErrors,
        downloadSpeed: Number(download.Succeeded?.averageSpeed) || 0,
        shareRatio: downloadBytes > 0 ? uploadBytes / downloadBytes : 0,
        timestamp: new Date(timestamp).getTime(),
        uploadBytes,
        uploadCount,
        uploadErrorRate:
          uploadCount > 0 ? (uploadErrors / uploadCount) * 100 : 0,
        uploadErrors,
        uploadSpeed: Number(upload.Succeeded?.averageSpeed) || 0,
        uploadWait: Number(upload.Succeeded?.averageWait) || 0,
      };
    });

const HISTORY_SERIES = [
  { color: '#21ba45', format: (value) => formatBytes(value, 1), key: 'uploadBytes', name: 'Upload Size' },
  { color: '#2185d0', format: (value) => formatBytes(value, 1), key: 'downloadBytes', name: 'Download Size' },
  { color: '#6435c9', format: (value) => value.toLocaleString(), key: 'uploadCount', name: 'Upload Count' },
  { color: '#e03997', format: (value) => value.toLocaleString(), key: 'downloadCount', name: 'Download Count' },
  { color: '#f2711c', format: formatSpeed, key: 'uploadSpeed', name: 'Upload Speed' },
  { color: '#fbbd08', format: formatSpeed, key: 'downloadSpeed', name: 'Download Speed' },
  { color: '#db2828', format: (value) => value.toLocaleString(), key: 'uploadErrors', name: 'Upload Errors' },
  { color: '#a333c8', format: (value) => value.toLocaleString(), key: 'downloadErrors', name: 'Download Errors' },
  { color: '#d4500a', format: (value) => `${value.toFixed(1)}%`, key: 'uploadErrorRate', name: 'Upload Error Rate' },
  { color: '#1aa9b0', format: (value) => `${value.toFixed(1)}%`, key: 'downloadErrorRate', name: 'Download Error Rate' },
  { color: '#8e44ad', format: formatWait, key: 'uploadWait', name: 'Upload Queue Wait' },
  { color: '#b5cc18', format: (value) => value.toFixed(2), key: 'shareRatio', name: 'Share Ratio' },
];

const SearchBar = ({ server } = {}) => {
  const navigate = useNavigate();
  const [searchText, setSearchText] = useState('');
  const [creating, setCreating] = useState(false);
  const mountedRef = useMountedRef();
  const createInFlightRef = useRef(false);
  const connected = Boolean(server?.isConnected);

  const create = async ({ navigateToResults = false } = {}) => {
    const query = searchText.trim();
    if (
      !query ||
      !connected ||
      creating ||
      !mountedRef.current ||
      createInFlightRef.current
    ) {
      return;
    }

    createInFlightRef.current = true;
    try {
      setCreating(true);
      const id = uuidv4();
      await searches.create({ id, searchText: query });
      if (!mountedRef.current) return;
      setSearchText('');
      if (navigateToResults) {
        navigate(`/searches/${id}`);
      } else {
        const label = query.length > 30 ? `${query.slice(0, 15)}...` : query;
        toast.info(`Search for '${label}' started.`);
      }
    } catch (error) {
      if (mountedRef.current) {
        toast.error(toDisplayError(error, 'Failed to start search'));
      }
    } finally {
      createInFlightRef.current = false;
      if (mountedRef.current) setCreating(false);
    }
  };

  return (
    <Segment
      className="compatibility-search-segment"
      raised
    >
      <Icon
        name="search"
        size="big"
      />
      <Input
        action={(
          <>
            <Button
              disabled={!connected || creating || !searchText.trim()}
              icon="plus"
              loading={creating}
              onClick={() => create()}
            />
            <Button
              disabled={!connected || creating || !searchText.trim()}
              icon="search"
              onClick={() => create({ navigateToResults: true })}
            />
          </>
        )}
        className="compatibility-search-input"
        disabled={!connected || creating}
        onChange={(_, { value }) => setSearchText(value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') create();
        }}
        placeholder={
          connected
            ? 'Search phrase'
            : 'Connect to server to perform a search'
        }
        value={searchText}
      />
    </Segment>
  );
};

const CompatibilityGraph = ({ data = [], defaultSeries, series = [] }) => {
  const [visible, setVisible] = useState(() => new Set(defaultSeries ?? []));
  const [hoverIndex, setHoverIndex] = useState(null);
  const width = 800;
  const height = 240;
  const padding = { bottom: 28, left: 46, right: 18, top: 18 };
  const innerWidth = width - padding.left - padding.right;
  const innerHeight = height - padding.top - padding.bottom;

  const ranges = useMemo(() => {
    if (data.length === 0) return { max: 1, min: 0, xMax: 1, xMin: 0 };
    const xValues = data.map((point) => point.timestamp);
    const values = data.flatMap((point) =>
      series.filter((item) => visible.has(item.key)).map((item) => Number(point[item.key]) || 0),
    );
    return {
      max: Math.max(1, ...values),
      min: Math.min(...values, 0),
      xMax: Math.max(...xValues),
      xMin: Math.min(...xValues),
    };
  }, [data, series, visible]);

  const toPoint = (value, index) => {
    const xSpan = Math.max(1, ranges.xMax - ranges.xMin);
    const ySpan = Math.max(1, ranges.max - ranges.min);
    const x = padding.left + (((data[index]?.timestamp ?? ranges.xMin) - ranges.xMin) / xSpan) * innerWidth;
    const y = padding.top + innerHeight - ((Number(value) - ranges.min) / ySpan) * innerHeight;
    return `${x},${y}`;
  };

  const toggleSeries = (key) => {
    setVisible((previous) => {
      const next = new Set(previous);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  if (data.length === 0) {
    return <Segment className="compatibility-graph-empty">No data to display</Segment>;
  }

  const hovered = hoverIndex == null ? null : data[hoverIndex];

  return (
    <div className="compatibility-graph">
      <div className="compatibility-graph-frame">
        <svg
          aria-label="Transfer history"
          onMouseLeave={() => setHoverIndex(null)}
          onMouseMove={(event) => {
            const rect = event.currentTarget.getBoundingClientRect();
            const position = ((event.clientX - rect.left) / rect.width) * width;
            const index = Math.round(((position - padding.left) / innerWidth) * Math.max(0, data.length - 1));
            if (index >= 0 && index < data.length) setHoverIndex(index);
          }}
          role="img"
          viewBox={`0 0 ${width} ${height}`}
        >
          <line
            className="compatibility-graph-axis"
            x1={padding.left}
            x2={padding.left}
            y1={padding.top}
            y2={height - padding.bottom}
          />
          <line
            className="compatibility-graph-axis"
            x1={padding.left}
            x2={width - padding.right}
            y1={height - padding.bottom}
            y2={height - padding.bottom}
          />
          {series.filter((item) => visible.has(item.key)).map((item) => (
            <polyline
              className="compatibility-graph-line"
              fill="none"
              key={item.key}
              points={data.map((point, index) => toPoint(point[item.key], index)).join(' ')}
              stroke={item.color}
            />
          ))}
          {hovered && (
            <line
              className="compatibility-graph-hover"
              x1={toPoint(0, hoverIndex).split(',')[0]}
              x2={toPoint(0, hoverIndex).split(',')[0]}
              y1={padding.top}
              y2={height - padding.bottom}
            />
          )}
        </svg>
        {hovered && (
          <div className="compatibility-graph-tooltip">
            <strong>{new Date(hovered.timestamp).toLocaleString()}</strong>
            {series.filter((item) => visible.has(item.key)).map((item) => (
              <span key={item.key}>
                <i style={{ backgroundColor: item.color }} />
                {item.name}: {item.format ? item.format(Number(hovered[item.key]) || 0) : hovered[item.key]}
              </span>
            ))}
          </div>
        )}
      </div>
      <div className="compatibility-graph-legend">
        {series.map((item) => (
          <button
            aria-pressed={visible.has(item.key)}
            className={visible.has(item.key) ? '' : 'is-hidden'}
            key={item.key}
            onClick={() => toggleSeries(item.key)}
            type="button"
          >
            <i style={{ backgroundColor: item.color }} />
            {item.name}
          </button>
        ))}
      </div>
    </div>
  );
};

const LeaderboardTable = ({ loading, onSort, rows = [], sortBy }) => {
  const fields = [
    { label: 'Count', sort: 'Count' },
    { label: 'Total Size', sort: 'TotalBytes' },
    { label: 'Avg Speed', sort: 'AverageSpeed' },
  ];

  return (
    <Table
      className="unstackable"
      compact="very"
    >
      <Table.Header>
        <Table.Row>
          <Table.HeaderCell textAlign="right">#</Table.HeaderCell>
          <Table.HeaderCell>Username</Table.HeaderCell>
          {fields.map((field) => (
            <Table.HeaderCell
              key={field.sort}
              onClick={() => onSort(field.sort)}
              textAlign="right"
            >
              {field.label}
              {sortBy === field.sort && <Icon name="chevron down" />}
            </Table.HeaderCell>
          ))}
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {loading && (
          <Table.Row>
            <Table.Cell
              colSpan={5}
              textAlign="center"
            >
              <LoaderSegment size="small" />
            </Table.Cell>
          </Table.Row>
        )}
        {!loading && rows.length === 0 && (
          <Table.Row>
            <Table.Cell
              colSpan={5}
              style={{ opacity: 0.5, textAlign: 'center' }}
            >
              No data to display
            </Table.Cell>
          </Table.Row>
        )}
        {!loading && rows.map((row, index) => (
          <Table.Row key={row.username}>
            <Table.Cell textAlign="right">{index + 1}</Table.Cell>
            <Table.Cell>{row.username}</Table.Cell>
            <Table.Cell textAlign="right">{(row.count ?? 0).toLocaleString()}</Table.Cell>
            <Table.Cell textAlign="right">{formatBytes(row.totalBytes ?? 0)}</Table.Cell>
            <Table.Cell textAlign="right">{formatSpeed(row.averageSpeed ?? 0)}</Table.Cell>
          </Table.Row>
        ))}
      </Table.Body>
    </Table>
  );
};

const Leaderboard = ({ downloads, end, start, uploads }) => {
  const [sortBy, setSortBy] = useState('Count');
  const [rows, setRows] = useState({
    download: asRows(downloads),
    upload: asRows(uploads),
  });
  const [loading, setLoading] = useState({ download: false, upload: false });
  const sortRequest = useRef(0);
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      sortRequest.current += 1;
    };
  }, []);

  useEffect(() => {
    sortRequest.current += 1;
    setRows({ download: asRows(downloads), upload: asRows(uploads) });
    setSortBy('Count');
  }, [downloads, uploads]);

  const onSort = async (nextSort) => {
    if (nextSort === sortBy) return;
    const requestId = sortRequest.current + 1;
    sortRequest.current = requestId;
    setSortBy(nextSort);
    setLoading({ download: true, upload: true });
    try {
      const [upload, download] = await Promise.all([
        reports.getLeaderboard({ direction: 'Upload', end: new Date(end), sortBy: nextSort, start: start ? new Date(start) : undefined }),
        reports.getLeaderboard({ direction: 'Download', end: new Date(end), sortBy: nextSort, start: start ? new Date(start) : undefined }),
      ]);
      if (mountedRef.current && requestId === sortRequest.current) {
        setRows({ download: asRows(download), upload: asRows(upload) });
      }
    } catch (error) {
      if (mountedRef.current && requestId === sortRequest.current) {
        toast.error(toDisplayError(error, 'Failed to sort leaderboard'));
      }
    } finally {
      if (mountedRef.current && requestId === sortRequest.current) {
        setLoading({ download: false, upload: false });
      }
    }
  };

  return (
    <Grid
      columns={2}
      stackable
    >
      <Grid.Column>
        <Header size="small"><Icon name="download" /> Downloads</Header>
        <LeaderboardTable loading={loading.download} onSort={onSort} rows={rows.download} sortBy={sortBy} />
      </Grid.Column>
      <Grid.Column>
        <Header size="small"><Icon name="upload" /> Uploads</Header>
        <LeaderboardTable loading={loading.upload} onSort={onSort} rows={rows.upload} sortBy={sortBy} />
      </Grid.Column>
    </Grid>
  );
};

const TopDirectories = ({ rows = [] }) => (
  <>
    <Header size="small"><Icon name="folder open" /> Directories</Header>
    <Table
      className="unstackable"
      compact="very"
    >
      <Table.Header>
        <Table.Row>
          <Table.HeaderCell textAlign="right">#</Table.HeaderCell>
          <Table.HeaderCell>Directory</Table.HeaderCell>
          <Table.HeaderCell textAlign="right">Downloads</Table.HeaderCell>
          <Table.HeaderCell textAlign="right">Distinct Users</Table.HeaderCell>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {rows.length === 0 && (
          <Table.Row><Table.Cell colSpan={4} style={{ opacity: 0.5, textAlign: 'center' }}>No data to display</Table.Cell></Table.Row>
        )}
        {rows.map((row, index) => {
          const parts = (row.directory || '').split(/[/\\]/u).filter(Boolean);
          return (
            <Table.Row key={row.directory}>
              <Table.Cell textAlign="right">{index + 1}</Table.Cell>
              <Table.Cell title={row.directory}>{parts.slice(-2).join('/')}</Table.Cell>
              <Table.Cell textAlign="right">{(row.count ?? 0).toLocaleString()}</Table.Cell>
              <Table.Cell textAlign="right">{(row.distinctUsers ?? 0).toLocaleString()}</Table.Cell>
            </Table.Row>
          );
        })}
      </Table.Body>
    </Table>
  </>
);

const DIRECTION_OPTIONS = ['Upload', 'Download', 'All'];

const DirectionButtons = ({ direction, onChange }) => (
  <ButtonGroup
    size="mini"
    floated="right"
  >
    {DIRECTION_OPTIONS.map((option) => (
      <Button
        active={direction === option}
        key={option}
        onClick={() => onChange(option)}
      >
        {option}
      </Button>
    ))}
  </ButtonGroup>
);

const ExceptionPareto = ({ direction, loading, onDirectionChange, rows }) => {
  const maxCount = rows.length > 0 ? rows[0].count : 1;
  return (
    <>
      <Header size="small"><Icon name="sort amount down" /> Error Count By Type <DirectionButtons direction={direction} onChange={onDirectionChange} /></Header>
      <Table
        className="unstackable"
        compact="very"
      >
        <Table.Header><Table.Row><Table.HeaderCell>Direction</Table.HeaderCell><Table.HeaderCell>Exception</Table.HeaderCell><Table.HeaderCell /><Table.HeaderCell textAlign="right">Count</Table.HeaderCell><Table.HeaderCell textAlign="right">Distinct Users</Table.HeaderCell></Table.Row></Table.Header>
        <Table.Body>
          {loading && <Table.Row><Table.Cell colSpan={5} textAlign="center"><LoaderSegment size="small" /></Table.Cell></Table.Row>}
          {!loading && rows.length === 0 && <Table.Row><Table.Cell colSpan={5} style={{ opacity: 0.5, textAlign: 'center' }}>No data to display</Table.Cell></Table.Row>}
          {!loading && rows.map((row) => (
            <Table.Row key={`${row.direction}-${row.exception ?? ''}`}>
              <Table.Cell>{row.direction}</Table.Cell>
              <Table.Cell title={row.exception}>{truncate(row.exception, 80)}</Table.Cell>
              <Table.Cell><Progress color="red" percent={Math.round(((row.count ?? 0) / maxCount) * 100)} size="tiny" /></Table.Cell>
              <Table.Cell textAlign="right"><strong>{(row.count ?? 0).toLocaleString()}</strong></Table.Cell>
              <Table.Cell textAlign="right">{(row.distinctUsers ?? 0).toLocaleString()}</Table.Cell>
            </Table.Row>
          ))}
        </Table.Body>
      </Table>
    </>
  );
};

const ExceptionList = ({ direction, loading, onDirectionChange, rows }) => (
  <>
    <Header size="small"><Icon name="clipboard outline" /> Recent Errors <DirectionButtons direction={direction} onChange={onDirectionChange} /></Header>
    <Table
      className="unstackable"
      compact="very"
    >
      <Table.Header><Table.Row><Table.HeaderCell>Time</Table.HeaderCell><Table.HeaderCell>Direction</Table.HeaderCell><Table.HeaderCell>Username</Table.HeaderCell><Table.HeaderCell>Filename</Table.HeaderCell><Table.HeaderCell>Exception</Table.HeaderCell></Table.Row></Table.Header>
      <Table.Body>
        {loading && <Table.Row><Table.Cell colSpan={5} textAlign="center"><LoaderSegment size="small" /></Table.Cell></Table.Row>}
        {!loading && rows.length === 0 && <Table.Row><Table.Cell colSpan={5} style={{ opacity: 0.5, textAlign: 'center' }}>No data to display</Table.Cell></Table.Row>}
        {!loading && rows.map((row) => (
          <Table.Row key={`${row.direction}-${row.endedAt}-${row.filename}`}>
            <Table.Cell style={{ whiteSpace: 'nowrap' }}>{row.endedAt ? formatDate(row.endedAt) : ''}</Table.Cell>
            <Table.Cell>{row.direction}</Table.Cell>
            <Table.Cell>{row.username}</Table.Cell>
            <Table.Cell title={row.filename}>{row.filename ? getFileName(row.filename) : ''}</Table.Cell>
            <Table.Cell title={row.exception}>{truncate(row.exception, 80)}</Table.Cell>
          </Table.Row>
        ))}
      </Table.Body>
    </Table>
  </>
);

const mergeDirectionalRows = (uploadRows = [], downloadRows = []) => [
  ...uploadRows.map((row) => ({ ...row, direction: 'Upload' })),
  ...downloadRows.map((row) => ({ ...row, direction: 'Download' })),
].sort((left, right) => new Date(right.endedAt) - new Date(left.endedAt)).slice(0, 10);

const mergeParetoRows = (uploadRows = [], downloadRows = []) => [
  ...uploadRows.map((row) => ({ ...row, direction: 'Upload' })),
  ...downloadRows.map((row) => ({ ...row, direction: 'Download' })),
].sort((left, right) => (right.count ?? 0) - (left.count ?? 0)).slice(0, 10);

const TransferErrors = ({ data, end, start }) => {
  const [paretoDirection, setParetoDirection] = useState('All');
  const [recentDirection, setRecentDirection] = useState('All');
  const [paretoRows, setParetoRows] = useState(null);
  const [recentRows, setRecentRows] = useState(null);
  const [paretoLoading, setParetoLoading] = useState(false);
  const [recentLoading, setRecentLoading] = useState(false);
  const mountedRef = useRef(false);
  const paretoRequestRef = useRef(0);
  const recentRequestRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      paretoRequestRef.current += 1;
      recentRequestRef.current += 1;
    };
  }, []);

  useEffect(() => {
    paretoRequestRef.current += 1;
    recentRequestRef.current += 1;
    setParetoDirection('All');
    setRecentDirection('All');
    setParetoRows(null);
    setRecentRows(null);
    setParetoLoading(false);
    setRecentLoading(false);
  }, [end, start]);

  const fetchRows = async (kind, direction) => {
    const parameters = {
      direction,
      end: new Date(end),
      start: start ? new Date(start) : undefined,
    };
    try {
      return kind === 'pareto'
        ? asRows(await reports.getExceptionPareto(parameters))
        : asRows(await reports.getExceptions({ ...parameters, limit: 10 }));
    } catch (error) {
      if (mountedRef.current) {
        toast.error(toDisplayError(error, 'Failed to load transfer errors'));
      }
      return [];
    }
  };

  const updateRows = async (kind, direction) => {
    const setLoading = kind === 'pareto' ? setParetoLoading : setRecentLoading;
    const setRows = kind === 'pareto' ? setParetoRows : setRecentRows;
    const requestRef = kind === 'pareto' ? paretoRequestRef : recentRequestRef;
    const requestId = ++requestRef.current;
    setLoading(true);
    try {
      if (direction === 'All') {
        const [upload, download] = await Promise.all([
          fetchRows(kind, 'Upload'),
          fetchRows(kind, 'Download'),
        ]);
        if (mountedRef.current && requestId === requestRef.current) {
          setRows(kind === 'pareto' ? mergeParetoRows(upload, download) : mergeDirectionalRows(upload, download));
        }
      } else {
        const rows = await fetchRows(kind, direction);
        if (mountedRef.current && requestId === requestRef.current) {
          setRows(asRows(rows).map((row) => ({ ...row, direction })));
        }
      }
    } finally {
      if (mountedRef.current && requestId === requestRef.current) {
        setLoading(false);
      }
    }
  };

  const upload = data.upload ?? { pareto: [], recent: [] };
  const download = data.download ?? { pareto: [], recent: [] };
  return (
    <>
      <CompatibilityGraph
        data={data.chartData}
        defaultSeries={new Set(['uploadErrors', 'downloadErrors'])}
        series={[
          { color: '#db2828', format: (value) => value.toLocaleString(), key: 'uploadErrors', name: 'Upload Errors' },
          { color: '#a333c8', format: (value) => value.toLocaleString(), key: 'downloadErrors', name: 'Download Errors' },
        ]}
      />
      <ExceptionPareto
        direction={paretoDirection}
        loading={paretoLoading}
        onDirectionChange={(direction) => {
          setParetoDirection(direction);
          updateRows('pareto', direction);
        }}
        rows={paretoRows ?? mergeParetoRows(upload.pareto, download.pareto)}
      />
      <ExceptionList
        direction={recentDirection}
        loading={recentLoading}
        onDirectionChange={(direction) => {
          setRecentDirection(direction);
          updateRows('recent', direction);
        }}
        rows={recentRows ?? mergeDirectionalRows(upload.recent, download.recent)}
      />
    </>
  );
};

const HistoricalStatistics = ({ data, historyLabel, historyRanges, onHistoryRangeSelect }) => {
  const [activeTab, setActiveTab] = useState(0);
  const summary = data.summary ?? {};
  const downloadBytes = sumBytes(summary.Download ?? {});
  const uploadBytes = sumBytes(summary.Upload ?? {});
  const shareRatio = downloadBytes > 0 ? uploadBytes / downloadBytes : null;
  const chartData = useMemo(() => buildChartData(data.histogram), [data.histogram]);
  const downloadParts = formatBytesParts(downloadBytes);
  const uploadParts = formatBytesParts(uploadBytes);
  const panes = [
    {
      menuItem: { content: 'Users', icon: 'users', key: 'users' },
      render: () => <Tab.Pane><Leaderboard downloads={data.leaderboard.download} end={data.historyEnd} start={data.historyStart} uploads={data.leaderboard.upload} /></Tab.Pane>,
    },
    {
      menuItem: { content: 'Content', icon: 'folder open', key: 'content' },
      render: () => <Tab.Pane><TopDirectories rows={data.directories} /></Tab.Pane>,
    },
    {
      menuItem: { content: 'Errors', icon: 'warning sign', key: 'errors' },
      render: () => <Tab.Pane><TransferErrors data={{ ...data.exceptions, chartData }} end={data.historyEnd} start={data.historyStart} /></Tab.Pane>,
    },
  ];

  return (
    <Segment>
      <div className="compatibility-history-heading">
        <Header as="h4"><Icon name="history" /> History</Header>
        <ButtonGroup size="mini">
          {historyRanges.map((range) => (
            <Button
              active={historyLabel === range.label}
              key={range.label}
              onClick={() => onHistoryRangeSelect(range.label)}
            >
              {range.label}
            </Button>
          ))}
        </ButtonGroup>
      </div>
      <Statistic.Group
        size="small"
        widths="four"
      >
        <Statistic color="blue">
          <Statistic.Value><Icon name="arrow down" size="tiny" />{downloadParts.value}<small>{downloadParts.unit}</small></Statistic.Value>
          <Statistic.Label>Downloaded · {sumCounts(summary.Download ?? {}).toLocaleString()} files</Statistic.Label>
        </Statistic>
        <Statistic color="green">
          <Statistic.Value><Icon name="arrow up" size="tiny" />{uploadParts.value}<small>{uploadParts.unit}</small></Statistic.Value>
          <Statistic.Label>Uploaded · {sumCounts(summary.Upload ?? {}).toLocaleString()} files</Statistic.Label>
        </Statistic>
        <Statistic color={shareRatio === null ? 'grey' : shareRatio > 0.66 ? 'green' : shareRatio >= 0.33 ? 'yellow' : 'red'}>
          <Statistic.Value><Icon name="chart pie" size="tiny" />{shareRatio === null ? '—' : shareRatio.toFixed(2)}</Statistic.Value>
          <Statistic.Label>Share ratio (↑/↓)</Statistic.Label>
        </Statistic>
        <Statistic>
          <Statistic.Value><Icon name="user" size="tiny" />{((summary.Upload?.Succeeded?.distinctUsers ?? 0) + (summary.Download?.Succeeded?.distinctUsers ?? 0)).toLocaleString()}</Statistic.Value>
          <Statistic.Label>Distinct peers</Statistic.Label>
        </Statistic>
      </Statistic.Group>
      <CompatibilityGraph
        data={chartData}
        defaultSeries={new Set(['uploadBytes', 'downloadBytes'])}
        series={HISTORY_SERIES}
      />
      <Tab
        activeIndex={activeTab}
        onTabChange={(_, { activeIndex }) => setActiveTab(activeIndex)}
        panes={panes}
      />
    </Segment>
  );
};

const CompatibilityDashboard = ({ runtimeProfile, server } = {}) => {
  const [historyLabel, setHistoryLabel] = useState('30d');
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState(EMPTY_REPORT);
  const [error, setError] = useState(null);
  const mountedRef = useMountedRef();
  const range = HISTORY_RANGES.find((item) => item.label === historyLabel) ?? HISTORY_RANGES[2];
  const historyParameters = useMemo(() => {
    const end = new Date();
    return {
      buckets: range.buckets,
      end: end.toISOString(),
      start: range.days == null ? new Date(0).toISOString() : new Date(end.getTime() - range.days * 86_400_000).toISOString(),
    };
  }, [range]);

  useEffect(() => {
    let active = true;
    const reportFailures = [];
    const safeRequest = (request, fallback) =>
      Promise.resolve()
        .then(request)
        .catch((loadError) => {
          reportFailures.push(loadError);
          return fallback;
        });

    if (mountedRef.current) {
      setError(null);
      setLoading(true);
    }
    const start = new Date(historyParameters.start);
    const end = new Date(historyParameters.end);
    Promise.all([
      safeRequest(() => reports.getSummary({ end, start }), {}),
      safeRequest(
        () => reports.getHistogram({ buckets: historyParameters.buckets, end, start }),
        {},
      ),
      safeRequest(
        () => reports.getLeaderboard({ direction: 'Upload', end, start }),
        [],
      ),
      safeRequest(
        () => reports.getLeaderboard({ direction: 'Download', end, start }),
        [],
      ),
      safeRequest(() => reports.getTopDirectories({ end, start }), []),
      safeRequest(
        () => reports.getExceptionPareto({ direction: 'Upload', end, start }),
        [],
      ),
      safeRequest(
        () => reports.getExceptionPareto({ direction: 'Download', end, start }),
        [],
      ),
      safeRequest(
        () => reports.getExceptions({ direction: 'Upload', end, start }),
        [],
      ),
      safeRequest(
        () => reports.getExceptions({ direction: 'Download', end, start }),
        [],
      ),
    ]).then(([summary, histogram, uploadLeaderboard, downloadLeaderboard, directories, uploadPareto, downloadPareto, uploadRecent, downloadRecent]) => {
      if (!active || !mountedRef.current) return;
      setData({
        ...normalizeReport({
          directories,
          downloadLeaderboard,
          downloadPareto,
          downloadRecent,
          histogram,
          summary,
          uploadLeaderboard,
          uploadPareto,
          uploadRecent,
        }),
        historyEnd: historyParameters.end,
        historyStart: historyParameters.start,
      });
      setError(
        reportFailures.length > 0
          ? `Some transfer reports could not be loaded: ${toDisplayError(
              reportFailures[0],
              'report service unavailable',
            )}`
          : null,
      );
      setLoading(false);
    }).catch((loadError) => {
      if (!active || !mountedRef.current) return;
      setError(toDisplayError(loadError, 'Unable to load transfer reports'));
      setLoading(false);
    });
    return () => {
      active = false;
    };
  }, [historyParameters, mountedRef]);

  return (
    <div className="view dashboard compatibility-dashboard">
      <SearchBar server={server} />
      {error && <Message negative>{error}</Message>}
      {runtimeProfile === 'legacy' ? (
        <Segment>
          <Header as="h4">
            <Icon name="history" />
            History
          </Header>
          <ButtonGroup size="mini">
            {HISTORY_RANGES.map((historyRange) => (
              <Button
                active={historyLabel === historyRange.label}
                key={historyRange.label}
                onClick={() => setHistoryLabel(historyRange.label)}
              >
                {historyRange.label}
              </Button>
            ))}
          </ButtonGroup>
        </Segment>
      ) : loading ? (
        <LoaderSegment />
      ) : (
        <HistoricalStatistics
          data={data}
          historyLabel={historyLabel}
          historyRanges={HISTORY_RANGES}
          onHistoryRangeSelect={setHistoryLabel}
        />
      )}
    </div>
  );
};

export default CompatibilityDashboard;
