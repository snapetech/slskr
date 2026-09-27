import React from 'react';
import RoomCreateModal from '../Rooms/RoomCreateModal';
import { channelLabel, normalizeConversationName } from './messagingWorkspaceState';
import {
  Button,
  Dropdown,
  Icon,
  Input,
  Label,
  Message,
  Popup,
  Segment,
} from 'semantic-ui-react';

const MessagingSidebar = ({ actions, state }) => {
  const {
    availableRoomsError,
    bridgedPodNames,
    chatTarget,
    conversations,
    joinedRooms,
    roomOptions,
    roomSearchLoading,
    visiblePodChannels,
    workspaceError,
  } = state;
  const {
    deleteConversation,
    fetchAvailableRooms,
    isWorkspaceActionPending,
    joinRoom,
    leavePod,
    leaveRoom,
    openPanel,
    refreshWorkspace,
    setChatTarget,
  } = actions;

  return (
        <Segment className="messaging-sidebar">
          <div className="messaging-sidebar-header">
            <div className="messaging-sidebar-title">
              <Icon name="comments" />
              Messages
            </div>
            <Popup
              content="Reload saved conversations and joined rooms from the daemon."
              trigger={
                <Button
                  aria-label="Refresh messages workspace"
                  icon="refresh"
                  onClick={() => refreshWorkspace()}
                  size="mini"
                  title="Refresh messages workspace"
                />
              }
            />
          </div>
          {workspaceError ? <Message warning>{workspaceError}</Message> : null}

          <div className="messaging-sidebar-section">
            <div className="messaging-sidebar-section-title">Direct Message</div>
            <div className="messaging-start-row">
              <Input
                aria-label="Chat username"
                fluid
                onChange={(event) => setChatTarget(event.target.value)}
                onKeyUp={(event) => {
                  if (event.key === 'Enter' && chatTarget.trim()) {
                    openPanel('chat', chatTarget);
                    setChatTarget('');
                  }
                }}
                placeholder="username"
                size="small"
                value={chatTarget}
              />
              <Popup
                content="Open a direct-message panel for this user."
                trigger={
                  <Button
                    aria-label="Open direct-message panel"
                    disabled={!chatTarget.trim()}
                    icon="comment"
                    onClick={() => {
                      openPanel('chat', chatTarget);
                      setChatTarget('');
                    }}
                    size="small"
                    title="Open direct-message panel"
                  />
                }
              />
            </div>
          </div>

          <div className="messaging-sidebar-section">
            <div className="messaging-sidebar-section-title">
              Saved Chats
              <Label size="mini">{conversations.length}</Label>
            </div>
            <div className="messaging-list">
              {conversations.map((conversation) => (
                <div
                  className="messaging-list-action-row"
                  key={conversation.username}
                >
                  <Popup
                    content="Open this conversation as a workspace panel."
                    trigger={
                      <Button
                        basic
                        className="messaging-list-button"
                        compact
                        onClick={() => openPanel('chat', conversation.username)}
                        size="small"
                      >
                        <Icon name="comment alternate" />
                        {conversation.username}
                        {bridgedPodNames.has(
                          normalizeConversationName(conversation.username),
                        ) && (
                          <Label
                            size="mini"
                            title="Pod direct channel is folded into this saved direct message."
                          >
                            pod
                          </Label>
                        )}
                        {conversation.hasUnAcknowledgedMessages && (
                          <Label
                            color="red"
                            size="mini"
                          >
                            {conversation.unAcknowledgedMessageCount}
                          </Label>
                        )}
                      </Button>
                    }
                  />
                  <Popup
                    content="Permanently delete this saved message thread."
                    trigger={
                      <Button
                        aria-label={`Delete message thread with ${conversation.username}`}
                        disabled={isWorkspaceActionPending(`chat:delete:${conversation.username}`)}
                        icon="trash alternate"
                        negative
                        onClick={() => deleteConversation(conversation.username)}
                        size="small"
                        title={`Delete message thread with ${conversation.username}`}
                      />
                    }
                  />
                </div>
              ))}
            </div>
          </div>

          <div className="messaging-sidebar-section">
            <div className="messaging-sidebar-section-title">Join Room</div>
            <div className="messaging-start-row">
              <Dropdown
                aria-label="Search rooms"
                clearable
                fluid
                loading={roomSearchLoading}
                onChange={(_, { value }) => joinRoom(value)}
                onOpen={fetchAvailableRooms}
                options={roomOptions}
                placeholder="Search rooms"
                search
                selection
                size="small"
              />
              <RoomCreateModal onCreateRoom={(roomName) => joinRoom(roomName)} />
            </div>
            {availableRoomsError ? (
              <Message negative>{availableRoomsError}</Message>
            ) : null}
          </div>

          <div className="messaging-sidebar-section">
            <div className="messaging-sidebar-section-title">
              Joined Rooms
              <Label size="mini">{joinedRooms.length}</Label>
            </div>
            <div className="messaging-list">
              {joinedRooms.map((roomName) => (
                <div
                  className="messaging-list-action-row"
                  key={roomName}
                >
                  <Popup
                    content="Open this room as a workspace panel."
                    trigger={
                      <Button
                        basic
                        className="messaging-list-button"
                        compact
                        onClick={() => openPanel('room', roomName)}
                        size="small"
                      >
                        <Icon name="comments" />
                        #{roomName}
                      </Button>
                    }
                  />
                  <Popup
                    content="Leave this room and remove it from joined rooms."
                    trigger={
                      <Button
                        aria-label={`Leave room ${roomName}`}
                        disabled={isWorkspaceActionPending(`room:leave:${roomName}`)}
                        icon="sign-out"
                        negative
                        onClick={() => leaveRoom(roomName)}
                        size="small"
                        title={`Leave room ${roomName}`}
                      />
                    }
                  />
                </div>
              ))}
            </div>
          </div>

          <div className="messaging-sidebar-section">
            <div className="messaging-sidebar-section-title">
              Pod Channels
              <Label size="mini">{visiblePodChannels.length}</Label>
            </div>
            <div className="messaging-list">
              {visiblePodChannels.map((channel) => (
                <div
                  className="messaging-list-action-row"
                  key={channel.target}
                >
                  <Popup
                    content="Open this pod channel in the unified message workspace."
                    trigger={
                      <Button
                        basic
                        className="messaging-list-button"
                        compact
                        onClick={() =>
                          openPanel('pod', channel.target, {
                            label: channelLabel(channel),
                          })
                        }
                        size="small"
                      >
                        <Icon name="comments outline" />
                        {channelLabel(channel)}
                      </Button>
                    }
                  />
                  <Popup
                    content="Leave this pod and remove its channels from Messages."
                    trigger={
                      <Button
                        aria-label={`Leave pod ${channel.podName}`}
                        disabled={isWorkspaceActionPending(`pod:leave:${channel.podId}`)}
                        icon="sign-out"
                        negative
                        onClick={() => leavePod(channel)}
                        size="small"
                        title={`Leave pod ${channel.podName}`}
                      />
                    }
                  />
                </div>
              ))}
            </div>
          </div>
        </Segment>
  );
};

export default MessagingSidebar;
