import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import PodWorkflowNotice from './PodWorkflowNotice';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import { Card, Grid, Header, Icon, Input, Message } from 'semantic-ui-react';

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

const PodMessageBackfillPanel = ({ visible = true }) => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Pod Message Backfill states
  const [backfillStats, setBackfillStats] = useMountedState(mountedRef, null);
  const [backfillStatsError, setBackfillStatsError] = useMountedState(
    mountedRef,
    null,
  );
  const [backfillStatsLoading, setBackfillStatsLoading] = useMountedState(mountedRef, false);
  const [syncBackfillLoading, setSyncBackfillLoading] = useMountedState(mountedRef, false);
  const [lastSeenTimestamps, setLastSeenTimestamps] = useMountedState(mountedRef, null);
  const [lastSeenTimestampsLoadedFor, setLastSeenTimestampsLoadedFor] =
    useMountedState(mountedRef, null);
  const [lastSeenTimestampsError, setLastSeenTimestampsError] = useMountedState(
    mountedRef,
    null,
  );
  const [lastSeenTimestampsLoading, setLastSeenTimestampsLoading] =
    useMountedState(mountedRef, false);
  const [backfillPodId, setBackfillPodId] = useMountedState(mountedRef, '');


  const hasCurrentLastSeenTimestamps =
    lastSeenTimestampsLoadedFor === backfillPodId.trim();

  const handleGetBackfillStats = async () => {
    try {
      setBackfillStatsLoading(true);
      setBackfillStatsError(null);
      const result = await mediacore.getBackfillStats();
      if (
        result === null ||
        typeof result !== 'object' ||
        Array.isArray(result)
      ) {
        throw new Error('Invalid backfill statistics response');
      }
      setBackfillStats(result);
    } catch (error_) {
      const message = toDisplayError(
        error_,
        'Failed to get backfill statistics',
      );
      setBackfillStatsError(message);
      toast.error(`Failed to get backfill stats: ${message}`);
    } finally {
      setBackfillStatsLoading(false);
    }
  };

  const handleSyncPodBackfill = async () => {
    const requestedPodId = backfillPodId.trim();
    if (!requestedPodId) {
      toast.error('Pod ID is required for backfill sync');
      return;
    }

    try {
      setSyncBackfillLoading(true);
      // Get current last seen timestamps
      const timestamps = await mediacore.getLastSeenTimestamps(requestedPodId);
      const result = await mediacore.syncPodBackfill(requestedPodId, timestamps);
      toast.success(
        `Backfill sync completed: ${result.totalMessagesReceived} messages received`,
      );
      // Refresh stats
      await handleGetBackfillStats();
    } catch (error_) {
      toast.error(`Failed to sync pod backfill: ${toDisplayError(error_)}`);
    } finally {
      setSyncBackfillLoading(false);
    }
  };

  const handleGetLastSeenTimestamps = async () => {
    const requestedPodId = backfillPodId.trim();
    if (!requestedPodId) {
      toast.error('Pod ID is required');
      return;
    }

    try {
      setLastSeenTimestampsLoading(true);
      setLastSeenTimestampsError(null);
      const timestamps = await mediacore.getLastSeenTimestamps(requestedPodId);
      if (
        timestamps === null ||
        typeof timestamps !== 'object' ||
        Array.isArray(timestamps)
      ) {
        throw new Error('Invalid last-seen timestamps response');
      }
      setLastSeenTimestamps(timestamps);
      setLastSeenTimestampsLoadedFor(requestedPodId);
    } catch (error_) {
      const message = toDisplayError(
        error_,
        'Failed to get last seen timestamps',
      );
      setLastSeenTimestampsError(message);
      toast.error(`Failed to get last seen timestamps: ${message}`);
    } finally {
      setLastSeenTimestampsLoading(false);
    }
  };


  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-message-backfill" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="sync" />
                Pod Message Backfill
              </Card.Header>
              <Card.Description>
                Synchronize missed messages when peers rejoin pods
              </Card.Description>
              <PodWorkflowNotice title="Syncs local pod state">
                Backfill sync uses last-seen timestamps to request missed
                messages. Confirm the target pod before syncing all channels.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Message Backfill */}
            <Card.Content>
              <Header size="small">Backfill Management</Header>

              <div style={{ marginBottom: '1em' }}>
                <Button
                  color="purple"
                  loading={backfillStatsLoading}
                  onClick={() => handleGetBackfillStats()}
                  size="small"
                >
                  Get Backfill Stats
                </Button>
              </div>

              {backfillStatsError && (
                <Message
                  data-testid="media-core-backfill-stats-error"
                  negative
                  size="small"
                >
                  {backfillStatsError}
                  {backfillStats && (
                    <div>Showing last successfully loaded statistics.</div>
                  )}
                </Message>
              )}

              {backfillStats && (
                <Message
                  size="small"
                  style={{ marginBottom: '1em' }}
                >
                  <Message.Header>Backfill Statistics</Message.Header>
                  <p>
                    <strong>Requests Sent:</strong>{' '}
                    {backfillStats.totalBackfillRequestsSent?.toLocaleString() ||
                      0}
                    <br />
                    <strong>Requests Received:</strong>{' '}
                    {backfillStats.totalBackfillRequestsReceived?.toLocaleString() ||
                      0}
                    <br />
                    <strong>Messages Backfilled:</strong>{' '}
                    {backfillStats.totalMessagesBackfilled?.toLocaleString() ||
                      0}
                    <br />
                    <strong>Data Transferred:</strong>{' '}
                    {(
                      backfillStats.totalBackfillBytesTransferred /
                      (1_024 * 1_024)
                    ).toFixed(2)}{' '}
                    MB
                    <br />
                    <strong>Avg Duration:</strong>{' '}
                    {backfillStats.averageBackfillDurationMs?.toFixed(2) || 0}ms
                    <br />
                    <strong>Last Operation:</strong>{' '}
                    {backfillStats.lastBackfillOperation
                      ? new Date(
                          backfillStats.lastBackfillOperation,
                        ).toLocaleString()
                      : 'Never'}
                  </p>
                </Message>
              )}

              <Header size="small">Pod Backfill Sync</Header>
              <Input
                action={
                  <>
                    <Button
                      color="blue"
                      disabled={!backfillPodId.trim()}
                      loading={lastSeenTimestampsLoading}
                      onClick={() => handleGetLastSeenTimestamps()}
                    >
                      Get Timestamps
                    </Button>
                    <Button
                      color="green"
                      disabled={!backfillPodId.trim()}
                      loading={syncBackfillLoading}
                      onClick={() => handleSyncPodBackfill()}
                    >
                      Sync Backfill
                    </Button>
                  </>
                }
                onChange={(e) => {
                  const nextPodId = e.target.value;
                  setBackfillPodId(nextPodId);
                  if (nextPodId.trim() !== backfillPodId.trim()) {
                    setLastSeenTimestampsError(null);
                    setLastSeenTimestampsLoadedFor(null);
                  }
                }}
                placeholder="Pod ID for backfill sync"
                style={{ marginBottom: '1em', width: '100%' }}
                value={backfillPodId}
              />

              {lastSeenTimestampsError && (
                <Message
                  data-testid="media-core-last-seen-error"
                  negative
                  size="small"
                >
                  {lastSeenTimestampsError}
                  {hasCurrentLastSeenTimestamps &&
                    Object.keys(lastSeenTimestamps).length > 0 && (
                    <div>
                      Showing last successfully loaded timestamps.
                    </div>
                  )}
                </Message>
              )}

              {hasCurrentLastSeenTimestamps &&
                lastSeenTimestamps &&
                Object.keys(lastSeenTimestamps).length > 0 && (
                  <Message size="small">
                    <Message.Header>
                      Last Seen Timestamps for Pod {backfillPodId}
                    </Message.Header>
                    <div style={{ maxHeight: '150px', overflowY: 'auto' }}>
                      {Object.entries(lastSeenTimestamps).map(
                        ([channelId, timestamp]) => (
                          <div
                            key={channelId}
                            style={{ marginBottom: '0.25em' }}
                          >
                            <strong>{channelId}:</strong>{' '}
                            {new Date(timestamp).toLocaleString()}
                          </div>
                        ),
                      )}
                    </div>
                  </Message>
                )}

              {hasCurrentLastSeenTimestamps &&
                lastSeenTimestamps &&
                Object.keys(lastSeenTimestamps).length === 0 &&
                !lastSeenTimestampsLoading &&
                !lastSeenTimestampsError && (
                  <Message
                    info
                    size="small"
                  >
                    No last seen timestamps recorded for pod {backfillPodId}
                  </Message>
                )}
            </Card.Content>
          </Card>
        </Grid.Column>

  );
};

export default React.memo(PodMessageBackfillPanel);
