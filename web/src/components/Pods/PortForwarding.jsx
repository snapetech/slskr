import { urlBase } from '../../config';
import { toDisplayError } from '../../lib/errors';
import * as pods from '../../lib/pods';
import * as portForwarding from '../../lib/portForwarding';
import { createPollingController } from '../../lib/usePolling';
import React, { Component } from 'react';
import {
  Button,
  Card,
  Dimmer,
  Dropdown,
  Form,
  Icon,
  Input,
  Label,
  List,
  Loader,
  Message,
  Modal,
  Popup,
  Segment,
  Statistic,
  Tab,
  Table,
} from 'semantic-ui-react';

const initialState = {
  activeTab: 0,
  availablePorts: [],
  createForm: {
    destinationHost: '',
    destinationPort: '',
    localPort: '',
    serviceName: '',
  },
  creatingForwarding: false,
  error: null,
  forwardingStatus: [],
  loading: false,
  pods: [],
  selectedPodDetail: null,
  selectedPodId: null,
  showCreateModal: false,
  stoppingForwarding: false,
  success: null,
  vpnPodStatus: {},
};

const nonNegativeNumber = (value) => {
  const number = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(number) && number >= 0 ? number : null;
};

const buildTunnelStats = (forwardingStatus) =>
  forwardingStatus.reduce((stats, forwarding) => {
    const bytesIn = nonNegativeNumber(forwarding.bytesIn);
    const bytesOut = nonNegativeNumber(forwarding.bytesOut);
    const startedAt = nonNegativeNumber(forwarding.startedAt);
    const lastActivity = nonNegativeNumber(forwarding.lastActivity);

    stats[forwarding.localPort] = {
      bytesIn,
      bytesOut,
      connections: nonNegativeNumber(forwarding.activeConnections) ?? 0,
      lastActivity,
      localPort: forwarding.localPort,
      uptime:
        startedAt === null ? null : Math.max(0, Date.now() - startedAt),
    };

    return stats;
  }, {});

class PortForwarding extends Component {
  constructor(props) {
    super(props);
    this.state = initialState;
    this.isMountedFlag = false;
    this.requestIds = {
      availablePorts: 0,
      createForwarding: 0,
      forwardingStatus: 0,
      podDetails: 0,
      pods: 0,
      vpnPodStatus: 0,
      stopForwarding: 0,
    };
    this.pollControllers = {
      status: null,
    };
    this.createInFlight = false;
    this.stopInFlight = false;
  }

  componentDidMount() {
    this.isMountedFlag = true;
    this.startPolling();
    void this.initializeComponent();
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    Object.keys(this.requestIds).forEach((key) => {
      this.requestIds[key] += 1;
    });
    this.stopPolling();
  }

  startPolling = () => {
    if (!this.pollControllers.status) {
      this.pollControllers.status = createPollingController(
        this.fetchForwardingStatus,
        5_000,
        { immediate: false },
      );
    }
  };

  stopPolling = () => {
    this.pollControllers.status?.stop();
    this.pollControllers.status = null;
  };

  initializeComponent = async () => {
    if (!this.isMountedFlag) return;
    this.setState({ error: null, loading: true });

    try {
      const podsList = await this.fetchPods();
      if (!this.isMountedFlag) return;
      await Promise.all([
        this.fetchAvailablePorts(),
        this.fetchForwardingStatus(),
        this.fetchVpnPodStatus(podsList),
      ]);
    } catch (error) {
      console.error('Failed to initialize port forwarding:', error);
      if (this.isMountedFlag) {
        this.setState({ error: toDisplayError(error, 'Failed to initialize port forwarding') });
      }
    } finally {
      if (this.isMountedFlag) this.setState({ loading: false });
    }
  };

