// <copyright file="SoulseekDiscoveryPanel.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import * as soulseekDiscovery from '../../lib/soulseekDiscovery';
import { toDisplayError } from '../../lib/errors';
import * as wishlist from '../../lib/wishlist';
import { useMountedRef } from '../../lib/useMountedRef';
import React, { useMemo, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import {
  Button,
  Form,
  Header,
  Icon,
  Label,
  List,
  Message,
  Popup,
  Segment,
} from 'semantic-ui-react';

const getValue = (value, camel, pascal, fallback = undefined) =>
  value?.[camel] ?? value?.[pascal] ?? fallback;

const normalizeRecommendation = (recommendation) => ({
  item: typeof getValue(recommendation, 'item', 'Item', '') === 'string'
    ? getValue(recommendation, 'item', 'Item', '').trim()
    : '',
  score: typeof getValue(recommendation, 'score', 'Score', null) === 'number'
    ? getValue(recommendation, 'score', 'Score', null)
    : null,
});

const normalizeUser = (user) => {
  if (typeof user === 'string') {
    return { rating: null, username: user };
  }

  return {
    rating: getValue(user, 'rating', 'Rating', null),
    username: typeof getValue(user, 'username', 'Username', '') === 'string'
      ? getValue(user, 'username', 'Username', '').trim()
      : '',
  };
};

const getRecommendations = (payload) =>
  (Array.isArray(getValue(payload, 'recommendations', 'Recommendations', []))
    ? getValue(payload, 'recommendations', 'Recommendations', [])
    : [])
    .map(normalizeRecommendation)
    .filter((recommendation) => recommendation.item);

const getUnrecommendations = (payload) =>
  (Array.isArray(getValue(payload, 'unrecommendations', 'Unrecommendations', []))
    ? getValue(payload, 'unrecommendations', 'Unrecommendations', [])
    : [])
    .map(normalizeRecommendation)
    .filter((recommendation) => recommendation.item);

const getUsernames = (payload) =>
  (Array.isArray(getValue(payload, 'usernames', 'Usernames', []))
    ? getValue(payload, 'usernames', 'Usernames', [])
    : [])
    .map(normalizeUser)
    .filter((user) => user.username);

const getStringList = (value) =>
  Array.isArray(value)
    ? value
      .filter((item) => typeof item === 'string')
      .map((item) => item.trim())
      .filter(Boolean)
    : [];

const errorMessage = (error, fallback) =>
  toDisplayError(error, fallback);

const SoulseekDiscoveryPanel = ({ disabled, onSearch }) => {
  const [error, setError] = useState('');
  const [interest, setInterest] = useState('');
  const [item, setItem] = useState('');
  const [loading, setLoading] = useState(false);
  const [recommendations, setRecommendations] = useState([]);
  const [similarUsers, setSimilarUsers] = useState([]);
  const [status, setStatus] = useState('');
  const [title, setTitle] = useState('');
  const [unrecommendations, setUnrecommendations] = useState([]);
  const [userInterests, setUserInterests] = useState(null);
  const [username, setUsername] = useState('');
  const mountedRef = useMountedRef();
  const requestIdRef = useRef(0);
  const actionRequestIdRef = useRef(0);
  const requestInFlightRef = useRef(false);
  const wishlistInFlightRef = useRef(false);
  const [wishlistLoading, setWishlistLoading] = useState(false);

  const hasResults = useMemo(
    () =>
      recommendations.length > 0 ||
      unrecommendations.length > 0 ||
      similarUsers.length > 0 ||
      userInterests,
    [recommendations.length, similarUsers.length, unrecommendations.length, userInterests],
  );

  const clearResults = () => {
    setRecommendations([]);
    setSimilarUsers([]);
    setUnrecommendations([]);
    setUserInterests(null);
  };

  const run = async (label, action) => {
    if (!mountedRef.current || disabled || requestInFlightRef.current) return;
    const requestId = ++requestIdRef.current;
    requestInFlightRef.current = true;
    setError('');
    setLoading(true);
    try {
      await action(() =>
        mountedRef.current && requestId === requestIdRef.current,
      );
    } catch (actionError) {
      if (
        mountedRef.current &&
        requestId === requestIdRef.current
      ) {
        setError(errorMessage(actionError, `Unable to ${label}.`));
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === requestIdRef.current
      ) {
        setLoading(false);
      }
      requestInFlightRef.current = false;
    }
  };

  const loadRecommendations = (global = false) =>
    run(global ? 'load global recommendations' : 'load recommendations', async (isCurrent) => {
      const response = global
        ? await soulseekDiscovery.getGlobalRecommendations()
        : await soulseekDiscovery.getRecommendations();
      const payload = response.data || {};

      if (!isCurrent()) return;
      clearResults();
      setRecommendations(getRecommendations(payload));
      setUnrecommendations(getUnrecommendations(payload));
      setTitle(global ? 'Global recommendations' : 'My recommendations');
      setStatus(
        `Loaded ${getRecommendations(payload).length} recommendation${
          getRecommendations(payload).length === 1 ? '' : 's'
        }.`,
      );
    });

  const loadItemRecommendations = () => {
    const trimmed = item.trim();
    if (!trimmed) {
      toast.error('Item is required');
      return;
    }

    run('load item recommendations', async (isCurrent) => {
      const response = await soulseekDiscovery.getItemRecommendations({
        item: trimmed,
      });

      if (!isCurrent()) return;
      clearResults();
      setRecommendations(getRecommendations(response.data || {}));
      setTitle(`Recommendations for ${trimmed}`);
      setStatus(`Loaded related recommendation seeds for ${trimmed}.`);
    });
  };

  const loadSimilarUsers = () =>
    run('load similar users', async (isCurrent) => {
      const response = await soulseekDiscovery.getSimilarUsers();

      if (!isCurrent()) return;
      clearResults();
      const users = Array.isArray(response.data) ? response.data : [];
      setSimilarUsers(users.map(normalizeUser).filter((user) => user.username));
      setTitle('Similar users');
      setStatus(`Loaded ${users.length} similar user${users.length === 1 ? '' : 's'}.`);
    });

  const loadItemSimilarUsers = () => {
    const trimmed = item.trim();
    if (!trimmed) {
      toast.error('Item is required');
      return;
    }

    run('load item similar users', async (isCurrent) => {
      const response = await soulseekDiscovery.getItemSimilarUsers({
        item: trimmed,
      });

      if (!isCurrent()) return;
      clearResults();
      setSimilarUsers(getUsernames(response.data || {}));
      setTitle(`Users similar to ${trimmed}`);
      setStatus(`Loaded users associated with ${trimmed}.`);
    });
  };

  const loadUserInterests = (target = username) => {
    const trimmed = `${target || ''}`.trim();
    if (!trimmed) {
      toast.error('Username is required');
      return;
    }

    run('load user interests', async (isCurrent) => {
      const response = await soulseekDiscovery.getUserInterests({
        username: trimmed,
      });

      if (!isCurrent()) return;
      clearResults();
      setUserInterests(response.data || {});
      setTitle(`${trimmed} interests`);
      setStatus(`Loaded native interests for ${trimmed}.`);
    });
  };

  const updateInterest = (hated = false, remove = false) => {
    const trimmed = interest.trim();
    if (!trimmed) {
      toast.error('Interest is required');
      return;
    }

    const action = hated
      ? remove
        ? soulseekDiscovery.removeHatedInterest
        : soulseekDiscovery.addHatedInterest
      : remove
        ? soulseekDiscovery.removeInterest
        : soulseekDiscovery.addInterest;

    run('update interests', async (isCurrent) => {
      await action({ item: trimmed });
      if (!isCurrent()) return;
      setStatus(`${remove ? 'Removed' : 'Added'} ${trimmed} ${hated ? 'as a hated interest' : 'as an interest'}.`);
      setInterest('');
    });
  };

  const addToWishlist = async (searchText) => {
    const normalizedSearchText = typeof searchText === 'string' ? searchText.trim() : '';
    if (
      !normalizedSearchText ||
      !mountedRef.current ||
      disabled ||
      wishlistInFlightRef.current
    ) return;
    const requestId = ++actionRequestIdRef.current;
    wishlistInFlightRef.current = true;
    setWishlistLoading(true);
    try {
      await wishlist.create({
        autoDownload: false,
        enabled: false,
        filter: 'source:soulseek-native-discovery; review-only',
        maxResults: 25,
        searchText: normalizedSearchText,
      });
      if (
        mountedRef.current &&
        requestId === actionRequestIdRef.current
      ) {
        toast.success(`Added ${normalizedSearchText} to Wishlist for review`);
      }
    } catch (wishlistError) {
      if (
        mountedRef.current &&
        requestId === actionRequestIdRef.current
      ) {
        toast.error(errorMessage(wishlistError, 'Unable to add to Wishlist.'));
      }
    } finally {
      wishlistInFlightRef.current = false;
      if (mountedRef.current) setWishlistLoading(false);
    }
  };

  const renderRecommendation = (recommendation) => (
    <List.Item key={`${recommendation.item}-${recommendation.score ?? 'n'}`}>
      <List.Content floated="right">
        <Popup
          content="Start a normal Soulseek search for this recommendation."
          trigger={
            <Button
              aria-label={`Search ${recommendation.item}`}
              icon="search"
              onClick={() => onSearch?.(recommendation.item)}
              size="mini"
            />
          }
        />
        <Popup
          content="Save this raw recommendation to Wishlist for manual review."
          trigger={
            <Button
              aria-label={`Add ${recommendation.item} to Wishlist`}
              disabled={loading || wishlistLoading}
              icon="bookmark outline"
              onClick={() => addToWishlist(recommendation.item)}
              size="mini"
            />
          }
        />
      </List.Content>
      <List.Icon name="music" />
      <List.Content>
        <List.Header>{recommendation.item}</List.Header>
        {recommendation.score !== null && (
          <List.Description>
            <Label size="mini">score {recommendation.score}</Label>
          </List.Description>
        )}
      </List.Content>
    </List.Item>
  );

  const renderSimilarUser = (user) => (
    <List.Item key={user.username}>
      <List.Content floated="right">
        <Popup
          content="Load this user's native Soulseek interests."
          trigger={
            <Button
              aria-label={`Load ${user.username} interests`}
              disabled={loading}
              icon="heart outline"
              onClick={() => loadUserInterests(user.username)}
              size="mini"
            />
          }
        />
      </List.Content>
      <List.Icon name="user" />
      <List.Content>
        <List.Header>{user.username}</List.Header>
        {user.rating !== null && (
          <List.Description>
            <Label size="mini">rating {user.rating}</Label>
          </List.Description>
        )}
      </List.Content>
    </List.Item>
  );

  const renderInterestList = (label, values, color) => (
    <div style={{ marginBottom: '0.75em' }}>
      <Header
        as="h5"
        style={{ marginBottom: '0.35em' }}
      >
        {label}
      </Header>
      {values.length === 0 ? (
        <span style={{ opacity: 0.65 }}>None reported</span>
      ) : (
        values.map((value) => (
          <Label
            color={color}
            key={value}
            size="small"
          >
            {value}
          </Label>
        ))
      )}
    </div>
  );

  if (disabled) {
    return (
      <Segment raised>
        <Header as="h4">Soulseek Native Discovery</Header>
        <p>Connect to the server to use native Soulseek interests and recommendations.</p>
      </Segment>
    );
  }

  const liked = getStringList(getValue(userInterests, 'liked', 'Liked', []));
  const hated = getStringList(getValue(userInterests, 'hated', 'Hated', []));

  return (
    <Segment loading={loading}>
      <Header as="h4">
        <Icon name="compass outline" />
        <Header.Content>Soulseek Native Discovery</Header.Content>
      </Header>

      <Form>
        <Form.Group widths="equal">
          <Form.Input
            label="Interest"
            onChange={(event) => setInterest(event.target.value)}
            placeholder="genre, artist, scene, tag"
            value={interest}
          />
          <Form.Input
            label="Item"
            onChange={(event) => setItem(event.target.value)}
            placeholder="item to branch from"
            value={item}
          />
          <Form.Input
            label="User"
            onChange={(event) => setUsername(event.target.value)}
            placeholder="username"
            value={username}
          />
        </Form.Group>
        <Button.Group size="small">
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={() => updateInterest(false, false)}
            type="button"
          >
            <Icon name="heart" />
            Add Interest
          </Button>
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={() => updateInterest(true, false)}
            type="button"
          >
            <Icon name="ban" />
            Add Hated
          </Button>
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={() => updateInterest(false, true)}
            type="button"
          >
            <Icon name="minus circle" />
            Remove Interest
          </Button>
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={() => updateInterest(true, true)}
            type="button"
          >
            <Icon name="minus square outline" />
            Remove Hated
          </Button>
        </Button.Group>
        <Button.Group
          floated="right"
          size="small"
        >
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={() => loadRecommendations(false)}
            type="button"
          >
            <Icon name="lightbulb outline" />
            My Recs
          </Button>
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={() => loadRecommendations(true)}
            type="button"
          >
            <Icon name="globe" />
            Global
          </Button>
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={loadSimilarUsers}
            type="button"
          >
            <Icon name="users" />
            Similar Users
          </Button>
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={loadItemRecommendations}
            type="button"
          >
            <Icon name="sitemap" />
            Item Recs
          </Button>
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={loadItemSimilarUsers}
            type="button"
          >
            <Icon name="user plus" />
            Item Users
          </Button>
          <Button
            disabled={loading}
            icon
            labelPosition="left"
            onClick={() => loadUserInterests()}
            type="button"
          >
            <Icon name="address card outline" />
            User Interests
          </Button>
        </Button.Group>
      </Form>

      {error && (
        <Message
          content={error}
          error
          style={{ clear: 'both' }}
        />
      )}
      {status && (
        <Message
          content={status}
          positive={!error}
          style={{ clear: 'both' }}
        />
      )}

      {hasResults && (
        <Segment secondary>
          <Header as="h5">{title || 'Discovery results'}</Header>
          {recommendations.length > 0 && (
            <List
              divided
              relaxed
            >
              {recommendations.map(renderRecommendation)}
            </List>
          )}
          {unrecommendations.length > 0 && (
            <>
              <Header as="h5">Unrecommendations</Header>
              <List
                divided
                relaxed
              >
                {unrecommendations.map(renderRecommendation)}
              </List>
            </>
          )}
          {similarUsers.length > 0 && (
            <List
              divided
              relaxed
            >
              {similarUsers.map(renderSimilarUser)}
            </List>
          )}
          {userInterests && (
            <>
              {renderInterestList('Liked', liked, 'green')}
              {renderInterestList('Hated', hated, 'red')}
            </>
          )}
        </Segment>
      )}
    </Segment>
  );
};

export default SoulseekDiscoveryPanel;
