import './Rooms.css';
import { toDisplayError } from '../../lib/errors';
import { createRoomsHubConnection } from '../../lib/hubFactory';
import { getLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import {
  boundedTabText,
  maxStoredTabs,
  readBoundedTabState,
  writeBoundedTabState,
} from '../../lib/tabStorage';
import * as rooms from '../../lib/rooms';
import { usePolling } from '../../lib/usePolling';
import PlaceholderSegment from '../Shared/PlaceholderSegment';
import RoomCreateModal from './RoomCreateModal';
import RoomSession from './RoomSession';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { toast } from 'react-toastify';
import {
  Button,
  Dropdown,
  Icon,
  Message,
  Menu,
  Popup,
  Segment,
  Tab,
} from 'semantic-ui-react';

let tabCounter = 0;

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (record) => record && typeof record === 'object' && !Array.isArray(record),
  );

const normalizeJoinedRooms = (value) =>
  (Array.isArray(value) ? value : [])
    .filter((roomName) => typeof roomName === 'string' && roomName.trim())
    .map((roomName) => roomName.trim())
    .sort();

const normalizeTab = (tab) => {
  if (!tab || typeof tab !== 'object' || Array.isArray(tab)) return null;
  const roomName = boundedTabText(tab.roomName);
  const key = boundedTabText(tab.key);
  const label = boundedTabText(tab.label);
  return {
    key: key || `room-tab-${roomName || tabCounter}`,
    label: label || roomName || 'New Room Tab',
    roomName,
  };
};

// Load tabs from localStorage
const loadTabsFromStorage = () => {
  const { tabCounter: restoredCounter, tabs } = readBoundedTabState(
    getLocalStorageItem,
    'slskr-room-tabs',
  );
  tabCounter = restoredCounter;
  return tabs.map(normalizeTab).filter(Boolean);
};

// Save tabs to localStorage
const saveTabsToStorage = (tabsToSave) => {
  writeBoundedTabState(
    setLocalStorageItem,
    'slskr-room-tabs',
    tabCounter,
    tabsToSave.map(normalizeTab).filter(Boolean),
  );
};

