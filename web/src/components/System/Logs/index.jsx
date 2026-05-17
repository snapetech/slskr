import '../System.css';
import { createLogsHubConnection } from '../../../lib/hubFactory';
import { getLogs, updateLogLevel } from '../../../lib/options';
import { LoaderSegment } from '../../Shared';
import React, { Component } from 'react';
import { Button, ButtonGroup, Dropdown, Table } from 'semantic-ui-react';

const initialState = {
  connected: false,
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
  }

  componentDidMount() {
    this.fetchLogs();
    const logsHub = createLogsHubConnection();
    this.logsHub = logsHub;

    logsHub.on('buffer', (buffer) => this.mergeLogs(buffer));

    logsHub.on('log', (log) => {
      this.setState((previousState) => ({
        connected: true,
        logs: this.dedupeLogs([this.normalizeLog(log), ...previousState.logs]),
      }));
    });

    logsHub.onreconnecting(() => this.setState({ connected: false }));
    logsHub.onclose((error) => {
      this.setState({ connected: false });
      if (error) {
        console.error('[Logs] Hub connection closed with error:', error);
      }
    });
    logsHub.onreconnected(() => this.setState({ connected: true }));

    logsHub.start().catch((error) => {
      console.error('[Logs] Failed to start hub connection:', error);
      this.setState({ connected: false });
    });
  }

  componentWillUnmount() {
    this.logsHub?.stop();
  }

  fetchLogs = async () => {
    try {
      const response = await getLogs();
      this.setState({
        level: response.level || 'Information',
        levels: response.levels || initialState.levels,
        loading: false,
        logs: (response.entries || []).map(this.normalizeLog).slice(0, maxLogs),
      });
    } catch (error) {
      console.error('[Logs] Failed to fetch logs:', error);
      this.setState({ loading: false });
    }
  };

  normalizeLog = (log = {}) => {
    const payload = log.payload || log.data || log;
    return {
      ...payload,
      category: payload.category || payload.resource || 'daemon',
      id: payload.id || payload.timestamp || `${Date.now()}-${Math.random()}`,
      level: payload.level || 'Information',
      message: payload.message || payload.detail || payload.kind || '',
      timestamp: payload.timestamp || payload.created_at || Date.now() / 1000,
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
    this.setState((previousState) => ({
      connected: true,
      logs: this.dedupeLogs([
        ...buffer.map(this.normalizeLog).reverse(),
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
    this.setState({ savingLevel: true });
    try {
      const response = await updateLogLevel(value);
      this.setState({ level: response.level || value, savingLevel: false });
      await this.fetchLogs();
    } catch (error) {
      console.error('[Logs] Failed to update log level:', error);
      this.setState({ savingLevel: false });
    }
  };

  render() {
    const { connected, filterLevel, level, levels: levelOptions, loading, savingLevel } = this.state;
    const filteredLogs = this.getFilteredLogs();
    const dropdownOptions = levelOptions.map((option) => ({
      key: option,
      text: option,
      value: option,
    }));

    return (
      <div className="logs">
        <div style={{ marginBottom: '1em' }}>
          <ButtonGroup>
            <Button
              active={filterLevel === 'all'}
              onClick={() => this.handleFilterChange('all')}
            >
              All
            </Button>
            <Button
              active={filterLevel === 'Information'}
              onClick={() => this.handleFilterChange('Information')}
            >
              Info
            </Button>
            <Button
              active={filterLevel === 'Warning'}
              onClick={() => this.handleFilterChange('Warning')}
            >
              Warn
            </Button>
            <Button
              active={filterLevel === 'Error'}
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
                    No logs match the selected filter
                  </Table.Cell>
                </Table.Row>
              ) : (
                filteredLogs.map((log) => (
                  <Table.Row
                    disabled={log.level === 'Debug' && filterLevel !== 'Debug'}
                    key={log.timestamp}
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
