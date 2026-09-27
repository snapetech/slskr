import { formatBytes, formatSpeed, formatWait } from '../lib/util';
import React, { useMemo, useState } from 'react';
import { Segment } from 'semantic-ui-react';

export const isRecord = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

export const sumCounts = (directionData = {}) =>
  Object.values(isRecord(directionData) ? directionData : {}).reduce(
    (sum, state) => sum + (Number(state?.count) || 0),
    0,
  );

export const sumBytes = (directionData = {}) =>
  Object.values(isRecord(directionData) ? directionData : {}).reduce(
    (sum, state) => sum + (Number(state?.totalBytes) || 0),
    0,
  );

const errorCount = (directionData = {}) =>
  (Number(directionData?.Errored?.count) || 0) +
  (Number(directionData?.Cancelled?.count) || 0) +
  (Number(directionData?.TimedOut?.count) || 0);

export const buildChartData = (histogram = {}) =>
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

export const HISTORY_SERIES = [
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

export default CompatibilityGraph;
