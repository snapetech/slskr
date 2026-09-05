import * as bridge from '../../../lib/bridge';
import { toDisplayError } from '../../../lib/errors';
import { usePolling } from '../../../lib/usePolling';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Button,
  Card,
  Checkbox,
  Form,
  Grid,
  Header,
  Icon,
  Input,
  Label,
  Loader,
  Message,
  Segment,
  Statistic,
  Table,
} from 'semantic-ui-react';

const isRecord = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

const toNonNegativeNumber = (value) => {
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 ? number : 0;
};

const normalizeConfig = (value) => {
  if (!isRecord(value)) return null;
  return {
    ...value,
    enabled: value.enabled === true,
    max_clients: Math.max(
      1,
      Math.floor(toNonNegativeNumber(value.max_clients)) || 10,
    ),
    port: Math.max(
      1,
      Math.floor(toNonNegativeNumber(value.port)) || 2_242,
    ),
    require_auth: value.require_auth === true,
    soulfind_path:
      typeof value.soulfind_path === 'string' && value.soulfind_path
        ? value.soulfind_path
        : 'soulfind',
  };
};

const normalizeDashboard = (value) => {
  if (!isRecord(value)) return null;
  const health = isRecord(value.health) ? value.health : {};
  const healthStatus =
    typeof value.health === 'string' ? value.health.toLowerCase() : '';
  const stats = isRecord(value.stats) ? value.stats : {};
  const meshBenefits = isRecord(value.meshBenefits)
    ? value.meshBenefits
    : {};
  const connectedClients = Array.isArray(value.connectedClients)
    ? value.connectedClients.filter(isRecord).map((client, index) => ({
        ...client,
        clientId:
          typeof client.clientId === 'string' && client.clientId
            ? client.clientId
            : 'client-' + index,
        clientType:
          typeof client.clientType === 'string' ? client.clientType : 'Unknown',
        ipAddress:
          typeof client.ipAddress === 'string' ? client.ipAddress : 'Unknown',
        requestCount: toNonNegativeNumber(client.requestCount),
      }))
    : [];

  return {
    ...value,
    connectedClients,
    health: {
      ...health,
      isHealthy:
        health.isHealthy === true ||
        ['healthy', 'running', 'active'].includes(healthStatus),
      version:
        typeof health.version === 'string' ? health.version : '',
    },
    meshBenefits: {
      ...meshBenefits,
      bytesViaMesh: toNonNegativeNumber(meshBenefits.bytesViaMesh),
      meshPercentage: toNonNegativeNumber(meshBenefits.meshPercentage),
    },
    stats: {
      ...stats,
      currentConnections: toNonNegativeNumber(stats.currentConnections),
      totalBytesProxied: toNonNegativeNumber(stats.totalBytesProxied),
      totalDownloads: toNonNegativeNumber(stats.totalDownloads),
      totalSearches: toNonNegativeNumber(stats.totalSearches),
    },
  };
};

