// <copyright file="ArtistReleaseRadarPanel.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import {
  fetchArtistReleaseRadarNotifications,
  fetchArtistReleaseRadarSubscriptions,
  routeArtistReleaseRadarNotification,
  subscribeArtistReleaseRadar,
} from '../../lib/musicBrainz';
import { toDisplayError } from '../../lib/errors';
import { useMountedRef } from '../../lib/useMountedRef';
import React, { useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import {
  Button,
  Checkbox,
  Form,
  Header,
  Icon,
  Label,
  List,
  Message,
  Popup,
  Segment,
} from 'semantic-ui-react';

const getWorkTitle = (notification = {}) =>
  notification.workRef?.title ||
  notification.workRef?.Title ||
  notification.recordingId ||
  'Unresolved radar hit';

const getWorkArtist = (notification = {}) =>
  notification.workRef?.artist ||
  notification.workRef?.Artist ||
  notification.workRef?.creator ||
  notification.workRef?.Creator ||
  '';

const parseList = (value = '') =>
  String(value ?? '')
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (item) => item && typeof item === 'object' && !Array.isArray(item),
  );

const ArtistReleaseRadarPanel = ({ disabled }) => {
  const [artistId, setArtistId] = useState('');
  const [artistName, setArtistName] = useState('');
  const [enabled, setEnabled] = useState(true);
  const [error, setError] = useState(null);
  const [mutedReleaseGroups, setMutedReleaseGroups] = useState('');
  const [notifications, setNotifications] = useState([]);
  const [routeTargets, setRouteTargets] = useState('');
  const [scope, setScope] = useState('trusted');
  const [status, setStatus] = useState('');
  const [subscriptions, setSubscriptions] = useState([]);
  const [unreadOnly, setUnreadOnly] = useState(false);
  const [working, setWorking] = useState(false);
  const mountedRef = useMountedRef();
  const loadRequestIdRef = useRef(0);
  const operationRequestIdRef = useRef(0);
  const operationInFlightRef = useRef(false);

  const loadRadar = async () => {
    const requestId = ++loadRequestIdRef.current;
    if (disabled) return;

    if (!mountedRef.current) return;
    setError(null);
    try {
      const [subscriptionsResponse, notificationsResponse] = await Promise.all([
        fetchArtistReleaseRadarSubscriptions(),
        fetchArtistReleaseRadarNotifications({ unreadOnly }),
      ]);
      if (
        !mountedRef.current ||
        requestId !== loadRequestIdRef.current
      ) {
        return;
      }
      setSubscriptions(
        asRecords(subscriptionsResponse.data),
      );
      setNotifications(
        asRecords(notificationsResponse.data),
      );
    } catch (loadError) {
      if (
        mountedRef.current &&
        requestId === loadRequestIdRef.current
      ) {
        setError(toDisplayError(loadError, 'Unable to load release radar.'));
      }
    }
  };

  useEffect(() => {
    void loadRadar();
  }, [disabled, unreadOnly]);

  const subscribe = async () => {
    if (
      !mountedRef.current ||
      disabled ||
      operationInFlightRef.current
    ) {
      return;
    }
    if (!artistId.trim()) {
      toast.error('Artist MBID is required');
      return;
    }

    const requestId = ++operationRequestIdRef.current;
    operationInFlightRef.current = true;
    setWorking(true);
    setError(null);
    try {
      await subscribeArtistReleaseRadar({
        artistId: artistId.trim(),
        artistName: artistName.trim(),
        enabled,
        mutedReleaseGroupIds: parseList(mutedReleaseGroups),
        scope,
      });
      if (
        !mountedRef.current ||
        requestId !== operationRequestIdRef.current
      ) {
        return;
      }
      setStatus(`Watching ${artistName.trim() || artistId.trim()}.`);
      setArtistId('');
      setArtistName('');
      await loadRadar();
    } catch (subscribeError) {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        setError(
          toDisplayError(
            subscribeError,
            'Unable to save release radar subscription.',
          ),
        );
      }
    } finally {
      operationInFlightRef.current = false;
      if (mountedRef.current && requestId === operationRequestIdRef.current) {
        setWorking(false);
      }
    }
  };

  const routeNotification = async (notification) => {
    if (
      !notification?.id ||
      !mountedRef.current ||
      disabled ||
      operationInFlightRef.current
    ) {
      return;
    }
    const requestId = ++operationRequestIdRef.current;
    operationInFlightRef.current = true;
    setWorking(true);
    setError(null);
    try {
      const response = await routeArtistReleaseRadarNotification({
        notificationId: notification.id,
        targetPeerIds: parseList(routeTargets),
      });
      if (
        !mountedRef.current ||
        requestId !== operationRequestIdRef.current
      ) {
        return;
      }
      setStatus(
        `Route ${response.data?.success ? 'sent' : 'recorded'} for ${getWorkTitle(notification)}.`,
      );
    } catch (routeError) {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        setError(
          toDisplayError(routeError, 'Unable to route release radar notification.'),
        );
      }
    } finally {
      operationInFlightRef.current = false;
      if (mountedRef.current && requestId === operationRequestIdRef.current) {
        setWorking(false);
      }
    }
  };

  if (disabled) {
    return (
      <Segment raised>
        <Header as="h4">Artist Release Radar</Header>
        <p>Connect to the server to review artist radar subscriptions.</p>
      </Segment>
    );
  }

  return (
    <Segment raised>
      <Header as="h4">Artist Release Radar</Header>
      <Form>
        <Form.Group widths="equal">
          <Form.Input
            aria-label="Release radar artist MBID"
            label="Artist MBID"
            onChange={(event) => setArtistId(event.target.value)}
            value={artistId}
          />
          <Form.Input
            aria-label="Release radar artist name"
            label="Artist name"
            onChange={(event) => setArtistName(event.target.value)}
            value={artistName}
          />
          <Form.Input
            aria-label="Muted release group IDs"
            label="Muted release groups"
            onChange={(event) => setMutedReleaseGroups(event.target.value)}
            placeholder="comma-separated MBIDs"
            value={mutedReleaseGroups}
          />
        </Form.Group>
        <Form.Group widths="equal">
          <Form.Select
            aria-label="Release radar scope"
            label="Scope"
            onChange={(_event, data) => setScope(data.value)}
            options={[
              { key: 'trusted', text: 'Trusted', value: 'trusted' },
              { key: 'realm', text: 'Realm', value: 'realm' },
              { key: 'local', text: 'Local', value: 'local' },
            ]}
            value={scope}
          />
          <Form.Field>
            <label>Enabled</label>
            <Checkbox
              aria-label="Enable release radar subscription"
              checked={enabled}
              onChange={(_event, data) => setEnabled(data.checked)}
              toggle
            />
          </Form.Field>
          <Form.Field>
            <label>Unread filter</label>
            <Checkbox
              aria-label="Show unread release radar notifications only"
              checked={unreadOnly}
              onChange={(_event, data) => setUnreadOnly(data.checked)}
              toggle
            />
          </Form.Field>
        </Form.Group>
        <Popup
          content="Save this artist radar subscription. It observes already-ingested trusted radar evidence and does not poll MusicBrainz, search Soulseek, browse peers, or download."
          position="top center"
          trigger={
            <Button
              disabled={working || !artistId.trim()}
              loading={working}
              onClick={subscribe}
              primary
              type="button"
            >
              <Icon name="rss" />
              Watch Artist
            </Button>
          }
        />
        <Popup
          content="Reload release radar subscriptions and notifications from the local backend."
          position="top center"
          trigger={
            <Button
              disabled={working}
              loading={working}
              onClick={loadRadar}
              type="button"
            >
              <Icon name="refresh" />
              Refresh Radar
            </Button>
          }
        />
      </Form>
      {status && <Message compact size="mini">{status}</Message>}
      {error && <Message compact error size="mini">{String(error)}</Message>}
      <div className="search-acquisition-profile-strip">
        <Label basic>
          Subscriptions
          <Label.Detail>{subscriptions.length}</Label.Detail>
        </Label>
        <Label basic>
          Notifications
          <Label.Detail>{notifications.length}</Label.Detail>
        </Label>
      </div>
      <List divided relaxed>
        {subscriptions.map((subscription) => (
          <List.Item key={subscription.id || subscription.artistId}>
            <List.Icon name={subscription.enabled ? 'rss' : 'pause circle'} />
            <List.Content>
              <List.Header>{subscription.artistName || subscription.artistId}</List.Header>
              <List.Description>
                {subscription.scope} scope · muted {(subscription.mutedReleaseGroupIds || []).length}
              </List.Description>
            </List.Content>
          </List.Item>
        ))}
      </List>
      <Form>
        <Form.Input
          aria-label="Release radar route target peers"
          label="Route target peers"
          onChange={(event) => setRouteTargets(event.target.value)}
          placeholder="optional comma-separated peer IDs"
          value={routeTargets}
        />
      </Form>
      <List divided relaxed>
        {notifications.map((notification) => {
          const title = getWorkTitle(notification);
          const artist = getWorkArtist(notification);

          return (
            <List.Item key={notification.id}>
              <List.Icon name={notification.read ? 'check circle' : 'bell'} />
              <List.Content>
                <List.Header>{[artist, title].filter(Boolean).join(' - ') || title}</List.Header>
                <List.Description>
                  {notification.sourceRealm || 'unknown realm'} · {Math.round((notification.confidence || 0) * 100)}%
                </List.Description>
                <Popup
                  content="Route this radar notification to explicitly listed trusted peers through the backend route service."
                  position="top center"
                  trigger={
                    <Button
                      aria-label={`Route ${title} radar hit`}
                      disabled={working}
                      loading={working}
                      onClick={() => routeNotification(notification)}
                      size="mini"
                      type="button"
                    >
                      <Icon name="send" />
                      Route
                    </Button>
                  }
                />
              </List.Content>
            </List.Item>
          );
        })}
      </List>
    </Segment>
  );
};

export default ArtistReleaseRadarPanel;
