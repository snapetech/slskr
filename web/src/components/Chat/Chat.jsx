import './Chat.css';
import * as chat from '../../lib/chat';
import { toDisplayError } from '../../lib/errors';
import { getLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import { useMountedRef } from '../../lib/useMountedRef';
import ChatSession from './ChatSession';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { toast } from 'react-toastify';
import {
  Button,
  Icon,
  Input,
  Label,
  Menu,
  Popup,
  Segment,
  Tab,
} from 'semantic-ui-react';

let tabCounter = 0;

const normalizeTab = (tab) => {
  if (!tab || typeof tab !== 'object' || Array.isArray(tab)) return null;
  const username = `${tab.username ?? ''}`.trim();
  return {
    ...tab,
    key: `${tab.key || `chat-tab-${username || tabCounter}`}`,
    label: `${tab.label || username || 'New Chat'}`,
    username,
  };
};

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (record) => record && typeof record === 'object' && !Array.isArray(record),
  );

// Load tabs from localStorage
const loadTabsFromStorage = () => {
  try {
    const saved = getLocalStorageItem('slskr-chat-tabs');

    if (saved) {
      const parsed = JSON.parse(saved);
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return [];
      // Restore tabCounter to avoid key collisions
      tabCounter = Number.isInteger(parsed.tabCounter) && parsed.tabCounter >= 0
        ? parsed.tabCounter
        : 0;
      return Array.isArray(parsed.tabs)
        ? parsed.tabs.map(normalizeTab).filter(Boolean)
        : [];
    }
  } catch {
    // ignore
  }

  return [];
};

// Save tabs to localStorage
const saveTabsToStorage = (tabsToSave) => {
  setLocalStorageItem(
    'slskr-chat-tabs',
    JSON.stringify({ tabCounter, tabs: tabsToSave }),
  );
};

