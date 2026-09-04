import { createListeningPartyHubConnection } from '../../lib/hubFactory';
import { toDisplayError } from '../../lib/errors';
import * as listeningParty from '../../lib/listeningParty';
import { usePlayer } from './PlayerContext';
import { usePolling } from '../../lib/usePolling';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import { Button, Checkbox, Icon, Label, List, Popup, Segment } from 'semantic-ui-react';

const applyPartyState = (state, player) => {
  if (!state || typeof state !== 'object' || Array.isArray(state)) return;

  if (state.action === 'play' || state.action === 'seek') {
    const elapsed =
      state.action === 'play'
        ? Math.max(0, (Date.now() - state.serverTimeUnixMs) / 1000)
        : 0;
    player.playItem(
      {
        album: state.album,
        artist: state.artist || state.hostPeerId,
        contentId: state.contentId,
        streamUrl: state.streamUrl,
        title: state.title || state.contentId,
      },
      {
        positionSeconds: (state.positionSeconds || 0) + elapsed,
        replaceQueue: true,
      },
    );
  } else if (state.action === 'pause') {
    player.pause();
  } else if (state.action === 'stop') {
    player.clear();
  }
};

const PodListenAlongPanel = ({ channelId, compact = false, podId, user }) => {
  const player = usePlayer();
  const [connected, setConnected] = useState(false);
  const [directory, setDirectory] = useState([]);
  const [following, setFollowing] = useState(false);
  const [globalRadio, setGlobalRadio] = useState(false);
  const [meshStreaming, setMeshStreaming] = useState(false);
  const [partyState, setPartyState] = useState(null);
  const [publishing, setPublishing] = useState(false);
  const followingRef = useRef(false);
  const playerRef = useRef(player);
  const mountedRef = useRef(false);
  const directoryRequestIdRef = useRef(0);
  const operationRequestIdRef = useRef(0);
  const publishInFlightRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      directoryRequestIdRef.current += 1;
      operationRequestIdRef.current += 1;
    };
  }, []);

  useEffect(() => {
    followingRef.current = following;
  }, [following]);

  useEffect(() => {
    playerRef.current = player;
  }, [player]);

  useEffect(() => {
    if (!podId || !channelId) return undefined;

    let disposed = false;
    const hub = createListeningPartyHubConnection();

    hub.on('partyState', (state) => {
      if (disposed || !mountedRef.current) return;
      const nextState =
        state && typeof state === 'object' && !Array.isArray(state)
          ? state
          : null;
      setPartyState(nextState);
      if (followingRef.current) {
        playerRef.current.followParty(nextState);
        applyPartyState(nextState, playerRef.current);
      }
    });
    hub.onreconnecting(() => {
      if (!disposed && mountedRef.current) setConnected(false);
    });
    hub.onreconnected(() => {
      if (!disposed && mountedRef.current) setConnected(true);
    });
    hub.onclose(() => {
      if (!disposed && mountedRef.current) setConnected(false);
    });

    hub
      .start()
      .then(() => {
        if (disposed || !mountedRef.current) return undefined;
        return hub.invoke('JoinParty', podId, channelId);
      })
      .then(() => {
        if (!disposed && mountedRef.current) setConnected(true);
      })
      .catch(() => {
        if (!disposed && mountedRef.current) setConnected(false);
      });

    listeningParty
      .getPartyState(podId, channelId)
      .then((state) => {
        if (!disposed && mountedRef.current) {
          setPartyState(
            state && typeof state === 'object' && !Array.isArray(state)
              ? state
              : null,
          );
        }
      })
      .catch(() => {});

    return () => {
      disposed = true;
      operationRequestIdRef.current += 1;
      publishInFlightRef.current = false;
      if (mountedRef.current) setPublishing(false);
      hub.invoke('LeaveParty', podId, channelId).catch(() => {});
      hub.stop().catch(() => {});
    };
  }, [channelId, podId]);

  const refreshDirectory = useCallback(async () => {
    if (!mountedRef.current) return;
    const requestId = ++directoryRequestIdRef.current;
    try {
      const nextDirectory = await listeningParty.getPartyDirectory();
      if (
        mountedRef.current &&
        requestId === directoryRequestIdRef.current
      ) {
        setDirectory(
          Array.isArray(nextDirectory)
            ? nextDirectory.filter(
                (party) =>
                  party && typeof party === 'object' && !Array.isArray(party),
              )
            : [],
        );
      }
    } catch {
      if (
        mountedRef.current &&
        requestId === directoryRequestIdRef.current
      ) {
        setDirectory([]);
      }
    }
  }, []);

  usePolling(refreshDirectory, 30_000);

  const publish = async (action) => {
    const current = player.current;
    if (
      !mountedRef.current ||
      publishInFlightRef.current ||
      (action !== 'stop' && !current?.contentId)
    ) {
      return;
    }

    const requestId = ++operationRequestIdRef.current;
    publishInFlightRef.current = true;
    setPublishing(true);
    try {
      const state = await listeningParty.publishPartyState(podId, channelId, {
        action,
        album: current?.album || '',
        allowMeshStreaming: meshStreaming,
        artist: current?.artist || user || '',
        contentId: current?.contentId || '',
        hostPeerId: user || 'local-peer',
        listed: globalRadio,
        partyId: partyState?.partyId || '',
        positionSeconds: current?.positionSeconds || 0,
        title: current?.title || current?.fileName || '',
      });
      if (
        !mountedRef.current ||
        requestId !== operationRequestIdRef.current
      ) {
        return;
      }
      setPartyState(
        state && typeof state === 'object' && !Array.isArray(state)
          ? state
          : null,
      );
      await refreshDirectory();
    } catch (error) {
      console.error('[Listen Along] Failed to publish party state:', error);
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        toast.error(toDisplayError(error, 'Broadcast failed'));
      }
    } finally {
      publishInFlightRef.current = false;
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        setPublishing(false);
      }
    }
  };

  const joinListedParty = (party) => {
    if (!mountedRef.current || !party?.contentId) return;
    try {
      const streamUrl = listeningParty.buildRadioStreamUrl(party);
      player.followParty(party);
      player.playItem(
        {
          album: party.album,
          artist: party.artist || party.hostPeerId,
          contentId: party.contentId,
          streamUrl,
          title: party.title || party.contentId,
        },
        {
          replaceQueue: true,
          streamUrl,
        },
      );
    } catch (error) {
      console.error('[Listen Along] Failed to join listed party:', error);
      toast.error(toDisplayError(error, 'Unable to join listed party'));
    }
  };

  if (compact) {
    return (
      <Segment className="pod-listen-along pod-listen-along-compact">
        <div className="pod-listen-along-compact-status">
          <Popup
            content={
              partyState
                ? `${partyState.hostPeerId} ${partyState.action}: ${partyState.title || partyState.contentId}`
                : 'No active room broadcast'
            }
            trigger={
              <span
                aria-label={connected ? 'Listen Along live' : 'Listen Along offline'}
                className={`pod-listen-along-orb ${connected ? 'pod-listen-along-orb-live' : ''}`}
                role="status"
                title={connected ? 'Listen Along live' : 'Listen Along offline'}
              />
            }
          />
          <span className="pod-listen-along-compact-copy">
            {partyState ? partyState.title || partyState.contentId : 'Room broadcast'}
          </span>
        </div>
        <div className="pod-listen-along-compact-actions">
          <Popup
            content="Follow this room's broadcast using your own stream access."
            trigger={
              <Button
                active={following}
                aria-label="Follow room broadcast"
                icon
                onClick={() => {
                  const next = !following;
                  setFollowing(next);
                  if (next && partyState) {
                    player.followParty(partyState);
                    applyPartyState(partyState, player);
                  } else {
                    player.followParty(null);
                  }
                }}
                size="mini"
                title="Follow room broadcast"
              >
                <Icon name={following ? 'volume up' : 'volume off'} />
              </Button>
            }
          />
          <Popup
            content="Broadcast your current local player track to this room."
            trigger={
              <Button
                aria-label="Broadcast current track to room"
                disabled={publishing || !player.current}
                icon
                onClick={() => publish('play')}
                size="mini"
                title="Broadcast current track to room"
              >
                <Icon name="bullhorn" />
              </Button>
            }
          />
          <Popup
            content="List this room broadcast in the mesh radio directory."
            trigger={
              <Button
                active={globalRadio}
                aria-label="List room broadcast in mesh directory"
                icon
                onClick={() => setGlobalRadio((value) => !value)}
                size="mini"
                title="List room broadcast in mesh directory"
              >
                <Icon name="broadcast tower" />
              </Button>
            }
          />
          <Popup
            content="Allow directory listeners to stream the current track from this node."
            trigger={
              <Button
                active={meshStreaming}
                aria-label="Allow mesh streaming for broadcast"
                disabled={!globalRadio}
                icon
                onClick={() => setMeshStreaming((value) => !value)}
                size="mini"
                title="Allow mesh streaming for broadcast"
              >
                <Icon name="wifi" />
              </Button>
            }
          />
          <Popup
            content="Stop broadcasting listen-along metadata for this room."
            trigger={
              <Button
                aria-label="Stop room broadcast"
                disabled={publishing}
                icon
                onClick={() => publish('stop')}
                size="mini"
                title="Stop room broadcast"
              >
                <Icon name="stop" />
              </Button>
            }
          />
        </div>
      </Segment>
    );
  }

  return (
    <Segment className="pod-listen-along">
      <div className="pod-listen-along-main">
        <div>
          <strong>Listen Along</strong>
          <div className="pod-listen-along-state">
            {partyState
              ? `${partyState.hostPeerId} ${partyState.action}: ${partyState.title || partyState.contentId}`
              : 'No active party'}
          </div>
        </div>
        <Label color={connected ? 'green' : 'grey'}>
          {connected ? 'Live' : 'Offline'}
        </Label>
      </div>
      <div className="pod-listen-along-toggles">
        <Popup
          content="List this party in the slskr mesh radio directory so nearby mesh members can discover it."
          trigger={
            <Checkbox
              checked={globalRadio}
              label="List globally"
              onChange={(event, data) => setGlobalRadio(data.checked)}
              toggle
            />
          }
        />
        <Popup
          content="Allow listeners who join from the directory to stream this party's current track from this slskr node."
          trigger={
            <Checkbox
              checked={meshStreaming}
              disabled={!globalRadio}
              label="Mesh streaming"
              onChange={(event, data) => setMeshStreaming(data.checked)}
              toggle
            />
          }
        />
      </div>
      <Button.Group size="small">
        <Popup
          content="Follow this pod's host playback using your own stream access."
          trigger={
            <Button
              active={following}
              icon
              onClick={() => {
                const next = !following;
                setFollowing(next);
                if (next && partyState) {
                  player.followParty(partyState);
                  applyPartyState(partyState, player);
                } else {
                  player.followParty(null);
                }
              }}
            >
              <Icon name={following ? 'volume up' : 'volume off'} />
            </Button>
          }
        />
        <Popup
          content="Publish your current local player track as the pod listen-along host."
          trigger={
            <Button
              disabled={publishing || !player.current}
              icon
              onClick={() => publish('play')}
            >
              <Icon name="bullhorn" />
            </Button>
          }
        />
        <Popup
          content="Stop hosting listen-along metadata for this pod."
          trigger={
            <Button
              disabled={publishing}
              icon
              onClick={() => publish('stop')}
            >
              <Icon name="stop" />
            </Button>
          }
        />
      </Button.Group>
      {directory.length > 0 && (
        <div className="pod-listen-along-directory">
          <strong>Listed radio</strong>
          <List divided relaxed>
            {directory.slice(0, 6).map((party) => (
              <List.Item key={party.partyId}>
                <List.Content floated="right">
                  <Popup
                    content="Join this listed radio party and stream from the host's integrated slskr endpoint when available."
                    trigger={
                      <Button
                        disabled={!party.allowMeshStreaming}
                        icon
                        onClick={() => joinListedParty(party)}
                        size="mini"
                      >
                        <Icon name="play" />
                      </Button>
                    }
                  />
                </List.Content>
                <List.Content>
                  <List.Header>{party.title || party.contentId}</List.Header>
                  <List.Description>
                    {party.hostPeerId}
                    {party.allowMeshStreaming ? ' | streamable' : ' | metadata only'}
                  </List.Description>
                </List.Content>
              </List.Item>
            ))}
          </List>
        </div>
      )}
    </Segment>
  );
};

export default PodListenAlongPanel;
