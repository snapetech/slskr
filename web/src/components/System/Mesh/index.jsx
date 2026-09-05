import * as mesh from '../../../lib/mesh';
import * as soulseekDiscovery from '../../../lib/soulseekDiscovery';
import { toDisplayError } from '../../../lib/errors';
import MeshEvidencePolicy from './MeshEvidencePolicy';
import RealmSubjectIndexConflicts from './RealmSubjectIndexConflicts';
import { useMountedRef } from '../../../lib/useMountedRef';
import { usePolling } from '../../../lib/usePolling';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Button,
  Card,
  Grid,
  Header,
  Icon,
  Label,
  List,
  Loader,
  Message,
  Segment,
  Statistic,
} from 'semantic-ui-react';

const isRecord = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

const toCount = (value) => {
  if (Array.isArray(value)) return value.length;
  if (isRecord(value)) {
    for (const key of ['count', 'activeConnections', 'sessions', 'activeSessions']) {
      if (value[key] !== undefined) return toCount(value[key]);
    }
    return null;
  }
  if (value === null || value === undefined || value === '') return null;
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 ? number : null;
};

const formatCount = (value) => {
  const count = toCount(value);
  return count === null ? '—' : count.toLocaleString();
};

const normalizeMeshStats = (data) => {
  if (!isRecord(data)) return null;

  const dhtSessions = toCount(
    data.activeDhtSessions ?? data.dhtSessions ?? data.dht,
  );
  const overlaySessions = toCount(
    data.activeOverlaySessions ?? data.overlaySessions ?? data.overlay,
  );

  return {
    ...data,
    activeDhtSessions: dhtSessions,
    activeOverlaySessions: overlaySessions,
    description:
      data.description ??
      (dhtSessions !== null || overlaySessions !== null
        ? 'Mesh transport session counts are available.'
        : undefined),
    status:
      data.status ??
      (dhtSessions !== null || overlaySessions !== null ? 'Healthy' : 'Unknown'),
  };
};

