import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import PodMessageMaintenanceActions from './PodMessageMaintenanceActions';
import PodWorkflowNotice from './PodWorkflowNotice';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import {
  Card,
  Form,
  Grid,
  Header,
  Icon,
  Message,
} from 'semantic-ui-react';

const useMountedState = (mountedRef, initialValue) => {
  const [value, setValue] = React.useState(initialValue);
  const setMountedValue = React.useCallback(
    (nextValue) => {
      if (mountedRef.current) {
        setValue(nextValue);
      }
    },
    [mountedRef],
  );

  return [value, setMountedValue];
};

const PodMessageRoutingPanel = ({ visible = true }) => {
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Pod Message Routing states
  const [routeMessageData, setRouteMessageData] = useMountedState(mountedRef, '');
  const [routingMessage, setRoutingMessage] = useMountedState(mountedRef, false);
  const [routingResult, setRoutingResult] = useMountedState(mountedRef, null);
  const [routeToPeersMessage, setRouteToPeersMessage] = useMountedState(mountedRef, '');
  const [routeToPeersIds, setRouteToPeersIds] = useMountedState(mountedRef, '');
  const [routingToPeers, setRoutingToPeers] = useMountedState(mountedRef, false);
  const [routingToPeersResult, setRoutingToPeersResult] = useMountedState(mountedRef, null);
  const [routingStats, setRoutingStats] = useMountedState(mountedRef, null);
  const [loadingRoutingStats, setLoadingRoutingStats] = useMountedState(mountedRef, false);
  const [checkMessageId, setCheckMessageId] = useMountedState(mountedRef, '');
  const [checkPodId, setCheckPodId] = useMountedState(mountedRef, '');
  const [checkingMessageSeen, setCheckingMessageSeen] = useMountedState(mountedRef, false);
  const [messageSeenResult, setMessageSeenResult] = useMountedState(mountedRef, null);


  const handleRouteMessage = async () => {
    if (!routeMessageData.trim()) {
      toast.warning('Please enter message JSON data');
      return;
    }

    try {
      setRoutingMessage(true);
      setRoutingResult(null);
      const message = JSON.parse(routeMessageData);
      const result = await mediacore.routePodMessage(message);
      setRoutingResult(result);
      setRouteMessageData('');
    } catch (error_) {
      setRoutingResult({ error: toDisplayError(error_) });
    } finally {
      setRoutingMessage(false);
    }
  };

  const handleRouteMessageToPeers = async () => {
    if (!routeToPeersMessage.trim() || !routeToPeersIds.trim()) {
      toast.warning('Please enter message JSON and target peer IDs');
      return;
    }

    try {
      setRoutingToPeers(true);
      setRoutingToPeersResult(null);
      const message = JSON.parse(routeToPeersMessage);
      const targetPeerIds = routeToPeersIds
        .split(',')
        .map((id) => id.trim())
        .filter(Boolean);
      const result = await mediacore.routePodMessageToPeers(
        message,
        targetPeerIds,
      );
      setRoutingToPeersResult(result);
      setRouteToPeersMessage('');
      setRouteToPeersIds('');
    } catch (error_) {
      setRoutingToPeersResult({ error: toDisplayError(error_) });
    } finally {
      setRoutingToPeers(false);
    }
  };

  const handleLoadRoutingStats = async () => {
    try {
      setLoadingRoutingStats(true);
      setRoutingStats(null);
      const result = await mediacore.getPodMessageRoutingStats();
      setRoutingStats(result);
    } catch (error_) {
      setRoutingStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingRoutingStats(false);
    }
  };

  const handleCheckMessageSeen = async () => {
    if (!checkMessageId.trim() || !checkPodId.trim()) {
      toast.warning('Please enter both message ID and pod ID');
      return;
    }

    try {
      setCheckingMessageSeen(true);
      setMessageSeenResult(null);
      const result = await mediacore.checkMessageSeen(
        checkMessageId,
        checkPodId,
      );
      setMessageSeenResult(result);
    } catch (error_) {
      setMessageSeenResult({ error: toDisplayError(error_) });
    } finally {
      setCheckingMessageSeen(false);
    }
  };

  const handleRegisterMessageSeen = async () => {
    if (!checkMessageId.trim() || !checkPodId.trim()) {
      toast.warning('Please enter both message ID and pod ID');
      return;
    }

    try {
      const result = await mediacore.registerMessageSeen(
        checkMessageId,
        checkPodId,
      );
      toast.success(
        `Message registered as seen: ${result.wasNewlyRegistered ? 'New' : 'Already known'}`,
      );
    } catch (error_) {
      toast.error(`Failed to register message: ${toDisplayError(error_)}`);
    }
  };

  const handleCleanupSeenMessages = async () => {
    try {
      const result = await mediacore.cleanupSeenMessages();
      toast.success(
        `Cleanup completed: ${result.messagesCleaned} messages cleaned, ${result.messagesRetained} retained`,
      );
      // Reload stats to reflect changes
      await handleLoadRoutingStats();
    } catch (error_) {
      toast.error(`Failed to cleanup: ${toDisplayError(error_)}`);
    }
  };



  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-message-routing" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="send" />
                Pod Message Routing
              </Card.Header>
              <Card.Description>
                Decentralized message routing via overlay network with fanout
                and deduplication for reliable pod communication
              </Card.Description>
              <PodWorkflowNotice title="Sends pod messages">
                Routing actions can transmit message bodies and sender
                identifiers to selected peers or overlay routes. Use
                deduplication checks before resending.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Manual Message Routing */}
            <Card.Content>
              <Header size="small">Manual Message Routing</Header>
              <Form>
                <Form.TextArea
                  label="Pod Message JSON"
                  onChange={(e) => setRouteMessageData(e.target.value)}
                  placeholder='{"messageId": "msg123", "channelId": "pod:artist:mb:daft-punk-hash:general", "senderPeerId": "alice", "body": "Hello pod!", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature"}'
                  rows={4}
                  value={routeMessageData}
                />
                <Button
                  disabled={routingMessage || !routeMessageData.trim()}
                  loading={routingMessage}
                  onClick={handleRouteMessage}
                  primary
                >
                  Route Message
                </Button>
              </Form>

              {routingResult && (
                <div style={{ marginTop: '1em' }}>
                  {routingResult.error ? (
                    <Message error>
                      <p>Failed to route message: {routingResult.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>
                        Message Routed Successfully
                      </Message.Header>
                      <p>
                        <strong>Message ID:</strong> {routingResult.messageId}
                        <br />
                        <strong>Pod ID:</strong> {routingResult.podId}
                        <br />
                        <strong>Target Peers:</strong>{' '}
                        {routingResult.targetPeerCount}
                        <br />
                        <strong>Successfully Routed:</strong>{' '}
                        {routingResult.successfullyRoutedCount}
                        <br />
                        <strong>Failed:</strong>{' '}
                        {routingResult.failedRoutingCount}
                        <br />
                        <strong>Duration:</strong>{' '}
                        {routingResult.routingDuration?.totalMilliseconds?.toFixed(
                          0,
                        )}
                        ms
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>

            <Card.Content>
              <Grid>
                <Grid.Column width={8}>
                  {/* Route to Specific Peers */}
                  <Header size="small">Route to Specific Peers</Header>
                  <Form>
                    <Form.TextArea
                      label="Pod Message JSON"
                      onChange={(e) => setRouteToPeersMessage(e.target.value)}
                      placeholder='{"messageId": "msg123", "channelId": "pod:artist:mb:daft-punk-hash:general", "senderPeerId": "alice", "body": "Direct message", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature"}'
                      rows={3}
                      value={routeToPeersMessage}
                    />
                    <Form.Input
                      label="Target Peer IDs (comma-separated)"
                      onChange={(e) => setRouteToPeersIds(e.target.value)}
                      placeholder="bob,charlie,diana"
                      value={routeToPeersIds}
                    />
                    <Button
                      disabled={
                        routingToPeers ||
                        !routeToPeersMessage.trim() ||
                        !routeToPeersIds.trim()
                      }
                      fluid
                      loading={routingToPeers}
                      onClick={handleRouteMessageToPeers}
                    >
                      Route to Peers
                    </Button>
                  </Form>

                  {routingToPeersResult && (
                    <div style={{ marginTop: '0.5em' }}>
                      {routingToPeersResult.error ? (
                        <Message
                          error
                          size="tiny"
                        >
                          <p>{routingToPeersResult.error}</p>
                        </Message>
                      ) : (
                        <Message
                          info
                          size="tiny"
                        >
                          <p>
                            Routed to{' '}
                            {routingToPeersResult.successfullyRoutedCount}/
                            {routingToPeersResult.targetPeerCount} peers
                          </p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>

                <Grid.Column width={8}>
                  {/* Message Deduplication */}
                  <Header size="small">Message Deduplication</Header>
                  <Form>
                    <Form.Group widths="equal">
                      <Form.Input
                        label="Message ID"
                        onChange={(e) => setCheckMessageId(e.target.value)}
                        placeholder="msg123"
                        value={checkMessageId}
                      />
                      <Form.Input
                        label="Pod ID"
                        onChange={(e) => setCheckPodId(e.target.value)}
                        placeholder="pod:artist:mb:daft-punk-hash"
                        value={checkPodId}
                      />
                    </Form.Group>
                    <Button.Group fluid>
                      <Button
                        disabled={
                          checkingMessageSeen ||
                          !checkMessageId.trim() ||
                          !checkPodId.trim()
                        }
                        loading={checkingMessageSeen}
                        onClick={handleCheckMessageSeen}
                      >
                        Check Seen
                      </Button>
                      <Button
                        color="blue"
                        disabled={!checkMessageId.trim() || !checkPodId.trim()}
                        onClick={handleRegisterMessageSeen}
                      >
                        Mark Seen
                      </Button>
                    </Button.Group>
                  </Form>

                  {messageSeenResult && (
                    <div style={{ marginTop: '0.5em' }}>
                      {messageSeenResult.error ? (
                        <Message
                          error
                          size="tiny"
                        >
                          <p>{messageSeenResult.error}</p>
                        </Message>
                      ) : (
                        <Message size="tiny">
                          <p>
                            Message{' '}
                            {messageSeenResult.isSeen
                              ? 'has been'
                              : 'has not been'}{' '}
                            seen in pod {messageSeenResult.podId}
                          </p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>
              </Grid>
            </Card.Content>

            {/* Routing Statistics */}
            <Card.Content>
              <Button.Group fluid>
                <Button
                  disabled={loadingRoutingStats}
                  loading={loadingRoutingStats}
                  onClick={handleLoadRoutingStats}
                  primary
                >
                  Load Routing Stats
                </Button>
                <Button
                  color="red"
                  onClick={handleCleanupSeenMessages}
                >
                  Cleanup Seen Messages
                </Button>
              </Button.Group>

              {routingStats && !routingStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Message Routing Statistics</Message.Header>
                    <p>
                      <strong>Total Messages Routed:</strong>{' '}
                      {routingStats.totalMessagesRouted}
                      <br />
                      <strong>Total Routing Attempts:</strong>{' '}
                      {routingStats.totalRoutingAttempts}
                      <br />
                      <strong>Successful Routes:</strong>{' '}
                      {routingStats.successfulRoutingCount}
                      <br />
                      <strong>Failed Routes:</strong>{' '}
                      {routingStats.failedRoutingCount}
                      <br />
                      <strong>Avg Routing Time:</strong>{' '}
                      {routingStats.averageRoutingTimeMs.toFixed(2)}ms
                      <br />
                      <strong>Deduplication Items:</strong>{' '}
                      {routingStats.activeDeduplicationItems}
                      <br />
                      <strong>Bloom Filter Fill:</strong>{' '}
                      {(routingStats.bloomFilterFillRatio * 100).toFixed(1)}%
                      <br />
                      <strong>Est. False Positive:</strong>{' '}
                      {(routingStats.estimatedFalsePositiveRate * 100).toFixed(
                        4,
                      )}
                      %<br />
                      <strong>Last Operation:</strong>{' '}
                      {routingStats.lastRoutingOperation
                        ? new Date(
                            routingStats.lastRoutingOperation,
                          ).toLocaleString()
                        : 'Never'}
                    </p>
                  </Message>

                  <PodMessageMaintenanceActions />
                </div>
              )}

              {routingStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>Failed to load routing stats: {routingStats.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

  );
};

export default React.memo(PodMessageRoutingPanel);
