import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import PodWorkflowNotice from './PodWorkflowNotice';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import {
  Card,
  Grid,
  Header,
  Icon,
  Input,
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

const PodChannelManagementPanel = ({ visible = true }) => {
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Pod Channel Management states
  const [channels, setChannels] = useMountedState(mountedRef, []);
  const [channelsLoadedFor, setChannelsLoadedFor] = useMountedState(
    mountedRef,
    null,
  );
  const [channelsError, setChannelsError] = useMountedState(mountedRef, null);
  const [channelsLoading, setChannelsLoading] = useMountedState(mountedRef, false);
  const [createChannelLoading, setCreateChannelLoading] = useMountedState(mountedRef, false);
  const [updateChannelLoading, setUpdateChannelLoading] = useMountedState(mountedRef, false);
  const [deleteChannelLoading, setDeleteChannelLoading] = useMountedState(mountedRef, false);
  const [channelPodId, setChannelPodId] = useMountedState(mountedRef, '');
  const [newChannelName, setNewChannelName] = useMountedState(mountedRef, '');
  const [newChannelKind, setNewChannelKind] = useMountedState(mountedRef, 'General');
  const [editingChannel, setEditingChannel] = useMountedState(mountedRef, null);
  const [editChannelName, setEditChannelName] = useMountedState(mountedRef, '');


  const hasCurrentChannels = channelsLoadedFor === channelPodId.trim();

  const handleGetChannels = async () => {
    const requestedPodId = channelPodId.trim();
    if (!requestedPodId) {
      toast.error('Pod ID is required');
      return;
    }

    try {
      setChannelsLoading(true);
      setChannelsError(null);
      const result = await mediacore.getChannels(requestedPodId);
      if (!Array.isArray(result)) {
        throw new Error('Invalid channels response');
      }
      setChannels(result);
      setChannelsLoadedFor(requestedPodId);
    } catch (error_) {
      const message = toDisplayError(error_, 'Failed to get channels');
      setChannelsError(message);
      toast.error(`Failed to get channels: ${message}`);
    } finally {
      setChannelsLoading(false);
    }
  };

  const handleCreateChannel = async () => {
    if (!channelPodId.trim()) {
      toast.error('Pod ID is required');
      return;
    }

    if (!newChannelName.trim()) {
      toast.error('Channel name is required');
      return;
    }

    try {
      setCreateChannelLoading(true);
      const channel = {
        kind: newChannelKind,
        name: newChannelName,
      };
      await mediacore.createChannel(channelPodId, channel);
      toast.success(`Channel "${newChannelName}" created successfully`);
      setNewChannelName('');
      // Refresh channels list
      await handleGetChannels();
    } catch (error_) {
      toast.error(`Failed to create channel: ${toDisplayError(error_)}`);
    } finally {
      setCreateChannelLoading(false);
    }
  };

  const handleUpdateChannel = async (channelId) => {
    if (!editChannelName.trim()) {
      toast.error('Channel name is required');
      return;
    }

    try {
      setUpdateChannelLoading(true);
      const updatedChannel = {
        channelId,
        kind: editingChannel.kind,
        name: editChannelName,
      };
      await mediacore.updateChannel(channelPodId, channelId, updatedChannel);
      toast.success(`Channel updated successfully`);
      setEditingChannel(null);
      setEditChannelName('');
      // Refresh channels list
      await handleGetChannels();
    } catch (error_) {
      toast.error(`Failed to update channel: ${toDisplayError(error_)}`);
    } finally {
      setUpdateChannelLoading(false);
    }
  };

  const handleDeleteChannel = async (channelId, channelName) => {
    if (
      !confirm(
        `Are you sure you want to delete the channel "${channelName}"? This action cannot be undone.`,
      )
    ) {
      return;
    }

    try {
      setDeleteChannelLoading(true);
      await mediacore.deleteChannel(channelPodId, channelId);
      toast.success(`Channel "${channelName}" deleted successfully`);
      // Refresh channels list
      await handleGetChannels();
    } catch (error_) {
      toast.error(`Failed to delete channel: ${toDisplayError(error_)}`);
    } finally {
      setDeleteChannelLoading(false);
    }
  };

  const startEditingChannel = (channel) => {
    setEditingChannel(channel);
    setEditChannelName(channel.name);
  };

  const cancelEditingChannel = () => {
    setEditingChannel(null);
    setEditChannelName('');
  };


  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-channel-management" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="hashtag" />
                Pod Channel Management
              </Card.Header>
              <Card.Description>
                Create, update, and manage channels within pods for organized
                messaging
              </Card.Description>
              <PodWorkflowNotice title="Mutates pod structure">
                Channel create, update, and delete actions change how pod
                messages are organized. Deleting channels can disrupt routing
                and history workflows.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Channel Management */}
            <Card.Content>
              <Header size="small">Pod Channel Operations</Header>

              <Input
                action={
                  <Button
                    color="blue"
                    disabled={!channelPodId.trim()}
                    loading={channelsLoading}
                    onClick={() => handleGetChannels()}
                  >
                    Load Channels
                  </Button>
                }
                onChange={(e) => {
                  const nextPodId = e.target.value;
                  setChannelPodId(nextPodId);
                  if (nextPodId.trim() !== channelPodId.trim()) {
                    setChannelsError(null);
                    setChannelsLoadedFor(null);
                  }
                }}
                placeholder="Pod ID for channel management"
                style={{ marginBottom: '1em', width: '100%' }}
                value={channelPodId}
              />

              {channelsError && (
                <Message
                  data-testid="media-core-channels-error"
                  negative
                  size="small"
                >
                  {channelsError}
                  {hasCurrentChannels && channels.length > 0 && (
                    <div>Showing last successfully loaded channels.</div>
                  )}
                </Message>
              )}

              {/* Create New Channel */}
              <Header size="tiny">Create New Channel</Header>
              <Input
                action={
                  <>
                    <select
                      aria-label="New channel type"
                      onChange={(e) => setNewChannelKind(e.target.value)}
                      style={{
                        border: '1px solid #ccc',
                        borderRadius: '4px',
                        padding: '0.5em',
                      }}
                      value={newChannelKind}
                    >
                      <option value="General">General</option>
                      <option value="Custom">Custom</option>
                      <option value="Bound">Bound</option>
                    </select>
                    <Button
                      color="green"
                      disabled={!newChannelName.trim() || !channelPodId.trim()}
                      loading={createChannelLoading}
                      onClick={() => handleCreateChannel()}
                    >
                      Create
                    </Button>
                  </>
                }
                onChange={(e) => setNewChannelName(e.target.value)}
                placeholder="Channel name"
                style={{ marginBottom: '1em', width: '100%' }}
                value={newChannelName}
              />

              {/* Channels List */}
              {hasCurrentChannels && channels.length > 0 && (
                <div>
                  <Header size="tiny">Existing Channels</Header>
                  <div style={{ maxHeight: '400px', overflowY: 'auto' }}>
                    {channels.map((channel) => (
                      <Card
                        key={channel.channelId}
                        style={{ marginBottom: '0.5em' }}
                      >
                        <Card.Content style={{ padding: '0.5em' }}>
                          {editingChannel &&
                          editingChannel.channelId === channel.channelId ? (
                            <div>
                              <Input
                                action={
                                  <>
                                    <Button
                                      color="green"
                                      disabled={!editChannelName.trim()}
                                      loading={updateChannelLoading}
                                      onClick={() =>
                                        handleUpdateChannel(channel.channelId)
                                      }
                                      size="small"
                                    >
                                      Save
                                    </Button>
                                    <Button
                                      onClick={() => cancelEditingChannel()}
                                      size="small"
                                    >
                                      Cancel
                                    </Button>
                                  </>
                                }
                                onChange={(e) =>
                                  setEditChannelName(e.target.value)
                                }
                                placeholder="Channel name"
                                style={{ width: '100%' }}
                                value={editChannelName}
                              />
                            </div>
                          ) : (
                            <div
                              style={{
                                alignItems: 'center',
                                display: 'flex',
                                justifyContent: 'space-between',
                              }}
                            >
                              <div>
                                <strong>{channel.name}</strong>
                                <div
                                  style={{
                                    color: '#666',
                                    fontSize: '0.8em',
                                    marginTop: '0.25em',
                                  }}
                                >
                                  ID: {channel.channelId} • Type: {channel.kind}
                                  {channel.bindingInfo &&
                                    ` • Binding: ${channel.bindingInfo}`}
                                </div>
                              </div>
                              <div>
                                <Button
                                  disabled={
                                    channel.name.toLowerCase() === 'general' &&
                                    channel.kind === 'General'
                                  }
                                  onClick={() => startEditingChannel(channel)}
                                  size="tiny"
                                >
                                  Edit
                                </Button>
                                <Button
                                  color="red"
                                  disabled={
                                    channel.name.toLowerCase() === 'general' &&
                                    channel.kind === 'General'
                                  }
                                  loading={deleteChannelLoading}
                                  onClick={() =>
                                    handleDeleteChannel(
                                      channel.channelId,
                                      channel.name,
                                    )
                                  }
                                  size="tiny"
                                >
                                  Delete
                                </Button>
                              </div>
                            </div>
                          )}
                        </Card.Content>
                      </Card>
                    ))}
                  </div>
                </div>
              )}

              {hasCurrentChannels &&
                channels.length === 0 &&
                channelPodId &&
                !channelsLoading &&
                !channelsError && (
                <Message
                  info
                  size="small"
                >
                  No channels found in pod {channelPodId}. Create the first
                  channel above.
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

  );
};

export default React.memo(PodChannelManagementPanel);