const Mesh = ({ runtimeProfile } = {}) => {
  const [stats, setStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [rendezvousStatus, setRendezvousStatus] = useState(null);
  const [rendezvousStatusError, setRendezvousStatusError] = useState(null);
  const [rendezvousUsers, setRendezvousUsers] = useState([]);
  const [capabilityRecords, setCapabilityRecords] = useState([]);
  const [rendezvousLoading, setRendezvousLoading] = useState(false);
  const [rendezvousMessage, setRendezvousMessage] = useState(null);
  const mountedRef = useMountedRef();
  const statsRequestIdRef = useRef(0);
  const rendezvousStatusRequestIdRef = useRef(0);
  const rendezvousRequestIdRef = useRef(0);
  const rendezvousInFlightRef = useRef(false);

  const fetchStats = useCallback(async () => {
    if (!mountedRef.current) return;
    const requestId = ++statsRequestIdRef.current;
    try {
      setLoading(true);
      setError(null);
      const data = await mesh.getStats();
      if (
        mountedRef.current &&
        statsRequestIdRef.current === requestId
      ) {
        setStats(normalizeMeshStats(data));
      }
    } catch (error_) {
      if (
        mountedRef.current &&
        statsRequestIdRef.current === requestId
      ) {
        setError(toDisplayError(error_, 'Failed to load mesh statistics'));
      }
    } finally {
      if (
        mountedRef.current &&
        statsRequestIdRef.current === requestId
      ) {
        setLoading(false);
      }
    }
  }, [mountedRef]);

  const getRendezvousErrorText = (error_, fallback) => {
    return toDisplayError(error_, fallback);
  };

  usePolling(fetchStats, 30_000);

  useEffect(() => {
    const requestId = ++rendezvousStatusRequestIdRef.current;
    const fetchRendezvousStatus = async () => {
      if (!mountedRef.current) return;
      setRendezvousStatusError(null);
      try {
        const response = await soulseekDiscovery.getMeshRendezvousStatus();
        if (
          mountedRef.current &&
          rendezvousStatusRequestIdRef.current === requestId
        ) {
          setRendezvousStatus(
            response.data && typeof response.data === 'object'
              ? response.data
              : {},
          );
          setRendezvousStatusError(null);
        }
      } catch (error_) {
        if (
          mountedRef.current &&
          rendezvousStatusRequestIdRef.current === requestId
        ) {
          setRendezvousStatusError(
            toDisplayError(error_, 'Unable to load rendezvous status'),
          );
        }
      }
    };

    void fetchRendezvousStatus();

    return () => {
      rendezvousStatusRequestIdRef.current += 1;
    };
  }, [mountedRef]);

  const beginRendezvousRequest = () => {
    if (
      !mountedRef.current ||
      rendezvousLoading ||
      rendezvousInFlightRef.current
    ) return null;
    const requestId = ++rendezvousRequestIdRef.current;
    rendezvousInFlightRef.current = true;
    return {
      isCurrentRequest: () =>
        mountedRef.current && rendezvousRequestIdRef.current === requestId,
      requestId,
    };
  };

  const handleAddRendezvousInterest = async () => {
    const request = beginRendezvousRequest();
    if (!request) return;
    setRendezvousLoading(true);
    setRendezvousMessage(null);
    try {
      await soulseekDiscovery.addMeshRendezvousInterest();
      if (!request.isCurrentRequest()) return;
      setRendezvousMessage({
        positive: true,
        text: 'Published the slskr mesh rendezvous interest on this Soulseek account.',
      });
    } catch (error_) {
      if (!request.isCurrentRequest()) return;
      setRendezvousMessage({
        negative: true,
        text:
          error_?.response?.status === 403
            ? 'Soulseek rendezvous is disabled in configuration. Enable mesh.enableSoulseekRendezvous before publishing this public interest.'
            : getRendezvousErrorText(error_, 'Unable to publish rendezvous interest.'),
      });
    } finally {
      if (request.isCurrentRequest()) setRendezvousLoading(false);
      rendezvousInFlightRef.current = false;
    }
  };

  const handleRemoveRendezvousInterest = async () => {
    const request = beginRendezvousRequest();
    if (!request) return;
    setRendezvousLoading(true);
    setRendezvousMessage(null);
    try {
      await soulseekDiscovery.removeMeshRendezvousInterest();
      if (!request.isCurrentRequest()) return;
      setRendezvousMessage({
        positive: true,
        text: 'Removed the slskr mesh rendezvous interest from this Soulseek account.',
      });
    } catch (error_) {
      if (!request.isCurrentRequest()) return;
      setRendezvousMessage({
        negative: true,
        text:
          error_?.response?.status === 403
            ? 'Soulseek rendezvous is disabled in configuration. Enable mesh.enableSoulseekRendezvous to manage this public interest from the UI.'
            : getRendezvousErrorText(error_, 'Unable to remove rendezvous interest.'),
      });
    } finally {
      if (request.isCurrentRequest()) setRendezvousLoading(false);
      rendezvousInFlightRef.current = false;
    }
  };

  const handleLoadRendezvousUsers = async () => {
    const request = beginRendezvousRequest();
    if (!request) return;
    setRendezvousLoading(true);
    setRendezvousMessage(null);
    try {
      const response = await soulseekDiscovery.discoverMeshRendezvous();
      if (!request.isCurrentRequest()) return;
      const data = response.data && typeof response.data === 'object'
        ? response.data
        : {};
      const users = Array.isArray(data.users)
        ? data.users
          .filter((user) => user && typeof user === 'object' && !Array.isArray(user))
          .map((user) => ({
            ...user,
            username: user.username || user.Username || 'unknown user',
          }))
        : [];
      const records = Array.isArray(data.capabilityRecords)
        ? data.capabilityRecords
          .filter((record) => record && typeof record === 'object' && !Array.isArray(record))
          .map((record) => ({
            ...record,
            features: Array.isArray(record.features)
              ? record.features.filter((feature) => typeof feature === 'string')
              : [],
            username: record.username || 'unknown user',
          }))
        : [];
      setRendezvousUsers(users);
      setCapabilityRecords(records);
      setRendezvousMessage({
        positive: true,
        text: `Discovered ${users.length} Soulseek rendezvous candidate(s) and ${records.length} runtime capability record(s).`,
      });
    } catch (error_) {
      if (!request.isCurrentRequest()) return;
      setRendezvousMessage({
        negative: true,
        text:
          error_?.response?.status === 403
            ? 'Soulseek rendezvous is disabled in configuration. Enable mesh.enableSoulseekRendezvous before querying candidates.'
            : getRendezvousErrorText(error_, 'Unable to load rendezvous users.'),
      });
    } finally {
      if (request.isCurrentRequest()) setRendezvousLoading(false);
      rendezvousInFlightRef.current = false;
    }
  };

  const getHealthColor = (status) => {
    switch (status) {
      case 'Healthy':
        return 'green';
      case 'Degraded':
        return 'yellow';
      case 'Unhealthy':
        return 'red';
      default:
        return 'grey';
    }
  };

  const getHealthIcon = (status) => {
    switch (status) {
      case 'Healthy':
        return 'checkmark';
      case 'Degraded':
        return 'warning sign';
      case 'Unhealthy':
        return 'remove';
      default:
        return 'question';
    }
  };

  if (loading && !stats) {
    return (
      <Segment>
        <Loader
          active
          inline="centered"
        >
          Loading mesh statistics...
        </Loader>
      </Segment>
    );
  }

  if (error && !stats) {
    return (
      <Message error>
        <Message.Header>Failed to load mesh statistics</Message.Header>
        <p>{error}</p>
      </Message>
    );
  }

  return (
    <div>
      <Header as="h2">
        <Icon name="sitemap" />
        Mesh Network Status
      </Header>
      {error && (
        <Message
          data-testid="mesh-stats-load-error"
          error
        >
          <Message.Header>Mesh statistics refresh failed</Message.Header>
          <p>{error}</p>
          <p>Showing the last successfully loaded mesh snapshot.</p>
        </Message>
      )}
      <Button
        aria-label="Refresh mesh statistics"
        disabled={loading}
        icon="refresh"
        loading={loading}
        onClick={fetchStats}
      />

      <Grid stackable>
        {/* Overall Health Status */}
        <Grid.Column width={16}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon
                  color={getHealthColor(stats?.status)}
                  name={getHealthIcon(stats?.status)}
                />
                Network Health: {stats?.status || 'Unknown'}
              </Card.Header>
              <Card.Description>
                {stats?.description || 'No health information available'}
              </Card.Description>
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Key Statistics */}
        <Grid.Column width={16}>
          <Segment>
            <Header as="h3">Network Statistics</Header>
            <Statistic.Group size="small">
              <Statistic>
                <Statistic.Value>{formatCount(stats?.totalPeers)}</Statistic.Value>
                <Statistic.Label>Total Peers</Statistic.Label>
              </Statistic>
              <Statistic>
                <Statistic.Value>
                  {formatCount(stats?.activeDhtSessions)}
                </Statistic.Value>
                <Statistic.Label>DHT Sessions</Statistic.Label>
              </Statistic>
              <Statistic>
                <Statistic.Value>
                  {formatCount(stats?.activeOverlaySessions)}
                </Statistic.Value>
                <Statistic.Label>Overlay Sessions</Statistic.Label>
              </Statistic>
              <Statistic>
                <Statistic.Value>
                  {formatCount(stats?.routingTableSize)}
                </Statistic.Value>
                <Statistic.Label>Routing Table Size</Statistic.Label>
              </Statistic>
            </Statistic.Group>
          </Segment>
        </Grid.Column>

        {/* Connection Details */}
        <Grid.Column width={8}>
          <Segment>
            <Header as="h3">
              <Icon name="plug" />
              Connections
            </Header>
            <List
              divided
              relaxed
            >
              <List.Item>
                <List.Content>
                  <List.Header>DHT Nodes</List.Header>
                  <List.Description>
                    {formatCount(stats?.activeDhtSessions)} active connections
                  </List.Description>
                </List.Content>
              </List.Item>
              <List.Item>
                <List.Content>
                  <List.Header>Overlay Sessions</List.Header>
                  <List.Description>
                    {formatCount(stats?.activeOverlaySessions)} active sessions
                  </List.Description>
                </List.Content>
              </List.Item>
              <List.Item>
                <List.Content>
                  <List.Header>Mirrored Sessions</List.Header>
                  <List.Description>
                    {formatCount(stats?.activeMirroredSessions)} relay connections
                  </List.Description>
                </List.Content>
              </List.Item>
              <List.Item>
                <List.Content>
                  <List.Header>Bootstrap Peers</List.Header>
                  <List.Description>
                    {formatCount(stats?.bootstrapPeers)} bootstrap nodes
                  </List.Description>
                </List.Content>
              </List.Item>
            </List>
          </Segment>
        </Grid.Column>

        {/* Performance Metrics */}
        <Grid.Column width={8}>
          <Segment>
            <Header as="h3">
              <Icon name="chart line" />
              Performance
            </Header>
            <List
              divided
              relaxed
            >
              <List.Item>
                <List.Content>
                  <List.Header>DHT Operations/sec</List.Header>
                  <List.Description>
                    {Number.isFinite(Number(stats?.dhtOperationsPerSecond))
                      ? Number(stats.dhtOperationsPerSecond).toFixed(1)
                      : '—'} ops/sec
                  </List.Description>
                </List.Content>
              </List.Item>
              <List.Item>
                <List.Content>
                  <List.Header>Messages Sent</List.Header>
                  <List.Description>
                    {formatCount(stats?.messagesSent)} total messages
                  </List.Description>
                </List.Content>
              </List.Item>
              <List.Item>
                <List.Content>
                  <List.Header>Messages Received</List.Header>
                  <List.Description>
                    {formatCount(stats?.messagesReceived)} total messages
                  </List.Description>
                </List.Content>
              </List.Item>
              <List.Item>
                <List.Content>
                  <List.Header>Peer Churn Events</List.Header>
                  <List.Description>
                    {formatCount(stats?.peerChurnEvents)} churn events
                  </List.Description>
                </List.Content>
              </List.Item>
            </List>
          </Segment>
        </Grid.Column>

        {/* NAT and Network Info */}
        <Grid.Column width={16}>
          <Segment>
            <Header as="h3">
              <Icon name="shield" />
              Network Configuration
            </Header>
            <Grid>
              <Grid.Column width={8}>
                <List>
                  <List.Item>
                    <List.Content>
                      <List.Header>NAT Type</List.Header>
                      <List.Description>
                        <Label
                          color={
                            stats?.natType === 'Direct' ? 'green' : 'yellow'
                          }
                        >
                          {stats?.natType || 'Unknown'}
                        </Label>
                      </List.Description>
                    </List.Content>
                  </List.Item>
                </List>
              </Grid.Column>
              <Grid.Column width={8}>
                <List>
                  <List.Item>
                    <List.Content>
                      <List.Header>Health Indicators</List.Header>
                      <List.Description>
                        <Label
                          color={
                            stats?.routingTableHealthy == null
                              ? 'grey'
                              : stats.routingTableHealthy
                                ? 'green'
                                : 'red'
                          }
                        >
                          Routing Table:{' '}
                          {stats?.routingTableHealthy == null
                            ? 'Unavailable'
                            : stats.routingTableHealthy
                              ? 'Healthy'
                              : 'Unhealthy'}
                        </Label>
                        <br />
                        <Label
                          color={
                            stats?.peerConnectivityHealthy == null
                              ? 'grey'
                              : stats.peerConnectivityHealthy
                                ? 'green'
                                : 'red'
                          }
                        >
                          Peer Connectivity:{' '}
                          {stats?.peerConnectivityHealthy == null
                            ? 'Unavailable'
                            : stats.peerConnectivityHealthy
                              ? 'Healthy'
                              : 'Unhealthy'}
                        </Label>
                        <br />
                        <Label
                          color={
                            stats?.messageFlowHealthy == null
                              ? 'grey'
                              : stats.messageFlowHealthy
                                ? 'green'
                                : 'red'
                          }
                        >
                          Message Flow:{' '}
                          {stats?.messageFlowHealthy == null
                            ? 'Unavailable'
                            : stats.messageFlowHealthy
                              ? 'Healthy'
                              : 'Unhealthy'}
                        </Label>
                      </List.Description>
                    </List.Content>
                  </List.Item>
                </List>
              </Grid.Column>
            </Grid>
          </Segment>
        </Grid.Column>

        <Grid.Column width={16}>
          <Segment>
            <Header as="h3">
              <Icon name="users" />
              Soulseek Mesh Rendezvous
            </Header>
            {(!rendezvousStatusError || rendezvousStatus) && (
              <Message
                icon
                warning={!rendezvousStatus?.enabled}
              >
                <Icon name={rendezvousStatus?.enabled ? 'privacy' : 'lock'} />
                <Message.Content>
                  <Message.Header>
                    {rendezvousStatus?.enabled
                      ? 'Opt-in public rendezvous is enabled'
                      : 'Opt-in public rendezvous is disabled'}
                  </Message.Header>
                  <p>
                    This feature uses the native Soulseek interest graph to find
                    other slskr mesh-capable accounts. Publishing the interest
                    tag makes this account visibly identifiable as a slskr mesh
                    participant.
                  </p>
                  <p>
                    Interest tag:{' '}
                    <code>{rendezvousStatus?.interestTag || 'slskr-mesh-v1'}</code>
                  </p>
                  {!rendezvousStatus?.enabled && (
                    <p>
                      Enable <code>mesh.enableSoulseekRendezvous</code> in
                      configuration before using these controls.
                    </p>
                  )}
                </Message.Content>
              </Message>
            )}
            {rendezvousStatusError && (
              <Message
                data-testid="mesh-rendezvous-status-error"
                error
              >
                <Message.Header>Rendezvous status unavailable</Message.Header>
                <p>{rendezvousStatusError}</p>
                <p>Showing the last successfully loaded rendezvous status when available.</p>
              </Message>
            )}
            {rendezvousMessage && (
              <Message
                negative={rendezvousMessage.negative}
                positive={rendezvousMessage.positive}
              >
                {rendezvousMessage.text}
              </Message>
            )}
            <Button.Group>
              <Button
                disabled={!rendezvousStatus?.enabled || rendezvousLoading}
                loading={rendezvousLoading}
                onClick={handleAddRendezvousInterest}
                positive
              >
                <Icon name="bullhorn" />
                Publish Interest
              </Button>
              <Button
                disabled={!rendezvousStatus?.enabled || rendezvousLoading}
                loading={rendezvousLoading}
                onClick={handleRemoveRendezvousInterest}
              >
                <Icon name="remove circle" />
                Remove Interest
              </Button>
              <Button
                disabled={!rendezvousStatus?.enabled || rendezvousLoading}
                loading={rendezvousLoading}
                onClick={handleLoadRendezvousUsers}
                primary
              >
                <Icon name="search" />
                Load Candidates
              </Button>
            </Button.Group>
            {rendezvousUsers.length > 0 && (
              <List
                divided
                relaxed
                style={{ marginTop: '1rem' }}
              >
                {rendezvousUsers.map((user) => (
                  <List.Item key={user.username || user.Username}>
                    <List.Icon
                      name="user"
                      verticalAlign="middle"
                    />
                    <List.Content>
                      <List.Header>{user.username || user.Username}</List.Header>
                      <List.Description>
                        Similarity rating:{' '}
                        {user.rating ?? user.Rating ?? 'not reported'}
                      </List.Description>
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            )}
            {capabilityRecords.length > 0 && (
              <List
                divided
                relaxed
                style={{ marginTop: '1rem' }}
              >
                {capabilityRecords.map((record) => (
                  <List.Item key={`${record.username}-${record.nonce}`}>
                    <List.Icon
                      name={record.signed ? 'certificate' : 'id card outline'}
                      verticalAlign="middle"
                    />
                    <List.Content>
                      <List.Header>{record.username}</List.Header>
                      <List.Description>
                        {record.peerId || 'unsigned peer'}, {' '}
                        {(record.features || []).join(', ') || 'no features'}, {' '}
                        overlay port {record.overlayPort || 'not advertised'}
                      </List.Description>
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            )}
          </Segment>
        </Grid.Column>

        <Grid.Column width={16}>
          <MeshEvidencePolicy runtimeProfile={runtimeProfile} />
        </Grid.Column>

        <Grid.Column width={16}>
          <RealmSubjectIndexConflicts />
        </Grid.Column>
      </Grid>
    </div>
  );
};

export default Mesh;