const Bridge = () => {
  const [config, setConfig] = useState(null);
  const [dashboard, setDashboard] = useState(null);
  const [dashboardError, setDashboardError] = useState(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState(null);
  const [success, setSuccess] = useState(null);
  const [controlAction, setControlAction] = useState('');
  const mountedRef = useMountedRef();
  const dashboardRequestIdRef = useRef(0);
  const loadRequestIdRef = useRef(0);
  const saveRequestIdRef = useRef(0);
  const controlRequestIdRef = useRef(0);
  const saveInFlightRef = useRef(false);
  const controlInFlightRef = useRef(false);

  const fetchDashboard = useCallback(async () => {
    if (!mountedRef.current) return;
    const requestId = ++dashboardRequestIdRef.current;
    try {
      const dashboardData = await bridge.getDashboard();
      if (
        mountedRef.current &&
        dashboardRequestIdRef.current === requestId
      ) {
        setDashboard(normalizeDashboard(dashboardData));
        setDashboardError(null);
      }
    } catch (error_) {
      if (
        mountedRef.current &&
        dashboardRequestIdRef.current === requestId
      ) {
        setDashboardError(
          toDisplayError(error_, 'Live bridge dashboard unavailable'),
        );
      }
    }
  }, [mountedRef]);

  useEffect(() => {
    const requestId = ++loadRequestIdRef.current;
    const fetchData = async () => {
      if (!mountedRef.current) return;
      try {
        setLoading(true);
        setError(null);
        const [configData, dashboardData] = await Promise.all([
          bridge.getConfig(),
          bridge.getDashboard(),
        ]);
        if (
          !mountedRef.current ||
          loadRequestIdRef.current !== requestId
        ) {
          return;
        }
        setConfig(normalizeConfig(configData));
        setDashboard(normalizeDashboard(dashboardData));
        setDashboardError(null);
      } catch (error_) {
        if (
          mountedRef.current &&
          loadRequestIdRef.current === requestId
        ) {
          setError(toDisplayError(error_, 'Failed to load bridge configuration'));
        }
      } finally {
        if (
          mountedRef.current &&
          loadRequestIdRef.current === requestId
        ) {
          setLoading(false);
        }
      }
    };

    void fetchData();

    return () => {
      loadRequestIdRef.current += 1;
    };
  }, [mountedRef]);

  usePolling(fetchDashboard, 10_000, { immediate: false });

  const handleConfigChange = (field, value) => {
    setConfig((previous) =>
      previous ? { ...previous, [field]: value } : previous,
    );
  };

  const handleSaveConfig = async () => {
    if (
      !mountedRef.current ||
      saving ||
      saveInFlightRef.current ||
      !config
    ) {
      return;
    }
    saveInFlightRef.current = true;
    const requestId = ++saveRequestIdRef.current;
    try {
      setSaving(true);
      setError(null);
      setSuccess(null);
      await bridge.updateConfig(config);
      if (
        mountedRef.current &&
        saveRequestIdRef.current === requestId
      ) {
        setSuccess(
          'Configuration updated. Restart bridge service to apply changes.',
        );
      }
    } catch (error_) {
      if (
        mountedRef.current &&
        saveRequestIdRef.current === requestId
      ) {
        setError(toDisplayError(error_, 'Failed to save bridge configuration'));
      }
    } finally {
      saveInFlightRef.current = false;
      if (
        mountedRef.current &&
        saveRequestIdRef.current === requestId
      ) {
        setSaving(false);
      }
    }
  };

  const handleStartBridge = async () => {
    if (
      !mountedRef.current ||
      controlAction ||
      controlInFlightRef.current
    ) {
      return;
    }
    controlInFlightRef.current = true;
    const requestId = ++controlRequestIdRef.current;
    const isCurrentRequest = () =>
      mountedRef.current && controlRequestIdRef.current === requestId;
    try {
      setControlAction('start');
      setError(null);
      await bridge.startBridge();
      // Refresh dashboard
      const dashboardData = await bridge.getDashboard();
      if (isCurrentRequest()) setDashboard(normalizeDashboard(dashboardData));
    } catch (error_) {
      if (isCurrentRequest()) {
        setError(toDisplayError(error_, 'Failed to start bridge'));
      }
    } finally {
      controlInFlightRef.current = false;
      if (isCurrentRequest()) setControlAction('');
    }
  };

  const handleStopBridge = async () => {
    if (
      !mountedRef.current ||
      controlAction ||
      controlInFlightRef.current
    ) {
      return;
    }
    controlInFlightRef.current = true;
    const requestId = ++controlRequestIdRef.current;
    const isCurrentRequest = () =>
      mountedRef.current && controlRequestIdRef.current === requestId;
    try {
      setControlAction('stop');
      setError(null);
      await bridge.stopBridge();
      // Refresh dashboard
      const dashboardData = await bridge.getDashboard();
      if (isCurrentRequest()) setDashboard(normalizeDashboard(dashboardData));
    } catch (error_) {
      if (isCurrentRequest()) {
        setError(toDisplayError(error_, 'Failed to stop bridge'));
      }
    } finally {
      controlInFlightRef.current = false;
      if (isCurrentRequest()) setControlAction('');
    }
  };

  if (loading && !config) {
    return (
      <Segment>
        <Loader
          active
          inline="centered"
        >
          Loading bridge configuration...
        </Loader>
      </Segment>
    );
  }

  const health = dashboard?.health;
  const stats = dashboard?.stats;
  const clients = dashboard?.connectedClients || [];
  const meshBenefits = dashboard?.meshBenefits;

  return (
    <div>
      <Header as="h2">
        <Icon name="exchange" />
        Legacy Client Bridge
      </Header>

      {error && (
        <Message error>
          <Message.Header>Error</Message.Header>
          <p>{error}</p>
        </Message>
      )}

      {success && (
        <Message success>
          <Message.Header>Success</Message.Header>
          <p>{success}</p>
        </Message>
      )}

      {dashboardError && (
        <Message
          data-testid="bridge-dashboard-error"
          warning
        >
          <Message.Header>Live bridge dashboard unavailable</Message.Header>
          <p>{dashboardError}</p>
          <p>Showing the last successfully received dashboard values.</p>
        </Message>
      )}

      <Grid stackable>
        {/* Configuration */}
        <Grid.Column width={16}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="cog" />
                Configuration
              </Card.Header>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Group widths="equal">
                  <Form.Field>
                    <Checkbox
                      checked={config?.enabled || false}
                      label="Enable Bridge"
                      onChange={(e, { checked }) =>
                        handleConfigChange('enabled', checked)
                      }
                      toggle
                    />
                    <small>
                      Allow legacy Soulseek clients to connect via bridge
                    </small>
                  </Form.Field>
                </Form.Group>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Port</label>
                    <Input
                      disabled={!config?.enabled}
                      onChange={(e, { value }) =>
                        handleConfigChange(
                          'port',
                          Number.parseInt(value, 10) || 2_242,
                        )
                      }
                      type="number"
                      value={config?.port || 2_242}
                    />
                    <small>Soulseek protocol port (default: 2242)</small>
                  </Form.Field>
                  <Form.Field>
                    <label>Soulfind Path</label>
                    <Input
                      disabled={!config?.enabled}
                      onChange={(e, { value }) =>
                        handleConfigChange('soulfind_path', value)
                      }
                      placeholder="soulfind"
                      value={config?.soulfind_path || 'soulfind'}
                    />
                    <small>Path to Soulfind binary</small>
                  </Form.Field>
                </Form.Group>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Max Clients</label>
                    <Input
                      disabled={!config?.enabled}
                      max={50}
                      min={1}
                      onChange={(e, { value }) =>
                        handleConfigChange(
                          'max_clients',
                          Number.parseInt(value, 10) || 10,
                        )
                      }
                      type="number"
                      value={config?.max_clients || 10}
                    />
                    <small>Maximum concurrent legacy clients</small>
                  </Form.Field>
                  <Form.Field>
                    <Checkbox
                      checked={config?.require_auth || false}
                      disabled={!config?.enabled}
                      label="Require Authentication"
                      onChange={(e, { checked }) =>
                        handleConfigChange('require_auth', checked)
                      }
                      toggle
                    />
                    <small>Require password for bridge connections</small>
                  </Form.Field>
                </Form.Group>
                <Button
                  disabled={saving}
                  loading={saving}
                  onClick={handleSaveConfig}
                  primary
                >
                  Save Configuration
                </Button>
              </Form>
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Service Control */}
        <Grid.Column width={16}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="power" />
                Service Control
              </Card.Header>
            </Card.Content>
            <Card.Content>
              <div
                style={{ alignItems: 'center', display: 'flex', gap: '10px' }}
              >
                <Button
                  color="green"
                  disabled={health?.isHealthy || Boolean(controlAction)}
                  loading={controlAction === 'start'}
                  onClick={handleStartBridge}
                >
                  <Icon name="play" />
                  Start Bridge
                </Button>
                <Button
                  color="red"
                  disabled={!health?.isHealthy || Boolean(controlAction)}
                  loading={controlAction === 'stop'}
                  onClick={handleStopBridge}
                >
                  <Icon name="stop" />
                  Stop Bridge
                </Button>
                <Label
                  color={health?.isHealthy ? 'green' : 'red'}
                  size="large"
                >
                  <Icon name={health?.isHealthy ? 'checkmark' : 'remove'} />
                  {health?.isHealthy ? 'Running' : 'Stopped'}
                </Label>
                {health?.version && <Label>Version: {health.version}</Label>}
              </div>
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Statistics */}
        {stats && (
          <Grid.Column width={16}>
            <Card fluid>
              <Card.Content>
                <Card.Header>
                  <Icon name="chart bar" />
                  Statistics
                </Card.Header>
              </Card.Content>
              <Card.Content>
                <Statistic.Group size="small">
                  <Statistic>
                    <Statistic.Value>
                      {stats.currentConnections || 0}
                    </Statistic.Value>
                    <Statistic.Label>Active Connections</Statistic.Label>
                  </Statistic>
                  <Statistic>
                    <Statistic.Value>
                      {stats.totalSearches || 0}
                    </Statistic.Value>
                    <Statistic.Label>Total Searches</Statistic.Label>
                  </Statistic>
                  <Statistic>
                    <Statistic.Value>
                      {stats.totalDownloads || 0}
                    </Statistic.Value>
                    <Statistic.Label>Total Downloads</Statistic.Label>
                  </Statistic>
                  <Statistic>
                    <Statistic.Value>
                      {(stats.totalBytesProxied / 1_024 / 1_024).toFixed(2)}
                    </Statistic.Value>
                    <Statistic.Label>MB Proxied</Statistic.Label>
                  </Statistic>
                </Statistic.Group>
              </Card.Content>
            </Card>
          </Grid.Column>
        )}

        {/* Mesh Benefits */}
        {meshBenefits && (
          <Grid.Column width={8}>
            <Card fluid>
              <Card.Content>
                <Card.Header>
                  <Icon name="sitemap" />
                  Mesh Benefits
                </Card.Header>
              </Card.Content>
              <Card.Content>
                <Statistic.Group size="small">
                  <Statistic>
                    <Statistic.Value>
                      {meshBenefits.meshPercentage.toFixed(1)}%
                    </Statistic.Value>
                    <Statistic.Label>Via Mesh</Statistic.Label>
                  </Statistic>
                  <Statistic>
                    <Statistic.Value>
                      {(meshBenefits.bytesViaMesh / 1_024 / 1_024).toFixed(2)}
                    </Statistic.Value>
                    <Statistic.Label>MB via Mesh</Statistic.Label>
                  </Statistic>
                </Statistic.Group>
              </Card.Content>
            </Card>
          </Grid.Column>
        )}

        {/* Connected Clients */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="users" />
                Connected Clients ({clients.length})
              </Card.Header>
            </Card.Content>
            <Card.Content>
              {clients.length === 0 ? (
                <Message info>No clients connected</Message>
              ) : (
                <Table
                  compact
                  size="small"
                >
                  <Table.Header>
                    <Table.Row>
                      <Table.HeaderCell>Client</Table.HeaderCell>
                      <Table.HeaderCell>IP</Table.HeaderCell>
                      <Table.HeaderCell>Requests</Table.HeaderCell>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {clients.map((client, index) => (
                      <Table.Row key={client.clientId || 'client-' + index}>
                        <Table.Cell>{client.clientType}</Table.Cell>
                        <Table.Cell>{client.ipAddress}</Table.Cell>
                        <Table.Cell>{client.requestCount}</Table.Cell>
                      </Table.Row>
                    ))}
                  </Table.Body>
                </Table>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>
      </Grid>
    </div>
  );
};

export default Bridge;
