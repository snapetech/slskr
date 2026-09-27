import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
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

const PodJoinLeavePanel = ({ visible = true }) => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Pod Join/Leave states
  const [joinRequestData, setJoinRequestData] = useMountedState(mountedRef, '');
  const [requestingJoin, setRequestingJoin] = useMountedState(mountedRef, false);
  const [joinRequestResult, setJoinRequestResult] = useMountedState(mountedRef, null);
  const [acceptanceData, setAcceptanceData] = useMountedState(mountedRef, '');
  const [acceptingJoin, setAcceptingJoin] = useMountedState(mountedRef, false);
  const [acceptanceResult, setAcceptanceResult] = useMountedState(mountedRef, null);
  const [leaveRequestData, setLeaveRequestData] = useMountedState(mountedRef, '');
  const [requestingLeave, setRequestingLeave] = useMountedState(mountedRef, false);
  const [leaveRequestResult, setLeaveRequestResult] = useMountedState(mountedRef, null);
  const [acceptingLeave, setAcceptingLeave] = useMountedState(mountedRef, false);
  const [leaveAcceptanceResult, setLeaveAcceptanceResult] = useMountedState(mountedRef, null);
  const [pendingPodId, setPendingPodId] = useMountedState(mountedRef, '');
  const [loadingPendingRequests, setLoadingPendingRequests] = useMountedState(mountedRef, false);
  const [pendingJoinRequests, setPendingJoinRequests] = useMountedState(mountedRef, null);
  const [pendingLeaveRequests, setPendingLeaveRequests] = useMountedState(mountedRef, null);


  const handleRequestJoin = async () => {
    if (!joinRequestData.trim()) {
      toast.warning('Please enter join request JSON data');
      return;
    }

    try {
      setRequestingJoin(true);
      setJoinRequestResult(null);
      const joinRequest = JSON.parse(joinRequestData);
      const result = await mediacore.requestPodJoin(joinRequest);
      setJoinRequestResult(result);
      setJoinRequestData('');
    } catch (error_) {
      setJoinRequestResult({ error: toDisplayError(error_) });
    } finally {
      setRequestingJoin(false);
    }
  };

  const handleAcceptJoin = async () => {
    if (!acceptanceData.trim()) {
      toast.warning('Please enter acceptance JSON data');
      return;
    }

    try {
      setAcceptingJoin(true);
      setAcceptanceResult(null);
      const acceptance = JSON.parse(acceptanceData);
      const result = await mediacore.acceptPodJoin(acceptance);
      setAcceptanceResult(result);
      setAcceptanceData('');
    } catch (error_) {
      setAcceptanceResult({ error: toDisplayError(error_) });
    } finally {
      setAcceptingJoin(false);
    }
  };

  const handleRequestLeave = async () => {
    if (!leaveRequestData.trim()) {
      toast.warning('Please enter leave request JSON data');
      return;
    }

    try {
      setRequestingLeave(true);
      setLeaveRequestResult(null);
      const leaveRequest = JSON.parse(leaveRequestData);
      const result = await mediacore.requestPodLeave(leaveRequest);
      setLeaveRequestResult(result);
      setLeaveRequestData('');
    } catch (error_) {
      setLeaveRequestResult({ error: toDisplayError(error_) });
    } finally {
      setRequestingLeave(false);
    }
  };

  const handleAcceptLeave = async () => {
    if (!acceptanceData.trim()) {
      toast.warning('Please enter leave acceptance JSON data');
      return;
    }

    try {
      setAcceptingLeave(true);
      setLeaveAcceptanceResult(null);
      const acceptance = JSON.parse(acceptanceData);
      const result = await mediacore.acceptPodLeave(acceptance);
      setLeaveAcceptanceResult(result);
      setAcceptanceData('');
    } catch (error_) {
      setLeaveAcceptanceResult({ error: toDisplayError(error_) });
    } finally {
      setAcceptingLeave(false);
    }
  };

  const handleLoadPendingRequests = async () => {
    if (!pendingPodId.trim()) {
      toast.warning('Please enter a pod ID');
      return;
    }

    try {
      setLoadingPendingRequests(true);
      setPendingJoinRequests(null);
      setPendingLeaveRequests(null);

      const [joinRequests, leaveRequests] = await Promise.all([
        mediacore.getPendingJoinRequests(pendingPodId),
        mediacore.getPendingLeaveRequests(pendingPodId),
      ]);

      setPendingJoinRequests(joinRequests);
      setPendingLeaveRequests(leaveRequests);
    } catch (error_) {
      setPendingJoinRequests({ error: toDisplayError(error_) });
      setPendingLeaveRequests({ error: toDisplayError(error_) });
    } finally {
      setLoadingPendingRequests(false);
    }
  };


  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-join-leave" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="user plus" />
                Pod Join/Leave Operations
              </Card.Header>
              <Card.Description>
                Manage signed pod membership operations with Ed25519
                verification and role-based approvals
              </Card.Description>
              <PodWorkflowNotice title="Publishes join and leave events">
                Join and leave requests can expose peer IDs, requested roles,
                public keys, signatures, and operator-provided messages.
              </PodWorkflowNotice>
              <Message info>
                In Enforce mode, signatures use the
                <code> ed25519:&lt;base64 signature&gt;</code> format over the
                canonical join payload and require a fresh one-use nonce.
              </Message>
            </Card.Content>

            {/* Join Request */}
            <Card.Content>
              <Header size="small">Request to Join Pod</Header>
              <Form>
                <Form.TextArea
                  label="Join Request JSON (signed by requester)"
                  onChange={(e) => setJoinRequestData(e.target.value)}
                  placeholder='{"podId": "pod:artist:mb:daft-punk-hash", "peerId": "alice", "requestedRole": "member", "publicKey": "base64-ed25519-public-key", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature", "nonce": "unique-request-nonce", "message": "Please let me join!"}'
                  rows={4}
                  value={joinRequestData}
                />
                <Button
                  disabled={requestingJoin || !joinRequestData.trim()}
                  loading={requestingJoin}
                  onClick={handleRequestJoin}
                  primary
                >
                  Submit Join Request
                </Button>
              </Form>

              {joinRequestResult && (
                <div style={{ marginTop: '1em' }}>
                  {joinRequestResult.error ? (
                    <Message error>
                      <p>
                        Failed to submit join request: {joinRequestResult.error}
                      </p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>Join Request Submitted</Message.Header>
                      <p>
                        <strong>Pod ID:</strong> {joinRequestResult.podId}
                        <br />
                        <strong>Peer ID:</strong> {joinRequestResult.peerId}
                        <br />
                        <strong>Status:</strong>{' '}
                        {joinRequestResult.success
                          ? 'Pending approval'
                          : 'Failed'}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>

            <Card.Content>
              <Grid>
                <Grid.Column width={8}>
                  {/* Accept Join */}
                  <Header size="small">Accept Join Request</Header>
                  <Form>
                    <Form.TextArea
                      label="Acceptance JSON (signed by owner/mod)"
                      onChange={(e) => setAcceptanceData(e.target.value)}
                      placeholder='{"podId": "pod:artist:mb:daft-punk-hash", "peerId": "alice", "acceptedRole": "member", "acceptorPeerId": "bob", "acceptorPublicKey": "base64-ed25519-public-key", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature", "message": "Welcome!"}'
                      rows={4}
                      value={acceptanceData}
                    />
                    <Button
                      disabled={acceptingJoin || !acceptanceData.trim()}
                      loading={acceptingJoin}
                      onClick={handleAcceptJoin}
                      positive
                    >
                      Accept Join
                    </Button>
                  </Form>

                  {acceptanceResult && (
                    <div style={{ marginTop: '0.5em' }}>
                      {acceptanceResult.error ? (
                        <Message
                          error
                          size="tiny"
                        >
                          <p>{acceptanceResult.error}</p>
                        </Message>
                      ) : (
                        <Message
                          size="tiny"
                          success
                        >
                          <p>Join accepted successfully</p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>

                <Grid.Column width={8}>
                  {/* Leave Request */}
                  <Header size="small">Request to Leave Pod</Header>
                  <Form>
                    <Form.TextArea
                      label="Leave Request JSON (signed by member)"
                      onChange={(e) => setLeaveRequestData(e.target.value)}
                      placeholder='{"podId": "pod:artist:mb:daft-punk-hash", "peerId": "alice", "publicKey": "base64-ed25519-public-key", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature", "message": "Goodbye!"}'
                      rows={4}
                      value={leaveRequestData}
                    />
                    <Button
                      disabled={requestingLeave || !leaveRequestData.trim()}
                      loading={requestingLeave}
                      onClick={handleRequestLeave}
                    >
                      Submit Leave Request
                    </Button>
                  </Form>

                  {leaveRequestResult && (
                    <div style={{ marginTop: '0.5em' }}>
                      {leaveRequestResult.error ? (
                        <Message
                          error
                          size="tiny"
                        >
                          <p>{leaveRequestResult.error}</p>
                        </Message>
                      ) : (
                        <Message
                          size="tiny"
                          success
                        >
                          <p>Leave request submitted</p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>
              </Grid>
            </Card.Content>

            <Card.Content>
              <Grid>
                <Grid.Column width={8}>
                  {/* Accept Leave */}
                  <Header size="small">
                    Accept Leave Request (Owner/Mod Only)
                  </Header>
                  <Form>
                    <Form.TextArea
                      label="Leave Acceptance JSON (signed by owner/mod)"
                      onChange={(e) => setAcceptanceData(e.target.value)}
                      placeholder='{"podId": "pod:artist:mb:daft-punk-hash", "peerId": "alice", "acceptorPeerId": "bob", "acceptorPublicKey": "base64-ed25519-public-key", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature", "message": "Farewell!"}'
                      rows={4}
                      value={acceptanceData}
                    />
                    <Button
                      disabled={acceptingLeave || !acceptanceData.trim()}
                      loading={acceptingLeave}
                      negative
                      onClick={handleAcceptLeave}
                    >
                      Accept Leave
                    </Button>
                  </Form>

                  {leaveAcceptanceResult && (
                    <div style={{ marginTop: '0.5em' }}>
                      {leaveAcceptanceResult.error ? (
                        <Message
                          error
                          size="tiny"
                        >
                          <p>{leaveAcceptanceResult.error}</p>
                        </Message>
                      ) : (
                        <Message
                          size="tiny"
                          success
                        >
                          <p>Leave accepted successfully</p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>

                <Grid.Column width={8}>
                  {/* Pending Requests */}
                  <Header size="small">View Pending Requests</Header>
                  <Form>
                    <Form.Input
                      label="Pod ID"
                      onChange={(e) => setPendingPodId(e.target.value)}
                      placeholder="pod:artist:mb:daft-punk-hash"
                      value={pendingPodId}
                    />
                    <Button
                      disabled={loadingPendingRequests || !pendingPodId.trim()}
                      loading={loadingPendingRequests}
                      onClick={handleLoadPendingRequests}
                    >
                      Load Pending Requests
                    </Button>
                  </Form>

                  {pendingJoinRequests && !pendingJoinRequests.error && (
                    <div style={{ marginTop: '0.5em' }}>
                      <Message size="tiny">
                        <strong>Join Requests:</strong>{' '}
                        {pendingJoinRequests.pendingJoinRequests?.length || 0}
                      </Message>
                    </div>
                  )}

                  {pendingLeaveRequests && !pendingLeaveRequests.error && (
                    <div style={{ marginTop: '0.5em' }}>
                      <Message size="tiny">
                        <strong>Leave Requests:</strong>{' '}
                        {pendingLeaveRequests.pendingLeaveRequests?.length || 0}
                      </Message>
                    </div>
                  )}

                  {(pendingJoinRequests?.error ||
                    pendingLeaveRequests?.error) && (
                    <Message
                      error
                      size="tiny"
                      style={{ marginTop: '0.5em' }}
                    >
                      <p>Failed to load pending requests</p>
                    </Message>
                  )}
                </Grid.Column>
              </Grid>
            </Card.Content>
          </Card>
        </Grid.Column>

  );
};

export default React.memo(PodJoinLeavePanel);
