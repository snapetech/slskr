import '../System.css';
import { createLogsHubConnection } from '../../../lib/hubFactory';
import { toDisplayError } from '../../../lib/errors';
import { getLogs, updateLogLevel } from '../../../lib/options';
import { LoaderSegment } from '../../Shared';
import React, { Component } from 'react';
import {
  Button,
  ButtonGroup,
  Dropdown,
  Message,
  Table,
} from 'semantic-ui-react';

const initialState = {
  connected: false,
  error: null,
  filterLevel: 'all',
  level: 'Information',
  levels: ['Trace', 'Debug', 'Information', 'Warning', 'Error'],
  loading: true,
  logs: [],
  savingLevel: false,
};

const levels = {
  Debug: 'DBG',
  Error: 'ERR',
  Information: 'INF',
  Warning: 'WRN',
};

const maxLogs = 500;

class Logs extends Component {
  constructor(props) {
    super(props);

    this.state = initialState;
    this.isMountedFlag = false;
    this.logsRequestId = 0;
    this.levelRequestId = 0;
  }

  componentDidMount() {
    this.isMountedFlag = true;
    void this.fetchLogs();
    const logsHub = createLogsHubConnection();
    this.logsHub = logsHub;

    logsHub.on('buffer', (buffer) => this.mergeLogs(buffer));

    logsHub.on('log', (log) => {
      if (!this.isMountedFlag) return;
      this.setState((previousState) => ({
        connected: true,
        logs: this.dedupeLogs([this.normalizeLog(log), ...previousState.logs]),
      }));
    });

    logsHub.onreconnecting(() => {
      if (this.isMountedFlag) this.setState({ connected: false });
    });
    logsHub.onclose((error) => {
      if (this.isMountedFlag) this.setState({ connected: false });
      if (error) {
        console.error('[Logs] Hub connection closed with error:', error);
      }
    });
    logsHub.onreconnected(() => {
      if (this.isMountedFlag) this.setState({ connected: true });
    });

    logsHub.start().catch((error) => {
      console.error('[Logs] Failed to start hub connection:', error);
      if (this.isMountedFlag) this.setState({ connected: false });
    });
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    this.logsRequestId += 1;
    this.levelRequestId += 1;
    this.logsHub?.stop();
  }