  fetchPods = async () => {
    const requestId = ++this.requestIds.pods;
    try {
      const podsList = await pods.list();
      const normalizedPods = Array.isArray(podsList) ? podsList : [];
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.pods
      ) {
        this.setState({ pods: normalizedPods });
      }
      return normalizedPods;
    } catch (error) {
      console.error('Failed to fetch pods:', error);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.pods
      ) {
        this.setState({ pods: [] });
      }
      return [];
    }
  };

  fetchAvailablePorts = async () => {
    const requestId = ++this.requestIds.availablePorts;
    try {
      const result = await portForwarding.getAvailablePorts();
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.availablePorts
      ) {
        this.setState({
          availablePorts: Array.isArray(result?.availablePorts)
            ? result.availablePorts
            : [],
        });
      }
    } catch (error) {
      console.error('Failed to fetch available ports:', error);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.availablePorts
      ) {
        this.setState({ availablePorts: [] });
      }
    }
  };

  fetchForwardingStatus = async () => {
    const requestId = ++this.requestIds.forwardingStatus;
    try {
      const status = await portForwarding.getForwardingStatus();
      const normalizedStatus = Array.isArray(status) ? status : [];
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.forwardingStatus
      ) {
        this.setState({ forwardingStatus: normalizedStatus });
      }
      return normalizedStatus;
    } catch (error) {
      console.error('Failed to fetch forwarding status:', error);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.forwardingStatus
      ) {
        this.setState({ forwardingStatus: [] });
      }
      return [];
    }
  };

  fetchVpnPodStatus = async (podsList = this.state.pods) => {
    const requestId = ++this.requestIds.vpnPodStatus;
    const vpnCapablePods = podsList.filter(
      (pod) =>
        pod.capabilities?.includes('PrivateServiceGateway') ||
        pod.privateServicePolicy?.enabled === true,
    );

    const statusPromises = vpnCapablePods.map(async (pod) => {
      try {
        const detail = await pods.get(pod.podId);
        const memberCount = detail.members?.length || 0;
        const activeTunnels = nonNegativeNumber(detail.activeTunnels);
        const totalBandwidth = nonNegativeNumber(detail.totalBandwidth);

        return {
          activeTunnels,
          lastActivity: nonNegativeNumber(detail.lastActivity),
          members: memberCount,
          name: pod.name || pod.podId,
          podId: pod.podId,
          status: detail.privateServicePolicy?.enabled ? 'Active' : 'Inactive',
          totalBandwidth,
        };
      } catch (error) {
        console.error('Failed to fetch pod status', {
          error,
          podId: pod.podId,
        });
        return {
          activeTunnels: null,
          lastActivity: null,
          members: 0,
          name: pod.name || pod.podId,
          podId: pod.podId,
          status: 'Error',
          totalBandwidth: null,
        };
      }
    });

    try {
      const statusResults = await Promise.all(statusPromises);
      const status = statusResults.reduce((accumulator, stat) => {
        accumulator[stat.podId] = stat;
        return accumulator;
      }, {});

      if (
        this.isMountedFlag &&
        requestId === this.requestIds.vpnPodStatus
      ) {
        this.setState({ vpnPodStatus: status });
      }
    } catch (error) {
      console.error('Failed to fetch VPN pod status:', error);
    }
  };

  handlePodSelection = async (podId) => {
    if (!this.isMountedFlag) return;
    const requestId = ++this.requestIds.podDetails;
    this.setState({
      loading: true,
      selectedPodDetail: null,
      selectedPodId: podId,
    });

    try {
      const podDetail = await pods.get(podId);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.podDetails
      ) {
        this.setState({ selectedPodDetail: podDetail });
      }
    } catch (error) {
      console.error('Failed to fetch pod detail:', error);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.podDetails
      ) {
        this.setState({
          error: `Failed to load pod details: ${toDisplayError(error)}`,
        });
      }
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.podDetails
      ) {
        this.setState({ loading: false });
      }
    }
  };

  handleCreateForwarding = async () => {
    const { createForm, selectedPodId } = this.state;

    if (!selectedPodId) {
      this.setState({ error: 'Please select a pod first' });
      return;
    }

    // Validate form
    if (
      !createForm.localPort ||
      !createForm.destinationHost ||
      !createForm.destinationPort
    ) {
      this.setState({ error: 'Please fill in all required fields' });
      return;
    }

    if (this.state.creatingForwarding || this.createInFlight) return;

    const localPort = Number.parseInt(createForm.localPort, 10);
    const destinationPort = Number.parseInt(createForm.destinationPort, 10);

    if (isNaN(localPort) || localPort < 1_024 || localPort > 65_535) {
      this.setState({ error: 'Local port must be between 1024 and 65535' });
      return;
    }

    if (
      isNaN(destinationPort) ||
      destinationPort < 1 ||
      destinationPort > 65_535
    ) {
      this.setState({ error: 'Destination port must be between 1 and 65535' });
      return;
    }

    const requestId = ++this.requestIds.createForwarding;
    this.createInFlight = true;
    this.setState({ creatingForwarding: true, error: null });

    try {
      await portForwarding.startForwarding({
        destinationHost: createForm.destinationHost,
        destinationPort,
        localPort,
        podId: selectedPodId,
        serviceName: createForm.serviceName || undefined,
      });

      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.createForwarding
      ) {
        return;
      }
      // Reset form and refresh status
      this.setState({
        createForm: initialState.createForm,
        showCreateModal: false,
      });

      await Promise.all([
        this.fetchAvailablePorts(),
        this.fetchForwardingStatus(),
      ]);
    } catch (error) {
      console.error('Failed to create port forwarding:', error);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.createForwarding
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to create port forwarding'),
        });
      }
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.createForwarding
      ) {
        this.setState({ creatingForwarding: false });
      }
      this.createInFlight = false;
    }
  };

  handleStopForwarding = async (localPort) => {
    if (
      !this.isMountedFlag ||
      this.state.stoppingForwarding ||
      this.stopInFlight
    ) return;
    const requestId = ++this.requestIds.stopForwarding;
    this.stopInFlight = true;
    this.setState({ error: null, stoppingForwarding: true, success: null });

    try {
      await portForwarding.stopForwarding(localPort);
      await Promise.all([
        this.fetchAvailablePorts(),
        this.fetchForwardingStatus(),
      ]);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.stopForwarding
      ) {
        this.setState({
          success: `Successfully stopped forwarding on port ${localPort}`,
        });
      }
    } catch (error) {
      console.error('Failed to stop port forwarding:', error);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.stopForwarding
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to stop port forwarding'),
        });
      }
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.stopForwarding
      ) {
        this.setState({ stoppingForwarding: false });
      }
      this.stopInFlight = false;
    }
  };

  handleFormChange = (field, value) => {
    this.setState((previousState) => ({
      createForm: {
        ...previousState.createForm,
        [field]: value,
      },
    }));
  };

  render() {
    const {
      availablePorts,
      createForm,
      creatingForwarding,
      error,
      forwardingStatus,
      loading,
      pods,
      selectedPodDetail,
      selectedPodId,
      showCreateModal,
      stoppingForwarding,
      success,
      vpnPodStatus,
    } = this.state;

    // Filter pods that have VPN gateway capability
    const vpnCapablePods = pods.filter(
      (pod) =>
        pod.capabilities?.includes('PrivateServiceGateway') ||
        pod.privateServicePolicy?.enabled === true,
    );

    const tunnelStats = buildTunnelStats(forwardingStatus);
    const tunnelStatValues = Object.values(tunnelStats);
    const activeForwardingCount = forwardingStatus.filter(
      (forwarding) => forwarding.isActive,
    ).length;
    const totalConnections = tunnelStatValues.reduce(
      (sum, stats) => sum + stats.connections,
      0,
    );
    const byteStats = tunnelStatValues.filter(
      (stats) => stats.bytesIn !== null || stats.bytesOut !== null,
    );
    const totalBytes = byteStats.reduce(
      (sum, stats) => sum + (stats.bytesIn || 0) + (stats.bytesOut || 0),
      0,
    );
    const uptimeStats = tunnelStatValues.filter(
      (stats) => stats.uptime !== null,
    );
    const knownActiveTunnelCount = Object.values(vpnPodStatus).filter(
      (pod) => pod.activeTunnels !== null,
    ).length;
    const totalActiveTunnels = Object.values(vpnPodStatus).reduce(
      (sum, pod) => sum + (pod.activeTunnels || 0),
      0,
    );

    const panes = [
      {
        menuItem: 'Active Forwarding',
        render: () => (
          <Tab.Pane>
            {forwardingStatus.length === 0 ? (
              <Segment placeholder>
                <Icon name="exchange" />
                <h3>No active port forwarding</h3>
                <p>
                  Start forwarding local ports to remote services through VPN
                  tunnels.
                </p>
                <Button
                  disabled={vpnCapablePods.length === 0}
                  onClick={() => this.setState({ showCreateModal: true })}
                  primary
                >
                  Start Forwarding
                </Button>
              </Segment>
            ) : (
              <div>
                <div style={{ marginBottom: '20px', textAlign: 'right' }}>
                  <Button
                    disabled={vpnCapablePods.length === 0}
                    onClick={() => this.setState({ showCreateModal: true })}
                    primary
                  >
                    <Icon name="plus" />
                    Add Forwarding
                  </Button>
                </div>

                <Table celled>
                  <Table.Header>
                    <Table.Row>
                      <Table.HeaderCell>Local Port</Table.HeaderCell>
                      <Table.HeaderCell>Pod</Table.HeaderCell>
                      <Table.HeaderCell>Remote Service</Table.HeaderCell>
                      <Table.HeaderCell>Status</Table.HeaderCell>
                      <Table.HeaderCell>Connections</Table.HeaderCell>
                      <Table.HeaderCell>Data Transferred</Table.HeaderCell>
                      <Table.HeaderCell>Actions</Table.HeaderCell>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {forwardingStatus.map((forwarding) => (
                      <Table.Row key={forwarding.localPort}>
                        <Table.Cell>
                          <code>localhost:{forwarding.localPort}</code>
                        </Table.Cell>
                        <Table.Cell>
                          {forwarding.podId}
                          {forwarding.serviceName && (
                            <div style={{ color: 'var(--slskr-color-subtle, #666)', fontSize: '0.8em' }}>
                              Service: {forwarding.serviceName}
                            </div>
                          )}
                        </Table.Cell>
                        <Table.Cell>
                          <code>
                            {forwarding.destinationHost}:
                            {forwarding.destinationPort}
                          </code>
                        </Table.Cell>
                        <Table.Cell>
                          <Label color={forwarding.isActive ? 'green' : 'red'}>
                            {forwarding.isActive ? 'Active' : 'Inactive'}
                          </Label>
                        </Table.Cell>
                        <Table.Cell>{forwarding.activeConnections}</Table.Cell>
                        <Table.Cell>
                          {forwarding.bytesForwarded > 0
                            ? `${(forwarding.bytesForwarded / 1_024).toFixed(1)} KB`
                            : '0 KB'}
                        </Table.Cell>
                        <Table.Cell>
                          <Popup
                            content="Stop port forwarding"
                            trigger={
                              <Button
                                color="red"
                                icon="stop"
                                loading={stoppingForwarding}
                                onClick={() =>
                                  this.handleStopForwarding(
                                    forwarding.localPort,
                                  )
                                }
                                size="small"
                              />
                            }
                          />
                        </Table.Cell>
                      </Table.Row>
                    ))}
                  </Table.Body>
                </Table>
              </div>
            )}
          </Tab.Pane>
        ),
      },
      {
        menuItem: 'Available Ports',
        render: () => (
          <Tab.Pane>
            <div style={{ marginBottom: '20px' }}>
              <Statistic.Group size="small">
                <Statistic>
                  <Statistic.Value>{availablePorts.length}</Statistic.Value>
                  <Statistic.Label>Available Ports</Statistic.Label>
                </Statistic>
                <Statistic>
                  <Statistic.Value>{forwardingStatus.length}</Statistic.Value>
                  <Statistic.Label>In Use</Statistic.Label>
                </Statistic>
              </Statistic.Group>
            </div>

            <Segment>
              <p>Available ports for forwarding (1024-65535):</p>
              <div
                style={{
                  backgroundColor: 'var(--slskr-color-inset, #f8f9fa)',
                  borderRadius: '4px',
                  fontFamily: 'monospace',
                  fontSize: '12px',
                  maxHeight: '400px',
                  overflowY: 'auto',
                  padding: '10px',
                }}
              >
                {availablePorts.length > 0 ? (
                  availablePorts.slice(0, 100).join(', ') +
                  (availablePorts.length > 100
                    ? ` ... (+${availablePorts.length - 100} more)`
                    : '')
                ) : (
                  <em>No ports available or still loading...</em>
                )}
              </div>
            </Segment>
          </Tab.Pane>
        ),
      },
      {
        menuItem: 'Tunnel Statistics',
        render: () => (
          <Tab.Pane>
            <div style={{ marginBottom: '20px' }}>
              <Statistic.Group widths="four">
                <Statistic>
                  <Statistic.Value>{activeForwardingCount}</Statistic.Value>
                  <Statistic.Label>Active Tunnels</Statistic.Label>
                </Statistic>
                <Statistic>
                  <Statistic.Value>
                    {totalConnections}
                  </Statistic.Value>
                  <Statistic.Label>Total Connections</Statistic.Label>
                </Statistic>
                <Statistic>
                  <Statistic.Value>
                    {byteStats.length > 0
                      ? `${(totalBytes / 1_024 / 1_024).toFixed(2)} MB`
                      : 'N/A'}
                  </Statistic.Value>
                  <Statistic.Label>Data Transferred</Statistic.Label>
                </Statistic>
                <Statistic>
                  <Statistic.Value>
                    {uptimeStats.length > 0
                      ? (
                          uptimeStats.reduce(
                            (sum, stats) => sum + stats.uptime,
                            0,
                          ) /
                          uptimeStats.length /
                          1_000 /
                          60
                        ).toFixed(1)
                      : 'N/A'}{' '}
                    {uptimeStats.length > 0 && 'min'}
                  </Statistic.Value>
                  <Statistic.Label>Avg Uptime</Statistic.Label>
                </Statistic>
              </Statistic.Group>
            </div>

            <Table celled>
              <Table.Header>
                <Table.Row>
                  <Table.HeaderCell>Local Port</Table.HeaderCell>
                  <Table.HeaderCell>Data In</Table.HeaderCell>
                  <Table.HeaderCell>Data Out</Table.HeaderCell>
                  <Table.HeaderCell>Connections</Table.HeaderCell>
                  <Table.HeaderCell>Uptime</Table.HeaderCell>
                  <Table.HeaderCell>Last Activity</Table.HeaderCell>
                </Table.Row>
              </Table.Header>
              <Table.Body>
                {forwardingStatus.map((forwarding) => {
                  const stats = tunnelStats[forwarding.localPort];
                  return (
                    <Table.Row key={forwarding.localPort}>
                      <Table.Cell>
                        <code>localhost:{forwarding.localPort}</code>
                      </Table.Cell>
                      <Table.Cell>
                        {stats?.bytesIn !== null &&
                        stats?.bytesIn !== undefined
                          ? `${(stats.bytesIn / 1_024).toFixed(1)} KB`
                          : 'N/A'}
                      </Table.Cell>
                      <Table.Cell>
                        {stats?.bytesOut !== null &&
                        stats?.bytesOut !== undefined
                          ? `${(stats.bytesOut / 1_024).toFixed(1)} KB`
                          : 'N/A'}
                      </Table.Cell>
                      <Table.Cell>{stats?.connections ?? 'N/A'}</Table.Cell>
                      <Table.Cell>
                        {stats?.uptime !== null && stats?.uptime !== undefined
                          ? `${Math.floor(stats.uptime / 1_000 / 60)}m ${Math.floor((stats.uptime / 1_000) % 60)}s`
                          : 'N/A'}
                      </Table.Cell>
                      <Table.Cell>
                        {stats?.lastActivity !== null &&
                        stats?.lastActivity !== undefined
                          ? `${Math.max(0, Math.floor((Date.now() - stats.lastActivity) / 1_000))}s ago`
                          : 'N/A'}
                      </Table.Cell>
                    </Table.Row>
                  );
                })}
                {forwardingStatus.length === 0 && (
                  <Table.Row>
                    <Table.Cell
                      colSpan={6}
                      textAlign="center"
                    >
                      No active tunnels to display statistics for
                    </Table.Cell>
                  </Table.Row>
                )}
              </Table.Body>
            </Table>
          </Tab.Pane>
        ),
      },
      {
        menuItem: 'VPN Pods',
        render: () => (
          <Tab.Pane>
            <div style={{ marginBottom: '20px' }}>
              <Statistic.Group widths="three">
                <Statistic>
                  <Statistic.Value>
                    {Object.keys(vpnPodStatus).length}
                  </Statistic.Value>
                  <Statistic.Label>VPN-Capable Pods</Statistic.Label>
                </Statistic>
                <Statistic>
                  <Statistic.Value>
                    {Object.values(vpnPodStatus).reduce(
                      (sum, pod) => sum + pod.members,
                      0,
                    )}
                  </Statistic.Value>
                  <Statistic.Label>Total Members</Statistic.Label>
                </Statistic>
                <Statistic>
                  <Statistic.Value>
                    {knownActiveTunnelCount > 0
                      ? totalActiveTunnels
                      : 'N/A'}
                  </Statistic.Value>
                  <Statistic.Label>Active Tunnels</Statistic.Label>
                </Statistic>
              </Statistic.Group>
            </div>

            <Table celled>
              <Table.Header>
                <Table.Row>
                  <Table.HeaderCell>Pod Name</Table.HeaderCell>
                  <Table.HeaderCell>Members</Table.HeaderCell>
                  <Table.HeaderCell>Active Tunnels</Table.HeaderCell>
                  <Table.HeaderCell>Data Transferred</Table.HeaderCell>
                  <Table.HeaderCell>Status</Table.HeaderCell>
                  <Table.HeaderCell>Last Activity</Table.HeaderCell>
                </Table.Row>
              </Table.Header>
              <Table.Body>
                {Object.values(vpnPodStatus).map((pod) => (
                  <Table.Row key={pod.podId}>
                    <Table.Cell>
                      <strong>{pod.name}</strong>
                      <div style={{ color: 'var(--slskr-color-subtle, #666)', fontSize: '0.8em' }}>
                        ID: {pod.podId}
                      </div>
                    </Table.Cell>
                    <Table.Cell>{pod.members}</Table.Cell>
                    <Table.Cell>{pod.activeTunnels ?? 'N/A'}</Table.Cell>
                    <Table.Cell>
                      {pod.totalBandwidth !== null && pod.totalBandwidth > 0
                        ? `${(pod.totalBandwidth / 1_024 / 1_024).toFixed(2)} MB`
                        : pod.totalBandwidth === null
                          ? 'N/A'
                          : '0 MB'}
                    </Table.Cell>
                    <Table.Cell>
                      <Label color={pod.status === 'Active' ? 'green' : 'grey'}>
                        {pod.status}
                      </Label>
                    </Table.Cell>
                    <Table.Cell>
                      {pod.lastActivity === null
                        ? 'N/A'
                        : `${Math.max(0, Math.floor((Date.now() - pod.lastActivity) / 1_000))}s ago`}
                    </Table.Cell>
                  </Table.Row>
                ))}
                {Object.keys(vpnPodStatus).length === 0 && (
                  <Table.Row>
                    <Table.Cell
                      colSpan={6}
                      textAlign="center"
                    >
                      No VPN-capable pods found
                    </Table.Cell>
                  </Table.Row>
                )}
              </Table.Body>
            </Table>
          </Tab.Pane>
        ),
      },
    ];

    return (
      <div style={{ padding: '20px' }}>
        <Dimmer active={loading}>
          <Loader />
        </Dimmer>

        <div style={{ marginBottom: '30px' }}>
          <h2>Port Forwarding</h2>
          <p>
            Forward local ports to remote services through secure VPN tunnels.
          </p>
        </div>

        {error && (
          <Message error>
            <Message.Header>Error</Message.Header>
            <p>{error}</p>
            <Button
              onClick={() => this.setState({ error: null })}
              size="small"
            >
              Dismiss
            </Button>
          </Message>
        )}

        {success && (
          <Message success>
            <Message.Header>Success</Message.Header>
            <p>{success}</p>
            <Button
              onClick={() => this.setState({ success: null })}
              size="small"
            >
              Dismiss
            </Button>
          </Message>
        )}

        {vpnCapablePods.length === 0 && (
          <Message warning>
            <Message.Header>No VPN-Capable Pods</Message.Header>
            <p>
              You need at least one pod with VPN gateway capability to use port
              forwarding.
            </p>
            <p>
              Create or join a pod that has the{' '}
              <code>PrivateServiceGateway</code> capability enabled.
            </p>
          </Message>
        )}

        <Tab
          activeIndex={this.state.activeTab}
          menu={{ pointing: true }}
          onTabChange={(_event, { activeIndex }) =>
            this.setState({ activeTab: activeIndex })
          }
          panes={panes}
          renderActiveOnly={false}
        />

        {/* Create Forwarding Modal */}
        <Modal
          onClose={() => this.setState({ showCreateModal: false })}
          open={showCreateModal}
          size="small"
        >
          <Modal.Header>Start Port Forwarding</Modal.Header>
          <Modal.Content>
            <Form>
              <Form.Field>
                <label>VPN Pod</label>
                <Dropdown
                  fluid
                  onChange={(e, { value }) => this.handlePodSelection(value)}
                  options={vpnCapablePods.map((pod) => ({
                    key: pod.podId,
                    text: pod.name || pod.podId,
                    value: pod.podId,
                  }))}
                  placeholder="Select a VPN-capable pod"
                  selection
                  value={selectedPodId || ''}
                />
                {selectedPodDetail && (
                  <div
                    style={{
                      color: 'var(--slskr-color-subtle, #666)',
                      fontSize: '0.9em',
                      marginTop: '10px',
                    }}
                  >
                    <p>
                      <strong>Members:</strong>{' '}
                      {selectedPodDetail.members?.length || 0}
                    </p>
                    {selectedPodDetail.privateServicePolicy?.enabled && (
                      <p>
                        <strong>VPN Gateway:</strong> Enabled
                      </p>
                    )}
                  </div>
                )}
              </Form.Field>

              <Form.Field required>
                <label>Local Port</label>
                <Input
                  max="65535"
                  min="1024"
                  onChange={(e) =>
                    this.handleFormChange('localPort', e.target.value)
                  }
                  placeholder="e.g., 8080"
                  type="number"
                  value={createForm.localPort}
                />
                <small style={{ color: 'var(--slskr-color-subtle, #666)' }}>
                  Port on your local machine (1024-65535)
                </small>
              </Form.Field>

              <Form.Field required>
                <label>Remote Host</label>
                <Input
                  onChange={(e) =>
                    this.handleFormChange('destinationHost', e.target.value)
                  }
                  placeholder="e.g., database.internal.company.com"
                  value={createForm.destinationHost}
                />
                <small style={{ color: 'var(--slskr-color-subtle, #666)' }}>
                  Hostname or IP address of the remote service
                </small>
              </Form.Field>

              <Form.Field required>
                <label>Remote Port</label>
                <Input
                  max="65535"
                  min="1"
                  onChange={(e) =>
                    this.handleFormChange('destinationPort', e.target.value)
                  }
                  placeholder="e.g., 5432"
                  type="number"
                  value={createForm.destinationPort}
                />
                <small style={{ color: 'var(--slskr-color-subtle, #666)' }}>
                  Port number of the remote service
                </small>
              </Form.Field>

              <Form.Field>
                <label>Service Name (Optional)</label>
                <Input
                  onChange={(e) =>
                    this.handleFormChange('serviceName', e.target.value)
                  }
                  placeholder="e.g., postgres-db"
                  value={createForm.serviceName}
                />
                <small style={{ color: 'var(--slskr-color-subtle, #666)' }}>
                  Named service registered in the pod (for better organization)
                </small>
              </Form.Field>
            </Form>
          </Modal.Content>
          <Modal.Actions>
            <Button onClick={() => this.setState({ showCreateModal: false })}>
              Cancel
            </Button>
            <Button
              disabled={
                !selectedPodId ||
                !createForm.localPort ||
                !createForm.destinationHost ||
                !createForm.destinationPort
              }
              loading={creatingForwarding}
              onClick={this.handleCreateForwarding}
              primary
            >
              Start Forwarding
            </Button>
          </Modal.Actions>
        </Modal>
      </div>
    );
  }
}

export default PortForwarding;