const Rooms = ({ runtimeProfile } = {}) => {
  const navigate = useNavigate();
  const [tabs, setTabs] = useState(() => loadTabsFromStorage());
  const [activeIndex, setActiveIndex] = useState(0);
  const [availableRooms, setAvailableRooms] = useState([]);
  const [joinedRooms, setJoinedRooms] = useState([]);
  const [availableRoomsError, setAvailableRoomsError] = useState('');
  const [joinedRoomsError, setJoinedRoomsError] = useState('');
  const [roomSearchLoading, setRoomSearchLoading] = useState(false);
  const closeTabRef = useRef(null);
  const mountedRef = useRef(false);
  const hydrateRequestIdRef = useRef(0);
  const availableRoomsRequestIdRef = useRef(0);
  const roomActionRequestIdRef = useRef(0);
  const roomActionInFlightRef = useRef(false);
  const roomsRequestInFlightRef = useRef(false);
  const [roomActionPending, setRoomActionPending] = useState(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      hydrateRequestIdRef.current += 1;
      availableRoomsRequestIdRef.current += 1;
      roomActionRequestIdRef.current += 1;
    };
  }, []);

  const closeTab = useCallback((tabKey) => {
    setTabs((previous) => {
      const newTabs = previous.filter((t) => t.key !== tabKey);
      setActiveIndex((currentIndex) =>
        currentIndex >= newTabs.length
          ? Math.max(0, newTabs.length - 1)
          : currentIndex,
      );
      return newTabs;
    });
  }, []);

  closeTabRef.current = closeTab;

  const beginRoomAction = useCallback(() => {
    if (!mountedRef.current || roomActionInFlightRef.current) return false;
    roomActionInFlightRef.current = true;
    setRoomActionPending(true);
    return true;
  }, [mountedRef]);

  const finishRoomAction = useCallback(() => {
    roomActionInFlightRef.current = false;
    if (mountedRef.current) setRoomActionPending(false);
  }, [mountedRef]);

  const createTab = useCallback((roomName = '') => {
    tabCounter += 1;
    const safeRoomName = boundedTabText(roomName);
    const tabKey = `room-tab-${tabCounter}`;
    return {
      key: tabKey,
      label: safeRoomName || 'New Room Tab',
      roomName: safeRoomName,
    };
  }, []);

  // Create initial tab on mount
  useEffect(() => {
    if (tabs.length === 0) {
      setTabs([createTab()]);
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Auto-create tab if all closed, and reset counter to keep numbers reasonable
  useEffect(() => {
    if (tabs.length === 0) {
      tabCounter = 0; // Reset counter when starting fresh
      setTabs([createTab()]);
    }
  }, [tabs.length, createTab]);

  // Save tabs to localStorage whenever they change
  useEffect(() => {
    if (tabs.length > 0) {
      saveTabsToStorage(tabs);
    }
  }, [tabs]);

  const openRoomTab = useCallback(
    (roomName) => {
      if (!roomName) return;

      const existingTabIndex = tabs.findIndex((t) => t.roomName === roomName);
      if (existingTabIndex === -1) {
        setTabs((previous) => {
          const newTabs = [...previous, createTab(roomName)].slice(-maxStoredTabs);
          setActiveIndex(newTabs.length - 1);
          return newTabs;
        });
      } else {
        setActiveIndex(existingTabIndex);
      }
    },
    [createTab, tabs],
  );

  const hydrateJoinedRooms = useCallback(async () => {
    const requestId = ++hydrateRequestIdRef.current;
    if (!mountedRef.current) return;

    try {
      const joined = await rooms.getJoined();
      if (
        !mountedRef.current ||
        requestId !== hydrateRequestIdRef.current
      ) {
        return;
      }
      const normalized = normalizeJoinedRooms(joined);
      setJoinedRooms(normalized);
      setJoinedRoomsError('');
      if (normalized.length > 0) {
        setTabs((previous) => {
          const existingRooms = new Set(
            previous.map((tab) => tab.roomName).filter(Boolean),
          );
          const restoredTabs = normalized
            .filter((roomName) => !existingRooms.has(roomName))
            .map((roomName) => createTab(roomName));

          return restoredTabs.length > 0
            ? [...previous.filter((tab) => tab.roomName), ...restoredTabs].slice(-maxStoredTabs)
            : previous;
        });
      }
    } catch (error) {
      console.error('Failed to fetch joined rooms:', error);
      if (mountedRef.current && requestId === hydrateRequestIdRef.current) {
        setJoinedRoomsError(toDisplayError(error, 'Failed to load joined rooms'));
      }
    }
  }, [createTab]);

  usePolling(hydrateJoinedRooms, 60_000);

  useEffect(() => {
    let disposed = false;
    const roomsHub = createRoomsHubConnection();
    roomsHub.on('changed', () => {
      if (!disposed && mountedRef.current) void hydrateJoinedRooms();
    });
    roomsHub.start().catch((error) => {
      if (!disposed) console.error('Failed to start rooms event feed:', error);
    });
    return () => {
      disposed = true;
      hydrateRequestIdRef.current += 1;
      roomsHub.stop().catch(() => {});
    };
  }, [hydrateJoinedRooms, mountedRef]);

  const fetchAvailableRooms = async () => {
    if (!mountedRef.current || roomsRequestInFlightRef.current) return;

    const requestId = ++availableRoomsRequestIdRef.current;
    roomsRequestInFlightRef.current = true;
    setRoomSearchLoading(true);
    try {
      const available = await rooms.getAvailable();
      if (
        mountedRef.current &&
        requestId === availableRoomsRequestIdRef.current
      ) {
        setAvailableRooms(
          asRecords(available)
            .filter((room) => typeof room.name === 'string' && room.name.trim())
            .map((room) => ({
              ...room,
              name: room.name.trim(),
              userCount: Number.isFinite(Number(room.userCount))
                ? Number(room.userCount)
                : 0,
            })),
        );
        setAvailableRoomsError('');
      }
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === availableRoomsRequestIdRef.current
      ) {
        setAvailableRoomsError(
          toDisplayError(error, 'Failed to load available rooms'),
        );
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === availableRoomsRequestIdRef.current
      ) {
        setRoomSearchLoading(false);
      }
      roomsRequestInFlightRef.current = false;
    }
  };

  const joinRoom = async (roomName) => {
    const trimmedRoomName = `${roomName || ''}`.trim();
    if (!trimmedRoomName || !beginRoomAction()) return;
    const requestId = ++roomActionRequestIdRef.current;
    try {
      await rooms.join({ roomName: trimmedRoomName });

      // Refresh joined rooms
      const joined = await rooms.getJoined();
      if (
        !mountedRef.current ||
        requestId !== roomActionRequestIdRef.current
      ) {
        return;
      }
      setJoinedRooms(normalizeJoinedRooms(joined));
      setJoinedRoomsError('');
      openRoomTab(trimmedRoomName);
    } catch (error) {
      console.error('Failed to join room:', error);
      if (mountedRef.current) {
        toast.error(`Failed to join room: ${toDisplayError(error)}`);
      }
    } finally {
      finishRoomAction();
    }
  };

  const leaveRoom = async (roomName) => {
    const trimmedRoomName = `${roomName || ''}`.trim();
    if (!trimmedRoomName || !beginRoomAction()) return;
    const requestId = ++roomActionRequestIdRef.current;
    try {
      await rooms.leave({ roomName: trimmedRoomName });

      // Refresh joined rooms
      const joined = await rooms.getJoined();
      if (
        !mountedRef.current ||
        requestId !== roomActionRequestIdRef.current
      ) {
        return;
      }
      setJoinedRooms(normalizeJoinedRooms(joined));
      setJoinedRoomsError('');

      // Close the tab for this room
      const tabToClose = tabs.find((t) => t.roomName === trimmedRoomName);
      if (tabToClose) {
        closeTabRef.current?.(tabToClose.key);
      }
    } catch (error) {
      console.error('Failed to leave room:', error);
      if (mountedRef.current) {
        toast.error(`Failed to leave room: ${toDisplayError(error)}`);
      }
    } finally {
      finishRoomAction();
    }
  };

  const createRoom = async (roomName, isPrivate) => {
    // For now, private room creation isn't directly supported by Soulseek protocol
    // We just attempt to join the room, which may create it if it doesn't exist
    await joinRoom(roomName);
  };

  const handleAddTab = () => {
    setTabs((previous) => {
      const newTabs = [...previous, createTab()].slice(-maxStoredTabs);
      setActiveIndex(newTabs.length - 1);
      return newTabs;
    });
  };

  const handleUserProfile = useCallback(
    (username) => {
      navigate('/users', { state: { user: username } });
    },
    [navigate],
  );

  const handleBrowseShares = useCallback(
    (username) => {
      navigate('/browse', { state: { user: username } });
    },
    [navigate],
  );

  const roomOptions = availableRooms.map((r) => ({
    description: r.isPrivate ? 'Private' : '',
    key: r.name,
    text: `${r.name} (${r.userCount} users)`,
    value: r.name,
  }));

  const panes = tabs.map((tab, index) => ({
    menuItem: (
      <Menu.Item key={tab.key}>
        <Icon name={tab.roomName ? 'comments' : 'search'} />
        {tab.label}
        {tabs.length > 1 && (
          <Icon
            name="close"
            onClick={(event) => {
              event.stopPropagation();
              closeTabRef.current?.(tab.key);
            }}
            style={{ marginLeft: '8px', opacity: 0.7 }}
          />
        )}
      </Menu.Item>
    ),
    pane: (
      <Tab.Pane
        attached={false}
        key={tab.key}
        style={{ border: 'none', boxShadow: 'none' }}
      >
        <RoomSession
          active={index === activeIndex}
          key={tab.key}
          onBrowseShares={handleBrowseShares}
          onLeaveRoom={leaveRoom}
          onUserProfile={handleUserProfile}
          roomName={tab.roomName}
        />
      </Tab.Pane>
    ),
  }));

  if (runtimeProfile === 'legacy') {
    return (
      <div className="rooms compatibility-rooms">
        <button
          aria-label="Open a new room tab"
          onClick={() => handleAddTab()}
          type="button"
        >
          Open a new room tab
        </button>
      </div>
    );
  }

  return (
    <div className="rooms">
      <Segment
        className="rooms-segment"
        raised
      >
        <div className="rooms-segment-icon">
          <Icon
            name="comments"
            size="big"
          />
        </div>
        <div
          style={{
            display: 'flex',
            flex: 1,
            flexDirection: 'column',
            gap: '8px',
          }}
        >
          <div style={{ alignItems: 'center', display: 'flex', gap: '8px' }}>
            <Dropdown
              className="rooms-input"
              clearable
              disabled={roomActionPending}
              fluid
              loading={roomSearchLoading}
              onChange={(_, { value }) => {
                if (value) {
                  joinRoom(value);
                }
              }}
              onOpen={() => fetchAvailableRooms()}
              options={roomOptions}
              placeholder="Search existing rooms..."
              search
              selection
            />
            <RoomCreateModal onCreateRoom={createRoom} />
            <Popup
              content="Reload rooms joined by the daemon and reopen their tabs."
              trigger={
                <Button
                  aria-label="Reload joined rooms"
                  icon="refresh"
                  onClick={hydrateJoinedRooms}
                  title="Reload joined rooms"
                />
              }
            />
          </div>
          {(availableRoomsError || joinedRoomsError) && (
            <Message
              compact
              negative
            >
              {availableRoomsError || joinedRoomsError}
            </Message>
          )}
        </div>
      </Segment>
      <Tab
        activeIndex={activeIndex}
        menu={{
          attached: false,
          inverted: true,
          secondary: true,
          tabular: false,
        }}
        onTabChange={(event, { activeIndex: newIndex }) =>
          setActiveIndex(newIndex)
        }
        panes={[
          ...panes,
          {
            menuItem: (
              <Menu.Item
                aria-label="Open a new room tab"
                key="add-tab"
                onClick={handleAddTab}
                title="Open a new room tab"
              >
                <Icon name="plus" />
              </Menu.Item>
            ),
            pane: null,
          },
        ]}
        renderActiveOnly={false}
      />
    </div>
  );
};

export default Rooms;
