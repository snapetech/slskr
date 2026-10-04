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
  Label,
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

const PodMembershipManagementPanel = ({
  setVerifyingMembership,
  verifyingMembership,
  visible = true,
}) => {
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Pod Membership states
  const [membershipRecord, setMembershipRecord] = useMountedState(mountedRef, '');
  const [publishingMembership, setPublishingMembership] = useMountedState(mountedRef, false);
  const [membershipPublishResult, setMembershipPublishResult] = useMountedState(mountedRef, null);
  const [membershipPodId, setMembershipPodId] = useMountedState(mountedRef, '');
  const [membershipPeerId, setMembershipPeerId] = useMountedState(mountedRef, '');
  const [gettingMembership, setGettingMembership] = useMountedState(mountedRef, false);
  const [membershipResult, setMembershipResult] = useMountedState(mountedRef, null);
  const [verifyingMembershipStatus, setVerifyingMembershipStatus] =
    useMountedState(mountedRef, false);
  const [membershipVerification, setMembershipVerification] = useMountedState(mountedRef, null);
  const [banningMember, setBanningMember] = useMountedState(mountedRef, false);
  const [banReason, setBanReason] = useMountedState(mountedRef, '');
  const [banResult, setBanResult] = useMountedState(mountedRef, null);
  const [changingRole, setChangingRole] = useMountedState(mountedRef, false);
  const [newRole, setNewRole] = useMountedState(mountedRef, 'member');
  const [roleChangeResult, setRoleChangeResult] = useMountedState(mountedRef, null);
  const [membershipStats, setMembershipStats] = useMountedState(mountedRef, null);
  const [loadingMembershipStats, setLoadingMembershipStats] = useMountedState(mountedRef, false);


  const handlePublishMembership = async () => {
    if (!membershipRecord.trim()) {
      toast.warning('Please enter membership record JSON data');
      return;
    }

    try {
      setPublishingMembership(true);
      setMembershipPublishResult(null);
      const record = JSON.parse(membershipRecord);
      const result = await mediacore.publishMembership(record);
      setMembershipPublishResult(result);
      setMembershipRecord('');
    } catch (error_) {
      setMembershipPublishResult({ error: toDisplayError(error_) });
    } finally {
      setPublishingMembership(false);
    }
  };

  const handleGetMembership = async () => {
    if (!membershipPodId.trim() || !membershipPeerId.trim()) {
      toast.warning('Please enter both Pod ID and Peer ID');
      return;
    }

    try {
      setGettingMembership(true);
      setMembershipResult(null);
      const result = await mediacore.getMembership(
        membershipPodId,
        membershipPeerId,
      );
      setMembershipResult(result);
    } catch (error_) {
      setMembershipResult({ error: toDisplayError(error_) });
    } finally {
      setGettingMembership(false);
    }
  };

  const handleVerifyMembership = async () => {
    if (!membershipPodId.trim() || !membershipPeerId.trim()) {
      toast.warning('Please enter both Pod ID and Peer ID');
      return;
    }

    try {
      setVerifyingMembership(true);
      setMembershipVerification(null);
      const result = await mediacore.verifyMembership(
        membershipPodId,
        membershipPeerId,
      );
      setMembershipVerification(result);
    } catch (error_) {
      setMembershipVerification({ error: toDisplayError(error_) });
    } finally {
      setVerifyingMembership(false);
    }
  };

  const handleBanMember = async () => {
    if (!membershipPodId.trim() || !membershipPeerId.trim()) {
      toast.warning('Please enter both Pod ID and Peer ID');
      return;
    }

    if (
      !confirm(
        `Are you sure you want to ban member "${membershipPeerId}" from pod "${membershipPodId}"?`,
      )
    ) {
      return;
    }

    try {
      setBanningMember(true);
      setBanResult(null);
      const result = await mediacore.banMember(
        membershipPodId,
        membershipPeerId,
        banReason || null,
      );
      setBanResult(result);
      setBanReason('');
    } catch (error_) {
      setBanResult({ error: toDisplayError(error_) });
    } finally {
      setBanningMember(false);
    }
  };

  const handleChangeRole = async () => {
    if (!membershipPodId.trim() || !membershipPeerId.trim()) {
      toast.warning('Please enter both Pod ID and Peer ID');
      return;
    }

    try {
      setChangingRole(true);
      setRoleChangeResult(null);
      const result = await mediacore.changeMemberRole(
        membershipPodId,
        membershipPeerId,
        newRole,
      );
      setRoleChangeResult(result);
    } catch (error_) {
      setRoleChangeResult({ error: toDisplayError(error_) });
    } finally {
      setChangingRole(false);
    }
  };

  const handleLoadMembershipStats = async () => {
    try {
      setLoadingMembershipStats(true);
      setMembershipStats(null);
      const result = await mediacore.getMembershipStats();
      setMembershipStats(result);
    } catch (error_) {
      setMembershipStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingMembershipStats(false);
    }
  };

  const handleCleanupMemberships = async () => {
    if (
      !confirm('Are you sure you want to cleanup expired membership records?')
    ) {
      return;
    }

    try {
      const result = await mediacore.cleanupExpiredMemberships();
      toast.success(
        `Cleanup completed: ${result.recordsCleaned} records cleaned, ${result.errorsEncountered} errors`,
      );
      // Reload stats to reflect changes
      await handleLoadMembershipStats();
    } catch (error_) {
      toast.error(`Failed to cleanup: ${toDisplayError(error_)}`);
    }
  };


  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-membership-management" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="users" />
                Pod Membership Management
              </Card.Header>
              <Card.Description>
                Manage signed membership records in DHT with role-based access
                control
              </Card.Description>
              <PodWorkflowNotice title="Publishes membership state">
                Membership records can reveal peer IDs, roles, bans, public
                keys, and pod participation history. Verify the record before
                publishing it.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Publish Membership */}
            <Card.Content>
              <Header size="small">Publish Membership Record</Header>
              <Form>
                <Form.TextArea
                  label="Membership Record JSON"
                  onChange={(e) => setMembershipRecord(e.target.value)}
                  placeholder='{"podId": "pod:artist:mb:daft-punk-hash", "peerId": "alice", "role": "member", "isBanned": false, "publicKey": "base64-ed25519-key", "joinedAt": "2024-01-01T00:00:00Z"}'
                  rows={4}
                  value={membershipRecord}
                />
                <Button
                  disabled={publishingMembership || !membershipRecord.trim()}
                  loading={publishingMembership}
                  onClick={handlePublishMembership}
                  primary
                >
                  Publish Membership
                </Button>
              </Form>

              {membershipPublishResult && (
                <div style={{ marginTop: '1em' }}>
                  {membershipPublishResult.error ? (
                    <Message error>
                      <p>
                        Failed to publish membership:{' '}
                        {membershipPublishResult.error}
                      </p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>
                        Membership Published Successfully
                      </Message.Header>
                      <p>
                        <strong>Pod ID:</strong> {membershipPublishResult.podId}
                        <br />
                        <strong>Peer ID:</strong>{' '}
                        {membershipPublishResult.peerId}
                        <br />
                        <strong>DHT Key:</strong>{' '}
                        {membershipPublishResult.dhtKey}
                        <br />
                        <strong>Published:</strong>{' '}
                        {new Date(
                          membershipPublishResult.publishedAt,
                        ).toLocaleString()}
                        <br />
                        <strong>Expires:</strong>{' '}
                        {new Date(
                          membershipPublishResult.expiresAt,
                        ).toLocaleString()}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>

            <Card.Content>
              <Grid>
                <Grid.Column width={8}>
                  {/* Get Membership */}
                  <Header size="small">Get Membership Record</Header>
                  <Form>
                    <Form.Input
                      label="Pod ID"
                      onChange={(e) => setMembershipPodId(e.target.value)}
                      placeholder="pod:artist:mb:daft-punk-hash"
                      value={membershipPodId}
                    />
                    <Form.Input
                      label="Peer ID"
                      onChange={(e) => setMembershipPeerId(e.target.value)}
                      placeholder="alice"
                      value={membershipPeerId}
                    />
                    <Button.Group fluid>
                      <Button
                        disabled={
                          gettingMembership ||
                          !membershipPodId.trim() ||
                          !membershipPeerId.trim()
                        }
                        loading={gettingMembership}
                        onClick={handleGetMembership}
                      >
                        Get Membership
                      </Button>
                      <Button
                        disabled={
                          verifyingMembership ||
                          !membershipPodId.trim() ||
                          !membershipPeerId.trim()
                        }
                        loading={verifyingMembership}
                        onClick={handleVerifyMembership}
                      >
                        Verify Membership
                      </Button>
                    </Button.Group>
                  </Form>

                  {/* Membership Results */}
                  {membershipResult && (
                    <div style={{ marginTop: '1em' }}>
                      {membershipResult.error ? (
                        <Message error>
                          <p>
                            Failed to get membership: {membershipResult.error}
                          </p>
                        </Message>
                      ) : membershipResult.found ? (
                        <Message success>
                          <Message.Header>Membership Found</Message.Header>
                          <p>
                            <strong>Pod ID:</strong> {membershipResult.podId}
                            <br />
                            <strong>Peer ID:</strong> {membershipResult.peerId}
                            <br />
                            <strong>Role:</strong>{' '}
                            {membershipResult.signedRecord?.membership?.role}
                            <br />
                            <strong>Banned:</strong>{' '}
                            {membershipResult.signedRecord?.membership?.isBanned
                              ? 'Yes'
                              : 'No'}
                            <br />
                            <strong>Signature Valid:</strong>{' '}
                            {membershipResult.isValidSignature ? 'Yes' : 'No'}
                            <br />
                            <strong>Joined:</strong>{' '}
                            {membershipResult.signedRecord?.membership?.joinedAt
                              ? new Date(
                                  membershipResult.signedRecord.membership.joinedAt,
                                ).toLocaleString()
                              : 'Unknown'}
                          </p>
                        </Message>
                      ) : (
                        <Message warning>
                          <p>Membership not found in DHT</p>
                        </Message>
                      )}
                    </div>
                  )}

                  {/* Verification Results */}
                  {membershipVerification && (
                    <div style={{ marginTop: '1em' }}>
                      {membershipVerification.error ? (
                        <Message error>
                          <p>
                            Failed to verify membership:{' '}
                            {membershipVerification.error}
                          </p>
                        </Message>
                      ) : (
                        <Message info>
                          <Message.Header>
                            Membership Verification
                          </Message.Header>
                          <p>
                            <strong>Valid Member:</strong>{' '}
                            {membershipVerification.isValidMember
                              ? 'Yes'
                              : 'No'}
                            <br />
                            <strong>Role:</strong>{' '}
                            {membershipVerification.role || 'None'}
                            <br />
                            <strong>Banned:</strong>{' '}
                            {membershipVerification.isBanned ? 'Yes' : 'No'}
                          </p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>

                <Grid.Column width={8}>
                  {/* Member Management */}
                  <Header size="small">Member Management</Header>

                  {/* Ban Member */}
                  <Form style={{ marginBottom: '1em' }}>
                    <Form.Input
                      label="Ban Reason (optional)"
                      onChange={(e) => setBanReason(e.target.value)}
                      placeholder="Violation of community rules"
                      value={banReason}
                    />
                    <Button
                      color="red"
                      disabled={
                        banningMember ||
                        !membershipPodId.trim() ||
                        !membershipPeerId.trim()
                      }
                      fluid
                      loading={banningMember}
                      onClick={handleBanMember}
                    >
                      Ban Member
                    </Button>
                  </Form>

                  {/* Change Role */}
                  <Form>
                    <Form.Select
                      label="New Role"
                      onChange={(e, { value }) => setNewRole(value)}
                      options={[
                        { key: 'member', text: 'Member', value: 'member' },
                        { key: 'mod', text: 'Moderator', value: 'mod' },
                        { key: 'owner', text: 'Owner', value: 'owner' },
                      ]}
                      value={newRole}
                    />
                    <Button
                      color="blue"
                      disabled={
                        changingRole ||
                        !membershipPodId.trim() ||
                        !membershipPeerId.trim()
                      }
                      fluid
                      loading={changingRole}
                      onClick={handleChangeRole}
                    >
                      Change Role
                    </Button>
                  </Form>

                  {/* Management Results */}
                  {banResult && (
                    <Message
                      style={{ marginTop: '1em' }}
                      success
                    >
                      <p>Member banned successfully</p>
                    </Message>
                  )}

                  {roleChangeResult && (
                    <Message
                      style={{ marginTop: '1em' }}
                      success
                    >
                      <p>Member role changed successfully</p>
                    </Message>
                  )}
                </Grid.Column>
              </Grid>
            </Card.Content>

            {/* Membership Statistics */}
            <Card.Content>
              <Button.Group fluid>
                <Button
                  disabled={loadingMembershipStats}
                  loading={loadingMembershipStats}
                  onClick={handleLoadMembershipStats}
                  primary
                >
                  Load Membership Stats
                </Button>
                <Button
                  color="orange"
                  onClick={handleCleanupMemberships}
                >
                  Cleanup Expired
                </Button>
              </Button.Group>

              {membershipStats && !membershipStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Membership Statistics</Message.Header>
                    <p>
                      <strong>Total Memberships:</strong>{' '}
                      {membershipStats.totalMemberships}
                      <br />
                      <strong>Active Memberships:</strong>{' '}
                      {membershipStats.activeMemberships}
                      <br />
                      <strong>Banned Memberships:</strong>{' '}
                      {membershipStats.bannedMemberships}
                      <br />
                      <strong>Expired Memberships:</strong>{' '}
                      {membershipStats.expiredMemberships}
                      <br />
                      <strong>Last Operation:</strong>{' '}
                      {membershipStats.lastOperation
                        ? new Date(
                            membershipStats.lastOperation,
                          ).toLocaleString()
                        : 'Never'}
                    </p>
                    {membershipStats.membershipsByRole &&
                      Object.keys(membershipStats.membershipsByRole).length >
                        0 && (
                        <div style={{ marginTop: '0.5em' }}>
                          <strong>Memberships by Role:</strong>
                          {Object.entries(
                            membershipStats.membershipsByRole,
                          ).map(([role, count]) => (
                            <Label
                              key={role}
                              size="tiny"
                              style={{ margin: '0.1em' }}
                            >
                              {role}: {count}
                            </Label>
                          ))}
                        </div>
                      )}
                  </Message>
                </div>
              )}

              {membershipStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>
                    Failed to load membership stats: {membershipStats.error}
                  </p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

  );
};

export default React.memo(PodMembershipManagementPanel);