  fetchLogs = async () => {
    const requestId = ++this.logsRequestId;
    try {
      const response = await getLogs();
      const fetchedLogs = (Array.isArray(response?.entries) ? response.entries : [])
        .map(this.normalizeLog)
        .slice(0, maxLogs);
      if (
        this.isMountedFlag &&
        requestId === this.logsRequestId
      ) {
        this.setState((previousState) => ({
          error: null,
          level: response?.level || 'Information',
          levels: Array.isArray(response?.levels)
            ? response.levels
            : initialState.levels,
          loading: false,
          logs: this.dedupeLogs([...previousState.logs, ...fetchedLogs]),
        }));
      }
    } catch (error) {
      console.error('[Logs] Failed to fetch logs:', error);
      if (
        this.isMountedFlag &&
        requestId === this.logsRequestId
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to load logs'),
          loading: false,
        });
      }
    }
  };

  normalizeLog = (log = {}) => {
    const payloadCandidate = log?.payload || log?.data || log;
    const payload =
      payloadCandidate &&
      typeof payloadCandidate === 'object' &&
      !Array.isArray(payloadCandidate)
        ? payloadCandidate
        : { message: String(payloadCandidate ?? '') };
    const category = payload.category || payload.resource || 'daemon';
    const message = payload.message || payload.detail || payload.kind || '';
    const timestamp = payload.timestamp || payload.created_at || Date.now() / 1000;
    return {
      ...payload,
      category,
      id: payload.id || `${timestamp}:${category}:${message}`,
      level: payload.level || 'Information',
      message,
      timestamp,
    };
  };

  dedupeLogs = (logs) => {
    const seen = new Set();
    return logs
      .filter(Boolean)
      .filter((log) => {
        const key = log.id ?? `${log.timestamp}:${log.category}:${log.message}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      })
      .slice(0, maxLogs);
  };

  mergeLogs = (buffer = []) => {
    if (!this.isMountedFlag) return;
    const entries = Array.isArray(buffer) ? buffer : [];
    this.setState((previousState) => ({
      connected: true,
      logs: this.dedupeLogs([
        ...entries.map(this.normalizeLog).reverse(),
        ...previousState.logs,
      ]),
    }));
  };

  formatTimestamp = (timestamp) => {
    const date = new Date(
      Number(timestamp) < 10_000_000_000
        ? Number(timestamp) * 1000
        : Number(timestamp),
    );
    return `${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}:${date.getSeconds().toString().padStart(2, '0')}`; // eslint-disable-line max-len
  };

  handleFilterChange = (level) => {
    this.setState({ filterLevel: level });
  };

  getFilteredLogs = () => {
    const { filterLevel, logs } = this.state;
    if (filterLevel === 'all') {
      return logs;
    }

    return logs.filter((log) => log.level === filterLevel);
  };

  handleLevelChange = async (_, { value }) => {
    const requestId = ++this.levelRequestId;
    this.setState({ savingLevel: true });
    try {
      const response = await updateLogLevel(value);
      if (
        !this.isMountedFlag ||
        requestId !== this.levelRequestId
      ) {
        return;
      }
      this.setState({ level: response?.level || value, savingLevel: false });
      await this.fetchLogs();
    } catch (error) {
      console.error('[Logs] Failed to update log level:', error);
      if (
        this.isMountedFlag &&
        requestId === this.levelRequestId
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to update log level'),
          savingLevel: false,
        });
      }
    }
  };

  render() {
    const {
      connected,
      error,
      filterLevel,
      level,
      levels: levelOptions,
      loading,
      savingLevel,
    } = this.state;
    const filteredLogs = this.getFilteredLogs();
    const dropdownOptions = levelOptions.map((option) => ({
      key: option,
      text: option,
      value: option,
    }));

    return (
      <div className="logs">
        <div style={{ marginBottom: '1em' }}>
          {/* The one place severity color earns its keep: the selected filter
              picks up the same color its rows render with below. */}
          <ButtonGroup>
            <Button
              active={filterLevel === 'all'}
              onClick={() => this.handleFilterChange('all')}
            >
              All
            </Button>
            <Button
              active={filterLevel === 'Information'}
              color={filterLevel === 'Information' ? 'blue' : undefined}
              onClick={() => this.handleFilterChange('Information')}
            >
              Info
            </Button>
            <Button
              active={filterLevel === 'Warning'}
              color={filterLevel === 'Warning' ? 'yellow' : undefined}
              onClick={() => this.handleFilterChange('Warning')}
            >
              Warn
            </Button>
            <Button
              active={filterLevel === 'Error'}
              color={filterLevel === 'Error' ? 'red' : undefined}
              onClick={() => this.handleFilterChange('Error')}
            >
              Error
            </Button>
            <Button
              active={filterLevel === 'Debug'}
              onClick={() => this.handleFilterChange('Debug')}
            >
              Debug
            </Button>
          </ButtonGroup>
          <Dropdown
            compact
            disabled={savingLevel}
            loading={savingLevel}
            onChange={this.handleLevelChange}
            options={dropdownOptions}
            selection
            style={{ marginLeft: '1em' }}
            value={level}
          />
          <span style={{ color: '#666', marginLeft: '1em' }}>
            {connected
              ? `Showing ${filteredLogs.length} of ${this.state.logs.length} logs`
              : 'Connecting to logs...'}
          </span>
        </div>
        {error && (
          <Message
            data-testid="logs-error"
            error
          >
            <Message.Header>Logs unavailable</Message.Header>
            <p>{error}</p>
          </Message>
        )}
        {loading && <LoaderSegment />}
        {!loading && (
          <Table
            className="logs-table"
            compact="very"
          >
            <Table.Header>
              <Table.Row>
                <Table.HeaderCell>Timestamp</Table.HeaderCell>
                <Table.HeaderCell>Level</Table.HeaderCell>
                <Table.HeaderCell>Category</Table.HeaderCell>
                <Table.HeaderCell>Message</Table.HeaderCell>
              </Table.Row>
            </Table.Header>
            <Table.Body className="logs-table-body">
              {filteredLogs.length === 0 ? (
                  <Table.Row>
                    <Table.Cell
                    colSpan="4"
                    textAlign="center"
                  >
                    {error
                      ? 'No logs are available from the server'
                      : 'No logs match the selected filter'}
                  </Table.Cell>
                </Table.Row>
              ) : (
                filteredLogs.map((log) => (
                  <Table.Row
                    disabled={log.level === 'Debug' && filterLevel !== 'Debug'}
                    key={log.id}
                    negative={log.level === 'Error'}
                    warning={log.level === 'Warning'}
                  >
                    <Table.Cell>
                      {this.formatTimestamp(log.timestamp)}
                    </Table.Cell>
                    <Table.Cell>{levels[log.level] || log.level}</Table.Cell>
                    <Table.Cell>{log.category}</Table.Cell>
                    <Table.Cell className="logs-table-message">
                      {log.message}
                    </Table.Cell>
                  </Table.Row>
                ))
              )}
            </Table.Body>
          </Table>
        )}
      </div>
    );
  }
}

export default Logs;
