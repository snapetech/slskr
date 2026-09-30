import * as pods from '../../lib/pods';
import { toDisplayError } from '../../lib/errors';
import {
  historyCursor,
  isAbortError,
  mergeMessageHistory,
  messageKey,
} from '../../lib/messageHistory';
import { usePolling } from '../../lib/usePolling';
import PodListenAlongPanel from '../Player/PodListenAlongPanel';
import PlaceholderSegment from '../Shared/PlaceholderSegment';
import UserCard from '../Shared/UserCard';
import {
  MAX_POD_MESSAGES,
  asRecords,
  channelLabel,
  isPodDirectChannel,
} from './messagingWorkspaceState';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Button,
  Icon,
  Input,
  List,
  Message,
  Popup,
  Segment,
} from 'semantic-ui-react';

const PodChannelSession = ({ channel, state }) => {
  const [body, setBody] = useState('');
  const [members, setMembers] = useState([]);
  const [messages, setMessages] = useState([]);
  const mountedRef = useRef(false);
  const refreshRequestIdRef = useRef(0);
  const refreshAbortControllerRef = useRef(null);
  const refreshInFlightRef = useRef(null);
  const refreshPendingRef = useRef(false);
  const historyCursorRef = useRef(null);
  const refreshRef = useRef(null);
  const sendRequestIdRef = useRef(0);
  const sendInFlightRef = useRef(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState('');
  const [messagesLoadError, setMessagesLoadError] = useState('');

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshRequestIdRef.current += 1;
      refreshPendingRef.current = false;
      refreshAbortControllerRef.current?.abort();
      refreshAbortControllerRef.current = null;
      historyCursorRef.current = null;
      sendRequestIdRef.current += 1;
    };
  }, []);

  useEffect(() => {
    refreshRequestIdRef.current += 1;
    refreshPendingRef.current = false;
    refreshAbortControllerRef.current?.abort();
    refreshAbortControllerRef.current = null;
    historyCursorRef.current = null;
    setMessages([]);
    setMessagesLoadError('');
  }, [channel?.channelId, channel?.podId]);

  const refresh = useCallback(() => {
    if (!channel?.podId || !channel?.channelId) return Promise.resolve(false);
    if (refreshInFlightRef.current) {
      refreshPendingRef.current = true;
      return refreshInFlightRef.current;
    }

    const requestId = ++refreshRequestIdRef.current;
    const since = historyCursorRef.current;
    const controller = new AbortController();
    refreshAbortControllerRef.current = controller;
    let request;

    request = (async () => {
      try {
        const [messagesResult, membersResult] = await Promise.allSettled([
          pods.getMessages(
            channel.podId,
            channel.channelId,
            since,
            controller.signal,
          ),
          pods.getMembers(channel.podId, controller.signal),
        ]);

        if (
          !mountedRef.current ||
          requestId !== refreshRequestIdRef.current
        ) {
          return false;
        }

        if (messagesResult.status === 'rejected') {
          if (isAbortError(messagesResult.reason, controller.signal)) {
            return false;
          }
          setMessagesLoadError(
            toDisplayError(
              messagesResult.reason,
              'Failed to load pod channel messages',
            ),
          );
          throw messagesResult.reason;
        }

        const incomingMessages = asRecords(messagesResult.value);
        setMessages((previousMessages) =>
          mergeMessageHistory(
            since === null ? [] : previousMessages,
            incomingMessages,
            MAX_POD_MESSAGES,
          ).map((message) => ({ ...message })),
        );
        const incomingCursor = historyCursor(incomingMessages);
        if (incomingCursor !== null) {
          historyCursorRef.current = Math.max(
            since ?? incomingCursor,
            incomingCursor,
          );
        }
        setMessagesLoadError('');
        if (membersResult.status === 'fulfilled') {
          setMembers(asRecords(membersResult.value));
          setError('');
        } else if (!isAbortError(membersResult.reason, controller.signal)) {
          setError(
            toDisplayError(membersResult.reason, 'Failed to load pod members'),
          );
        }
        return true;
      } finally {
        if (refreshInFlightRef.current === request) {
          refreshInFlightRef.current = null;
          if (refreshAbortControllerRef.current === controller) {
            refreshAbortControllerRef.current = null;
          }
          if (refreshPendingRef.current && mountedRef.current) {
            refreshPendingRef.current = false;
            Promise.resolve().then(() => {
              if (mountedRef.current) void refreshRef.current?.();
            });
          }
        }
      }
    })();

    refreshInFlightRef.current = request;
    return request;
  }, [channel?.channelId, channel?.podId]);

  refreshRef.current = refresh;

  usePolling(
    () =>
      refresh().catch((error) => {
        console.error('Failed to load pod channel messages:', error);
        if (mountedRef.current) {
          setError(
            toDisplayError(error, 'Failed to load pod channel messages'),
          );
        }
      }),
    2_000,
    {
      enabled: Boolean(channel?.podId && channel?.channelId),
      resetKey: `${channel?.podId || ''}:${channel?.channelId || ''}`,
    },
  );

  const send = async () => {
    const trimmed = body.trim();
    if (
      !trimmed ||
      !channel?.podId ||
      !channel?.channelId ||
      sendInFlightRef.current ||
      !mountedRef.current
    ) return;

    sendInFlightRef.current = true;
    const requestId = ++sendRequestIdRef.current;
    setSending(true);
    setError('');
    try {
      await pods.sendMessage(
        channel.podId,
        channel.channelId,
        trimmed,
        state?.user?.username || 'local-peer',
      );
      if (
        !mountedRef.current ||
        requestId !== sendRequestIdRef.current
      ) {
        return;
      }
      setBody('');
      await refresh();
    } catch (sendError) {
      if (
        mountedRef.current &&
        requestId === sendRequestIdRef.current
      ) {
        setError(toDisplayError(sendError, 'Failed to send pod message'));
      }
    } finally {
      sendInFlightRef.current = false;
      if (
        mountedRef.current &&
        requestId === sendRequestIdRef.current
      ) {
        setSending(false);
      }
    }
  };

  return (
    <div className="pod-message-session">
      {!isPodDirectChannel(channel) && (
        <PodListenAlongPanel
          channelId={channel.channelId}
          compact
          podId={channel.podId}
          user={state?.user?.username}
        />
      )}
      <div className="pod-message-session-main">
        {error ? <Message negative>{error}</Message> : null}
        <Segment.Group>
          <Segment className="pod-message-session-history">
            {messagesLoadError ? null : messages.length === 0 ? (
              <PlaceholderSegment
                caption="No messages yet"
                icon="comments"
              />
            ) : (
              <List>
                {messages.map((message, index) => (
                  <List.Content
                    className={`room-message ${message.senderPeerId === state?.user?.username ? 'room-message-self' : ''}`}
                    key={messageKey(message)}
                  >
                    <span className="room-message-time">
                      {message.timestampUnixMs
                        ? new Date(message.timestampUnixMs).toLocaleTimeString()
                        : ''}
                    </span>
                    <span className="room-message-name">
                      {String(message.senderPeerId ?? message.username ?? 'Unknown peer')}:{' '}
                    </span>
                    <span className="room-message-message">
                      {String(message.body ?? '')}
                    </span>
                  </List.Content>
                ))}
              </List>
            )}
          </Segment>
          <Segment className="pod-message-session-composer">
            <div className="messaging-start-row">
              <Input
                aria-label={`Message ${channelLabel(channel)}`}
                className="pod-message-session-input"
                fluid
                onChange={(event) => setBody(event.target.value)}
                onKeyUp={(event) => {
                  if (event.key === 'Enter') {
                    send().catch((error) => {
                      console.error('Failed to send pod message:', error);
                    });
                  }
                }}
                placeholder={`Message ${channel.channelName || channel.channelId}`}
                value={body}
              />
              <Popup
                content="Send this message to the pod channel."
                trigger={
                  <Button
                    aria-label={`Send message to ${channelLabel(channel)}`}
                    disabled={!body.trim() || sending}
                    icon="send"
                    loading={sending}
                    onClick={() =>
                      send().catch((error) => {
                        console.error('Failed to send pod message:', error);
                      })
                    }
                    primary
                    title={`Send message to ${channelLabel(channel)}`}
                  />
                }
              />
            </div>
          </Segment>
        </Segment.Group>
        <Segment className="room-users pod-message-users">
          <div className="room-users-header">
            <Icon name="users" />
            Members ({members.length})
          </div>
          <List
            divided
            relaxed
          >
            {members.map((member) => {
              const username = String(
                member.peerId || member.username || member.PeerId || 'Unknown peer',
              );

              return (
                <List.Item key={username}>
                  <List.Content>
                    <List.Header>
                      <UserCard username={username}>{username}</UserCard>
                    </List.Header>
                    <List.Description>
                      {String(member.role || member.Role || 'Member')}
                    </List.Description>
                  </List.Content>
                </List.Item>
              );
            })}
          </List>
        </Segment>
      </div>
    </div>
  );
};

export default PodChannelSession;