const Chat = ({ runtimeProfile, state }) => {
  const location = useLocation();
  const [tabs, setTabs] = useState(() => loadTabsFromStorage());
  const [activeIndex, setActiveIndex] = useState(0);
  const [conversations, setConversations] = useState([]);
  const [usernameInput, setUsernameInput] = useState('');
  const mountedRef = useMountedRef();
  const hydrateRequestIdRef = useRef(0);
  const deleteRequestIdRef = useRef(0);
  const deleteInFlightRef = useRef(false);
  const closeTabRef = useRef(null);
  const updateTabRef = useRef(null);

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

  const updateTabLabel = useCallback((tabKey, newUsername) => {
    setTabs((previous) =>
      previous.map((t) =>
        t.key === tabKey
          ? { ...t, label: newUsername, username: newUsername }
          : t,
      ),
    );
  }, []);

  closeTabRef.current = closeTab;
  updateTabRef.current = updateTabLabel;

  const createTab = useCallback((username = '') => {
    tabCounter += 1;
    const tabKey = `chat-tab-${tabCounter}`;
    return {
      key: tabKey,
      label: username || 'New Chat',
      username,
    };
  }, []);

  const startConversation = useCallback(
    (username) => {
      if (!username || !username.trim()) return;

      const trimmedUsername = username.trim();
      const existingIndex = tabs.findIndex(
        (t) => t.username === trimmedUsername,
      );

      if (existingIndex === -1) {
        setTabs((previous) => {
          const newTabs = [...previous, createTab(trimmedUsername)];
          setActiveIndex(newTabs.length - 1);
          return newTabs;
        });
      } else {
        setActiveIndex(existingIndex);
      }
    },
    [createTab, tabs],
  );

  const hydrateConversations = useCallback(async () => {
    if (!mountedRef.current) return;
    const requestId = ++hydrateRequestIdRef.current;

    try {
      const serverConversations = await chat.getAll();
      if (
        !mountedRef.current ||
        hydrateRequestIdRef.current !== requestId
      ) {
        return;
      }
      const activeConversations = asRecords(serverConversations)
        .filter((conversation) => typeof conversation.username === 'string' && conversation.username.trim())
        .map((conversation) => ({
          ...conversation,
          hasUnAcknowledgedMessages: Boolean(conversation.hasUnAcknowledgedMessages),
          username: conversation.username.trim(),
        }))
        .sort((a, b) => {
          if (a.hasUnAcknowledgedMessages !== b.hasUnAcknowledgedMessages) {
            return a.hasUnAcknowledgedMessages ? -1 : 1;
          }

          return a.username.localeCompare(b.username);
        });

      setConversations(activeConversations);
      if (activeConversations.length > 0) {
        setTabs((previous) => {
          const existingUsernames = new Set(
            previous.map((tab) => tab.username).filter(Boolean),
          );
          const restoredTabs = activeConversations
            .filter((conversation) => !existingUsernames.has(conversation.username))
            .map((conversation) => createTab(conversation.username));

          return restoredTabs.length > 0
            ? [...previous.filter((tab) => tab.username), ...restoredTabs]
            : previous;
        });
      }
    } catch (error) {
      if (
        mountedRef.current &&
        hydrateRequestIdRef.current === requestId
      ) {
        console.error('Failed to hydrate conversations:', error);
      }
    }
  }, [createTab, mountedRef]);

  // Create initial tab on mount if none exist
  useEffect(() => {
    if (tabs.length === 0) {
      setTabs([createTab()]);
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    void hydrateConversations();
  }, [hydrateConversations]);

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

  // Handle navigation with username in state (quick chat from user profile)
  useEffect(() => {
    const username = location.state?.user;

    if (username) {
      const existingIndex = tabs.findIndex((t) => t.username === username);

      if (existingIndex === -1) {
        // Create new tab for this user
        setTabs((previous) => {
          const newTabs = [...previous, createTab(username)];
          setActiveIndex(newTabs.length - 1);
          return newTabs;
        });
      } else {
        setActiveIndex(existingIndex);
      }

      // Clear the state to prevent re-triggering
      window.history.replaceState({}, document.title);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state]);

  const handleAddTab = () => {
    setTabs((previous) => {
      const newTabs = [...previous, createTab()];
      setActiveIndex(newTabs.length - 1);
      return newTabs;
    });
  };

  const handleDeleteConversation = useCallback(
    async (username) => {
      if (!mountedRef.current || deleteInFlightRef.current) return;
      deleteInFlightRef.current = true;
      const requestId = ++deleteRequestIdRef.current;

      try {
        // Remove the conversation from chat API
        await chat.remove({ username });
        if (
          !mountedRef.current ||
          deleteRequestIdRef.current !== requestId
        ) {
          return;
        }

        // Close the tab only after the server confirms deletion.
        const tabToClose = tabs.find((t) => t.username === username);
        if (tabToClose) {
          closeTabRef.current?.(tabToClose.key);
        }
      } catch (error) {
        console.error('Failed to remove conversation:', error);
        if (mountedRef.current) {
          toast.error(`Failed to delete conversation: ${toDisplayError(error)}`);
        }
      } finally {
        deleteInFlightRef.current = false;
      }
    },
    [mountedRef, tabs],
  );

  const panes = tabs.map((tab, index) => ({
    menuItem: (
      <Menu.Item key={tab.key}>
        <Icon name={tab.username ? 'comment' : 'search'} />
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
        <ChatSession
          active={index === activeIndex}
          key={tab.key}
          onDelete={handleDeleteConversation}
          user={state?.user}
          username={tab.username}
        />
      </Tab.Pane>
    ),
  }));

  if (runtimeProfile === 'legacy') {
    return (
      <div className="chats compatibility-chat">
        <button
          aria-label="Open a new chat tab"
          onClick={() => handleAddTab()}
          type="button"
        >
          Open a new chat tab
        </button>
      </div>
    );
  }

  return (
    <div className="chats">
      <Segment
        className="chat-segment"
        raised
      >
        <div className="chat-segment-icon">
          <Icon
            name="comment"
            size="big"
          />
        </div>
        <Input
          action={{
            'aria-label': 'Start chat with user',
            icon: 'chat',
            onClick: () => {
              if (usernameInput?.trim()) {
                startConversation(usernameInput.trim());
                setUsernameInput('');
              }
            },
            title: 'Start chat with user',
          }}
          className="chat-input"
          onChange={(event) => setUsernameInput(event.target.value)}
          onKeyUp={(event) => {
            if (event.key === 'Enter' && usernameInput?.trim()) {
              startConversation(usernameInput.trim());
              setUsernameInput('');
            }
          }}
          placeholder="Chat with user..."
          size="big"
          value={usernameInput}
        />
        <Popup
          content="Reload saved conversations from the server after a browser refresh or restart."
          trigger={
            <Button
              aria-label="Reload saved conversations"
              icon="refresh"
              onClick={hydrateConversations}
              title="Reload saved conversations"
            />
          }
        />
      </Segment>
      {conversations.length > 0 && (
        <Segment className="chat-recovery-rail">
          {conversations.map((conversation) => (
            <Popup
              content="Open this saved conversation."
              key={conversation.username}
              trigger={
                <Button
                  basic
                  compact
                  onClick={() => startConversation(conversation.username)}
                  size="small"
                >
                  <Icon name="comment alternate" />
                  {conversation.username}
                  {conversation.hasUnAcknowledgedMessages && (
                    <Label
                      circular
                      color="red"
                      size="mini"
                    >
                      {conversation.unAcknowledgedMessageCount}
                    </Label>
                  )}
                </Button>
              }
            />
          ))}
        </Segment>
      )}
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
                aria-label="Open a new chat tab"
                key="add-tab"
                onClick={handleAddTab}
                title="Open a new chat tab"
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

export default Chat;
