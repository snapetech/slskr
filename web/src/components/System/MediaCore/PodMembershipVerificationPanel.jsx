import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import PodWorkflowNotice from './PodWorkflowNotice';
import React, { useRef } from 'react';
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

const PodMembershipVerificationPanel = React.memo(({
  messageToVerify,
  setMessageToVerify,
  setVerifyingMembership,
  verifyingMembership,
  visible,
}) => {
  const mountedRef = useRef(false);
  React.useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Pod Membership Verification states
  const [verifyPodId, setVerifyPodId] = useMountedState(mountedRef, '');
  const [verifyPeerId, setVerifyPeerId] = useMountedState(mountedRef, '');
  const [membershipVerificationResult, setMembershipVerificationResult] =
    useMountedState(mountedRef, null);
  const [membershipMessageToVerify, setMembershipMessageToVerify] =
    useMountedState(mountedRef, '');
  const [verifyingMessage, setVerifyingMessage] = useMountedState(mountedRef, false);
  const [messageVerificationResult, setMessageVerificationResult] =
    useMountedState(mountedRef, null);
  const [roleCheckPodId, setRoleCheckPodId] = useMountedState(mountedRef, '');
  const [roleCheckPeerId, setRoleCheckPeerId] = useMountedState(mountedRef, '');
  const [requiredRole, setRequiredRole] = useMountedState(mountedRef, 'member');
  const [checkingRole, setCheckingRole] = useMountedState(mountedRef, false);
  const [roleCheckResult, setRoleCheckResult] = useMountedState(mountedRef, null);
  const [verificationStats, setVerificationStats] = useMountedState(mountedRef, null);
  const [loadingVerificationStats, setLoadingVerificationStats] =
    useMountedState(mountedRef, false);


  // Pod Membership Verification handlers
  const handleVerifyPodMembership = async () => {
    if (!verifyPodId.trim() || !verifyPeerId.trim()) {
      toast.warning('Please enter both Pod ID and Peer ID');
      return;
    }

    try {
      setVerifyingMembership(true);
      setMembershipVerificationResult(null);
      const result = await mediacore.verifyPodMembership(
        verifyPodId,
        verifyPeerId,
      );
      setMembershipVerificationResult(result);
    } catch (error_) {
      setMembershipVerificationResult({ error: toDisplayError(error_) });
    } finally {
      setVerifyingMembership(false);
    }
  };

  const handleVerifyMessage = async () => {
    if (!membershipMessageToVerify.trim()) {
      toast.warning('Please enter a message JSON');
      return;
    }

    try {
      setVerifyingMessage(true);
      setMessageVerificationResult(null);
      const message = JSON.parse(membershipMessageToVerify);
      const result = await mediacore.verifyPodMessage(message);
      setMessageVerificationResult(result);
    } catch (error_) {
      setMessageVerificationResult({ error: toDisplayError(error_) });
    } finally {
      setVerifyingMessage(false);
    }
  };

  const handleCheckRole = async () => {
    if (!roleCheckPodId.trim() || !roleCheckPeerId.trim()) {
      toast.warning('Please enter both Pod ID and Peer ID');
      return;
    }

    try {
      setCheckingRole(true);
      setRoleCheckResult(null);
      const hasRole = await mediacore.checkPodRole(
        roleCheckPodId,
        roleCheckPeerId,
        requiredRole,
      );
      setRoleCheckResult({ hasRole });
    } catch (error_) {
      setRoleCheckResult({ error: toDisplayError(error_) });
    } finally {
      setCheckingRole(false);
    }
  };

  const handleLoadVerificationStats = async () => {
    try {
      setLoadingVerificationStats(true);
      setVerificationStats(null);
      const result = await mediacore.getVerificationStats();
      setVerificationStats(result);
    } catch (error_) {
      setVerificationStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingVerificationStats(false);
    }
  };

  return (
    <>
        {/* Pod Membership Verification */}
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-membership-verification" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="shield" />
                Pod Membership Verification
              </Card.Header>
              <Card.Description>
                Verify membership status, message authenticity, and role
                permissions for pod security
              </Card.Description>
              <PodWorkflowNotice
                color="blue"
                icon="check circle"
                title="Read-only verification"
              >
                Verification checks should not mutate pod state, but pasted
                messages and membership records can still contain sensitive
                peer or signature data.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Membership Verification */}
            <Card.Content>
              <Header size="small">Verify Membership Status</Header>
              <Form>
                <Form.Group widths="equal">
                  <Form.Input
                    label="Pod ID"
                    onChange={(e) => setVerifyPodId(e.target.value)}
                    placeholder="pod:artist:mb:daft-punk-hash"
                    value={verifyPodId}
                  />
                  <Form.Input
                    label="Peer ID"
                    onChange={(e) => setVerifyPeerId(e.target.value)}
                    placeholder="alice"
                    value={verifyPeerId}
                  />
                </Form.Group>
                <Button
                  disabled={
                    verifyingMembership ||
                    !verifyPodId.trim() ||
                    !verifyPeerId.trim()
                  }
                  fluid
                  loading={verifyingMembership}
                  onClick={handleVerifyPodMembership}
                >
                  Verify Membership
                </Button>
              </Form>

              {membershipVerificationResult && (
                <div style={{ marginTop: '1em' }}>
                  {membershipVerificationResult.error ? (
                    <Message error>
                      <p>
                        Failed to verify membership:{' '}
                        {membershipVerificationResult.error}
                      </p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>
                        Membership Verification Result
                      </Message.Header>
                      <p>
                        <strong>Valid Member:</strong>{' '}
                        {membershipVerificationResult.isValidMember
                          ? 'Yes'
                          : 'No'}
                        <br />
                        <strong>Role:</strong>{' '}
                        {membershipVerificationResult.role || 'None'}
                        <br />
                        <strong>Banned:</strong>{' '}
                        {membershipVerificationResult.isBanned ? 'Yes' : 'No'}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>

            <Card.Content>
              <Grid>
                <Grid.Column width={8}>
                  {/* Message Verification */}
                  <Header size="small">Verify Message Authenticity</Header>
                  <Form>
                    <Form.TextArea
                      label="Pod Message JSON"
                      onChange={(e) => setMessageToVerify(e.target.value)}
                      placeholder='{"messageId": "msg123", "channelId": "pod:artist:mb:daft-punk-hash:general", "senderPeerId": "alice", "body": "Hello everyone!", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature"}'
                      rows={4}
                      value={messageToVerify}
                    />
                    <Button
                      disabled={verifyingMessage || !messageToVerify.trim()}
                      fluid
                      loading={verifyingMessage}
                      onClick={handleVerifyMessage}
                    >
                      Verify Message
                    </Button>
                  </Form>

                  {messageVerificationResult && (
                    <div style={{ marginTop: '1em' }}>
                      {messageVerificationResult.error ? (
                        <Message error>
                          <p>
                            Failed to verify message:{' '}
                            {messageVerificationResult.error}
                          </p>
                        </Message>
                      ) : (
                        <Message info>
                          <Message.Header>
                            Message Verification Result
                          </Message.Header>
                          <p>
                            <strong>Valid:</strong>{' '}
                            {messageVerificationResult.isValid ? 'Yes' : 'No'}
                            <br />
                            <strong>From Valid Member:</strong>{' '}
                            {messageVerificationResult.isFromValidMember
                              ? 'Yes'
                              : 'No'}
                            <br />
                            <strong>Not Banned:</strong>{' '}
                            {messageVerificationResult.isNotBanned
                              ? 'Yes'
                              : 'No'}
                            <br />
                            <strong>Valid Signature:</strong>{' '}
                            {messageVerificationResult.hasValidSignature
                              ? 'Yes'
                              : 'No'}
                          </p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>

                <Grid.Column width={8}>
                  {/* Role Checking */}
                  <Header size="small">Check Role Permissions</Header>
                  <Form>
                    <Form.Group widths="equal">
                      <Form.Input
                        label="Pod ID"
                        onChange={(e) => setRoleCheckPodId(e.target.value)}
                        placeholder="pod:artist:mb:daft-punk-hash"
                        value={roleCheckPodId}
                      />
                      <Form.Input
                        label="Peer ID"
                        onChange={(e) => setRoleCheckPeerId(e.target.value)}
                        placeholder="alice"
                        value={roleCheckPeerId}
                      />
                    </Form.Group>
                    <Form.Select
                      label="Required Role"
                      onChange={(e, { value }) => setRequiredRole(value)}
                      options={[
                        { key: 'member', text: 'Member', value: 'member' },
                        { key: 'mod', text: 'Moderator', value: 'mod' },
                        { key: 'owner', text: 'Owner', value: 'owner' },
                      ]}
                      value={requiredRole}
                    />
                    <Button
                      disabled={
                        checkingRole ||
                        !roleCheckPodId.trim() ||
                        !roleCheckPeerId.trim()
                      }
                      fluid
                      loading={checkingRole}
                      onClick={handleCheckRole}
                    >
                      Check Role
                    </Button>
                  </Form>

                  {roleCheckResult && (
                    <div style={{ marginTop: '1em' }}>
                      {roleCheckResult.error ? (
                        <Message error>
                          <p>Failed to check role: {roleCheckResult.error}</p>
                        </Message>
                      ) : (
                        <Message>
                          <Message.Header>Role Check Result</Message.Header>
                          <p>
                            <strong>Has Required Role ({requiredRole}):</strong>{' '}
                            {roleCheckResult.hasRole ? 'Yes' : 'No'}
                          </p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>
              </Grid>
            </Card.Content>

            {/* Verification Statistics */}
            <Card.Content>
              <Button.Group fluid>
                <Button
                  disabled={loadingVerificationStats}
                  loading={loadingVerificationStats}
                  onClick={handleLoadVerificationStats}
                  primary
                >
                  Load Verification Stats
                </Button>
              </Button.Group>

              {verificationStats && !verificationStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Verification Statistics</Message.Header>
                    <p>
                      <strong>Total Verifications:</strong>{' '}
                      {verificationStats.totalVerifications}
                      <br />
                      <strong>Successful:</strong>{' '}
                      {verificationStats.successfulVerifications}
                      <br />
                      <strong>Failed Membership:</strong>{' '}
                      {verificationStats.failedMembershipChecks}
                      <br />
                      <strong>Failed Signatures:</strong>{' '}
                      {verificationStats.failedSignatureChecks}
                      <br />
                      <strong>Banned Rejections:</strong>{' '}
                      {verificationStats.bannedMemberRejections}
                      <br />
                      <strong>Avg Time:</strong>{' '}
                      {verificationStats.averageVerificationTimeMs.toFixed(2)}ms
                      <br />
                      <strong>Last Verification:</strong>{' '}
                      {verificationStats.lastVerification
                        ? new Date(
                            verificationStats.lastVerification,
                          ).toLocaleString()
                        : 'Never'}
                    </p>
                  </Message>
                </div>
              )}

              {verificationStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>
                    Failed to load verification stats: {verificationStats.error}
                  </p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>
    </>
  );
});

export default PodMembershipVerificationPanel;
