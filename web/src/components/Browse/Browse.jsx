import BrowseSession from './BrowseSession';
import { getLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import {
  boundedTabText,
  maxStoredTabs,
  readBoundedTabState,
  writeBoundedTabState,
} from '../../lib/tabStorage';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { Icon, Menu, Tab } from 'semantic-ui-react';

let tabCounter = 0;

// Load tabs from localStorage
const loadTabsFromStorage = () => {
  const { tabCounter: restoredCounter, tabs } = readBoundedTabState(
    getLocalStorageItem,
    'slskr-browse-tabs',
  );
  tabCounter = restoredCounter;
  return tabs
    .map((tab) => {
      if (!tab || typeof tab !== 'object' || Array.isArray(tab)) return null;
      const username = boundedTabText(tab.username);
      const key = boundedTabText(tab.key);
      const label = boundedTabText(tab.label);
      return {
        key: key || `tab-${tabCounter}`,
        label: label || username || 'New Tab',
        username,
      };
    })
    .filter(Boolean);
};

// Save tabs to localStorage
const saveTabsToStorage = (tabsToSave) => {
  writeBoundedTabState(
    setLocalStorageItem,
    'slskr-browse-tabs',
    tabCounter,
    tabsToSave.map((tab) => ({
      key: boundedTabText(tab.key),
      label: boundedTabText(tab.label),
      username: boundedTabText(tab.username),
    })),
  );
};

const Browse = () => {
  const location = useLocation();
  const [tabs, setTabs] = useState(() => loadTabsFromStorage());
  const [activeIndex, setActiveIndex] = useState(0);
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
    const username = boundedTabText(newUsername);
    setTabs((previous) =>
      previous.map((t) =>
        t.key === tabKey
          ? { ...t, label: username, username }
          : t,
      ),
    );
  }, []);

  closeTabRef.current = closeTab;
  updateTabRef.current = updateTabLabel;

  const createTab = useCallback((username = '') => {
    tabCounter += 1;
    const safeUsername = boundedTabText(username);
    const tabKey = `tab-${tabCounter}`;
    return {
      key: tabKey,
      label: safeUsername || 'New Tab',
      username: safeUsername,
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

  // Handle navigation with user in state (quick browse from search)
  useEffect(() => {
    const user =
      location.state?.user || new URLSearchParams(location.search).get('user');

    if (user?.trim()) {
      const requestedUser = user.trim();
      const existingIndex = tabs.findIndex(
        (t) => t.username === requestedUser,
      );

      if (existingIndex === -1) {
        // Create new tab for this user - use callback to get correct index
        setTabs((previous) => {
      const newTabs = [...previous, createTab(requestedUser)].slice(-maxStoredTabs);
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
  }, [location.search, location.state]);

  const handleAddTab = () => {
    setTabs((previous) => {
      const newTabs = [...previous, createTab()].slice(-maxStoredTabs);
      setActiveIndex(newTabs.length - 1);
      return newTabs;
    });
  };

  const panes = tabs.map((tab) => ({
    menuItem: (
      <Menu.Item key={tab.key}>
        <Icon name={tab.username ? 'folder open' : 'search'} />
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
    render: () => (
      <Tab.Pane
        attached={false}
        key={tab.key}
        style={{ border: 'none', boxShadow: 'none' }}
      >
        <BrowseSession
          key={tab.key}
          onUsernameChange={(newUsername) =>
            updateTabRef.current?.(tab.key, newUsername)
          }
          username={tab.username}
        />
      </Tab.Pane>
    ),
  }));

  return (
    <div className="search-container">
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
                aria-label="Open a new browse tab"
                key="add-tab"
                onClick={handleAddTab}
                title="Open a new browse tab"
              >
                <Icon name="plus" />
              </Menu.Item>
            ),
            render: () => null,
          },
        ]}
        renderActiveOnly={false}
      />
    </div>
  );
};

export default Browse;
