import './Users.css';
import { activeUserInfoKey } from '../../config';
import { toDisplayError } from '../../lib/errors';
import {
  getLocalStorageItem,
  removeLocalStorageItem,
  setLocalStorageItem,
} from '../../lib/storage';
import * as users from '../../lib/users';
import PlaceholderSegment from '../Shared/PlaceholderSegment';
import User from './User';
import React, { useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { Icon, Input, Item, Loader, Segment } from 'semantic-ui-react';

const toText = (value, fallback = '') => {
  if (typeof value === 'string' || typeof value === 'number') return String(value);
  return fallback;
};

const asRecord = (value) =>
  value && typeof value === 'object' && !Array.isArray(value) ? value : {};

const normalizeUser = (responses, username) => {
  const merged = responses.reduce(
    (result, response) => ({ ...result, ...asRecord(response?.data) }),
    {},
  );
  return {
    ...merged,
    address: toText(merged.address, 'Unknown'),
    description: toText(merged.description),
    hasPicture: merged.hasPicture === true && typeof merged.picture === 'string',
    picture: toText(merged.picture),
    port: toText(merged.port, 'Unknown'),
    presence: toText(merged.presence, 'Unknown'),
    queueLength: toText(merged.queueLength, 'Unknown'),
    uploadSlots: toText(merged.uploadSlots, 'Unknown'),
    username: toText(merged.username, username),
  };
};

const Users = () => {
  const location = useLocation();
  const inputRef = useRef();
  const [user, setUser] = useState();
  const [usernameInput, setUsernameInput] = useState('');
  const [selectedUsername, setSelectedUsername] = useState(undefined);
  // eslint-disable-next-line react/hook-use-state
  const [{ error, fetching }, setStatus] = useState({
    error: undefined,
    fetching: false,
  });

  const setInputText = (text) => {
    if (inputRef.current?.inputRef?.current) {
      inputRef.current.inputRef.current.value = text;
    }
  };

  const setInputFocus = () => {
    inputRef.current?.focus?.();
  };

  const clear = () => {
    removeLocalStorageItem(activeUserInfoKey);
    setSelectedUsername(undefined);
    setUsernameInput('');
    setUser(undefined);
    setInputText('');
    setInputFocus();
  };

  const keyUp = (event) => (event.key === 'Escape' ? clear() : '');

  useEffect(() => {
    document.addEventListener('keyup', keyUp, false);

    const storedUsername =
      location.state?.user || getLocalStorageItem(activeUserInfoKey);

    if (storedUsername !== undefined) {
      const normalizedUsername = toText(storedUsername).trim();
      if (normalizedUsername) {
        setSelectedUsername(normalizedUsername);
        setUsernameInput(normalizedUsername);
        setInputText(normalizedUsername);
      }
    }

    return () => document.removeEventListener('keyup', keyUp, false);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    let active = true;

    const fetchUser = async () => {
      if (!selectedUsername) {
        if (active) {
          setStatus({ error: undefined, fetching: false });
        }
        return;
      }

      if (active) {
        setStatus({ error: undefined, fetching: true });
      }

      try {
        const [info, status, endpoint] = await Promise.all([
          users.getInfo({ username: selectedUsername }),
          users.getStatus({ username: selectedUsername }),
          users.getEndpoint({ username: selectedUsername }),
        ]);

        if (!active) {
          return;
        }

        setLocalStorageItem(activeUserInfoKey, selectedUsername);
        setUser(normalizeUser([info, status, endpoint], selectedUsername));
        setStatus({ error: undefined, fetching: false });
      } catch (fetchError) {
        if (active) {
          setStatus({
            error: toDisplayError(fetchError, 'Failed to retrieve user information'),
            fetching: false,
          });
        }
      }
    };

    void fetchUser();

    return () => {
      active = false;
    };
  }, [selectedUsername]);

  const submitUsername = () => {
    const normalizedUsername =
      typeof usernameInput === 'string' ? usernameInput.trim() : '';
    if (normalizedUsername) setSelectedUsername(normalizedUsername);
  };

  return (
    <div className="users-container">
      <Segment
        className="users-segment"
        raised
      >
        <div className="users-segment-icon">
          <Icon
            name="users"
            size="big"
          />
        </div>
        <Input
          action={
            !fetching &&
            (user == null
              ? {
                  'aria-label': 'Search for user',
                  icon: 'search',
                  onClick: submitUsername,
                  title: 'Search for user',
                }
              : {
                  'aria-label': 'Clear selected user',
                  color: 'red',
                  icon: 'x',
                  onClick: clear,
                  title: 'Clear selected user',
                })
          }
          className="users-input"
          disabled={fetching}
          input={
            <input
              data-lpignore="true"
              disabled={Boolean(user) || fetching}
              placeholder="Username"
              type="search"
            />
          }
          loading={fetching}
          onChange={(event) => setUsernameInput(event.target.value)}
          onKeyUp={(event) =>
            event.key === 'Enter' ? submitUsername() : ''
          }
          placeholder="Username"
          ref={inputRef}
          size="big"
        />
      </Segment>
      {fetching ? (
        <Loader
          active
          className="search-loader"
          inline="centered"
          size="big"
        />
      ) : (
        <div>
          {error ? (
            <span>
              Failed to retrieve information for {selectedUsername}: {error}
            </span>
          ) : user == null ? (
            <PlaceholderSegment
              caption="No user info to display"
              icon="users"
            />
          ) : (
            <Segment
              className="users-user"
              raised
            >
              <Item.Group>
                <User {...user} />
              </Item.Group>
            </Segment>
          )}
        </div>
      )}
    </div>
  );
};

export default Users;
