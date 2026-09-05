/* eslint-disable promise/prefer-await-to-then */
import './Browse.css';
import * as transfers from '../../lib/transfers';
import {
  getLocalStorageItem,
  getLocalStorageKeys,
  removeLocalStorageItem,
  setLocalStorageItem,
} from '../../lib/storage';
import * as userNotes from '../../lib/userNotes';
import * as users from '../../lib/users';
import { createPollingController } from '../../lib/usePolling';
import PlaceholderSegment from '../Shared/PlaceholderSegment';
import UserCard from '../Shared/UserCard';
import UserNoteModal from '../Users/UserNoteModal';
import Directory from './Directory';
import DirectoryTree from './DirectoryTree';
import * as lzString from 'lz-string';
import React, { Component } from 'react';
import { toast } from 'react-toastify';
import {
  Button,
  Card,
  Icon,
  Input,
  Loader,
  Popup,
  Segment,
} from 'semantic-ui-react';

const initialState = {
  browseError: undefined,
  browseState: 'idle',
  browseStatus: 0,
  downloadPending: false,
  info: {
    directories: 0,
    files: 0,
    lockedDirectories: 0,
    lockedFiles: 0,
  },
  selectedDirectory: {},
  selectedFiles: [],
  separator: '\\',
  tree: [],
  username: '',
  userNote: null,
};

const MAX_BROWSE_CACHE_ENTRIES = 50;
const BROWSE_CACHE_PREFIX = 'slskr-browse-state-';
const MAX_BROWSE_CACHE_COMPRESSED_CHARACTERS = 512 * 1024;
const MAX_BROWSE_CACHE_JSON_CHARACTERS = 4 * 1024 * 1024;
const MAX_BROWSE_DIRECTORY_NODES = 10_000;
const MAX_BROWSE_FILES_PER_DIRECTORY = 2_000;
const MAX_BROWSE_TEXT_CHARACTERS = 2_048;
const MAX_BROWSE_TREE_DEPTH = 64;

export const getBrowseErrorMessage = (error) => {
  const data = error?.response?.data;

  if (typeof data === 'string' && data.trim()) {
    return data.trim();
  }

  if (data && typeof data === 'object' && !Array.isArray(data)) {
    const message = [data.detail, data.message, data.error, data.title].find(
      (value) => typeof value === 'string' && value.trim(),
    );
    if (message) {
      return message.trim();
    }
  }

  return typeof error?.message === 'string' && error.message.trim()
    ? error.message.trim()
    : 'Browse failed';
};

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (record) => record && typeof record === 'object' && !Array.isArray(record),
  );

const toNonNegativeInteger = (value) => {
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 ? Math.floor(number) : 0;
};

const normalizeBrowseFile = (file) => {
  if (!file || typeof file !== 'object' || Array.isArray(file)) return null;
  const filename = typeof file.filename === 'string'
    ? file.filename.trim().slice(0, MAX_BROWSE_TEXT_CHARACTERS)
    : '';
  if (!filename) return null;
  const id = typeof file.id === 'string' || typeof file.id === 'number'
    ? String(file.id).trim().slice(0, MAX_BROWSE_TEXT_CHARACTERS)
    : '';
  return {
    ...(id ? { id } : {}),
    bitDepth: toNonNegativeInteger(file.bitDepth),
    bitRate: toNonNegativeInteger(file.bitRate),
    filename,
    isVariableBitRate: file.isVariableBitRate === true,
    length: toNonNegativeInteger(file.length),
    sampleRate: toNonNegativeInteger(file.sampleRate),
    size: toNonNegativeInteger(file.size),
  };
};

const normalizeDirectory = (
  directory,
  depth = 0,
  budget = { remaining: MAX_BROWSE_DIRECTORY_NODES },
) => {
  if (
    !directory
    || typeof directory !== 'object'
    || Array.isArray(directory)
    || depth > MAX_BROWSE_TREE_DEPTH
    || budget.remaining <= 0
  ) {
    return null;
  }
  budget.remaining -= 1;
  const name = typeof directory.name === 'string'
    ? directory.name.trim().slice(0, MAX_BROWSE_TEXT_CHARACTERS)
    : '';
  if (!name) return null;
  return {
    children: depth < MAX_BROWSE_TREE_DEPTH
      ? asRecords(directory.children)
          .map((child) => normalizeDirectory(child, depth + 1, budget))
          .filter(Boolean)
      : [],
    fileCount: toNonNegativeInteger(directory.fileCount),
    files: asRecords(directory.files)
      .slice(0, MAX_BROWSE_FILES_PER_DIRECTORY)
      .map(normalizeBrowseFile)
      .filter(Boolean),
    locked: directory.locked === true,
    name,
  };
};

const normalizeDirectories = (value) => {
  const budget = { remaining: MAX_BROWSE_DIRECTORY_NODES };
  return asRecords(value)
    .map((directory) => normalizeDirectory(directory, 0, budget))
    .filter(Boolean);
};

// Cleanup old browse cache entries using LRU strategy
const cleanupBrowseCache = () => {
  try {
    const cacheEntries = getLocalStorageKeys()
      .filter((key) => key.startsWith(BROWSE_CACHE_PREFIX))
      .map((key) => {
        const data = getLocalStorageItem(key, '');
        return { key, size: data ? data.length : 0 };
      });

    if (cacheEntries.length > MAX_BROWSE_CACHE_ENTRIES) {
      // Sort by size (larger = older/more complete browses, keep those)
      // Remove smallest/oldest entries first
      cacheEntries.sort((a, b) => a.size - b.size);
      const toRemove = cacheEntries.slice(
        0,
        cacheEntries.length - MAX_BROWSE_CACHE_ENTRIES,
      );
      for (const entry of toRemove) {
        removeLocalStorageItem(entry.key);
      }
    }
  } catch (error) {
    console.debug('Browse cache cleanup error:', error);
  }
};

class BrowseSession extends Component {
  constructor(props) {
    super(props);

    this.state = initialState;
    this.isMountedFlag = false;
    this.mountTimeoutId = null;
    this.browseRequestId = 0;
    this.userNoteRequestId = 0;
    this.pollController = null;
    this.downloadInFlight = false;
    this.downloadRequestId = 0;
  }

  componentDidMount() {
    this.isMountedFlag = true;

    // Check for username from props (tab only - navigation handled by parent)
    const userToBrowse = this.props.username;

    if (userToBrowse) {
      this.fetchUserNote(userToBrowse);
      // Try to load cached data first
      const hasCachedData = this.loadState();

      // Small delay to ensure ref is ready
      this.mountTimeoutId = setTimeout(() => {
        this.mountTimeoutId = null;

        if (!this.isMountedFlag) {
          return;
        }

        if (this.inputtext?.inputRef?.current) {
          this.inputtext.inputRef.current.value = userToBrowse;
        }

        // Only fetch if we don't have cached data
        if (!hasCachedData) {
          this.setState({ username: userToBrowse }, this.browse);
        }
      }, 50);
    } else {
      this.loadState();
    }

    document.addEventListener('keyup', this.keyUp, false);
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    this.browseRequestId += 1;
    this.userNoteRequestId += 1;
    this.downloadRequestId += 1;
    if (this.mountTimeoutId) {
      clearTimeout(this.mountTimeoutId);
      this.mountTimeoutId = null;
    }
    this.stopPolling();
    document.removeEventListener('keyup', this.keyUp, false);
  }

  fetchUserNote = async (username) => {
    const requestId = ++this.userNoteRequestId;

    try {
      const response = await userNotes.getNote({ username });
      if (this.isMountedFlag && requestId === this.userNoteRequestId) {
        this.setState({ userNote: response.data });
      }
    } catch {
      if (this.isMountedFlag && requestId === this.userNoteRequestId) {
        this.setState({ userNote: null });
      }
    }
  };

  // Start polling only when needed (during active browse)
  startPolling = () => {
    if (!this.pollController) {
      this.pollController = createPollingController(this.fetchStatus, 500, {
        immediate: false,
      });
    }
  };

  // Stop polling when not needed
  stopPolling = () => {
    if (this.pollController) {
      this.pollController.stop();
      this.pollController = null;
    }
  };

  browse = () => {
    if (!this.isMountedFlag) return;
    const username = this.inputtext?.inputRef?.current?.value;
    const normalizedUsername =
      typeof username === 'string' ? username.trim() : '';

    if (!normalizedUsername) {
      return;
    }

    const requestId = ++this.browseRequestId;

    // Notify parent to update tab label
    if (this.props.onUsernameChange) {
      this.props.onUsernameChange(normalizedUsername);
    }

    this.setState(
      {
        browseError: undefined,
        browseState: 'pending',
        browseStatus: 0,
        username: normalizedUsername,
      },
      () => {
        if (!this.isMountedFlag || requestId !== this.browseRequestId) {
          return;
        }

        this.fetchUserNote(normalizedUsername);
        // Start polling only while browse is in progress
        this.startPolling();

        users
          .browse({ username: normalizedUsername })
          .then((response) => {
            if (!this.isMountedFlag || requestId !== this.browseRequestId) {
              return;
            }

            const directories = normalizeDirectories(response?.directories);
            const lockedDirectories = normalizeDirectories(
              response?.lockedDirectories,
            );

            let separator;

            const directoryCount = directories.length;
            const fileCount = directories.reduce((accumulator, directory) => {
              // examine each directory as we process it to see if it contains \ or /, and set separator accordingly
              if (!separator) {
                if (directory.name.includes('\\')) separator = '\\';
                else if (directory.name.includes('/')) separator = '/';
              }

              return accumulator + directory.fileCount;
            }, 0);

            const lockedDirectoryCount = lockedDirectories.length;
            const lockedFileCount = lockedDirectories.reduce(
              (accumulator, directory) => accumulator + directory.fileCount,
              0,
            );

            separator ||= initialState.separator;

            const allDirectories = directories.concat(
              lockedDirectories.map((d) => ({ ...d, locked: true })),
            );

            this.setState({
              info: {
                directories: directoryCount,
                files: fileCount,
                lockedDirectories: lockedDirectoryCount,
                lockedFiles: lockedFileCount,
              },
              separator,
              tree: this.getDirectoryTree({
                directories: allDirectories,
                separator,
              }),
            });
          })
          .then(() => {
            if (!this.isMountedFlag || requestId !== this.browseRequestId) {
              return;
            }

            // Stop polling when browse completes
            this.stopPolling();
            this.setState(
              { browseError: undefined, browseState: 'complete' },
              () => {
                if (this.isMountedFlag && requestId === this.browseRequestId) {
                  this.saveState();
                }
              },
            );
          })
          .catch((error) => {
            if (!this.isMountedFlag || requestId !== this.browseRequestId) {
              return;
            }

            // Stop polling on error too
            this.stopPolling();
            this.setState({
              browseError: getBrowseErrorMessage(error),
              browseState: 'error',
            });
          });
      },
    );
  };

  clear = () => {
    this.browseRequestId += 1;
    this.userNoteRequestId += 1;
    this.downloadRequestId += 1;
    this.stopPolling();
    this.downloadInFlight = false;
    this.setState(initialState, () => {
      if (!this.isMountedFlag) return;
      this.saveState();
      this.inputtext?.focus?.();
    });
  };

  keyUp = (event) => (event.key === 'Escape' ? this.clear() : '');

  getStorageKey = () => {
    const username = this.props.username || this.state.username || 'default';
    return `slskr-browse-state-${username}`;
  };

  saveState = () => {
    if (this.inputtext?.inputRef?.current) {
      this.inputtext.inputRef.current.value = this.state.username;
      this.inputtext.inputRef.current.disabled =
        this.state.browseState !== 'idle';
    }

    // Only save if we have actual browse data
    if (this.state.username && this.state.tree.length > 0) {
      try {
        const persistedState = {
          info: {
            directories: toNonNegativeInteger(this.state.info?.directories),
            files: toNonNegativeInteger(this.state.info?.files),
            lockedDirectories: toNonNegativeInteger(this.state.info?.lockedDirectories),
            lockedFiles: toNonNegativeInteger(this.state.info?.lockedFiles),
          },
          selectedDirectory: normalizeDirectory(this.state.selectedDirectory),
          separator: ['\\', '/'].includes(this.state.separator)
            ? this.state.separator
            : initialState.separator,
          tree: normalizeDirectories(this.state.tree),
          username: this.state.username.slice(0, MAX_BROWSE_TEXT_CHARACTERS),
        };
        const serializedState = JSON.stringify(persistedState);
        if (serializedState.length > MAX_BROWSE_CACHE_JSON_CHARACTERS) return;
        const compressedState = lzString.compress(serializedState);
        if (compressedState.length > MAX_BROWSE_CACHE_COMPRESSED_CHARACTERS) return;
        setLocalStorageItem(
          this.getStorageKey(),
          compressedState,
        );
        // Cleanup old cache entries to prevent unbounded growth
        cleanupBrowseCache();
      } catch (error) {
        console.error(error);
      }
    }
  };

  loadState = () => {
    // Try to load saved state for this username
    const username = this.props.username;

    if (username) {
      try {
        const key = `slskr-browse-state-${username}`;
        const compressedState = getLocalStorageItem(key, '') || '';
        if (
          typeof compressedState !== 'string'
          || compressedState.length > MAX_BROWSE_CACHE_COMPRESSED_CHARACTERS
        ) {
          return false;
        }
        const decompressedState = lzString.decompress(compressedState);
        if (
          typeof decompressedState !== 'string'
          || decompressedState.length > MAX_BROWSE_CACHE_JSON_CHARACTERS
        ) {
          return false;
        }
        const savedState = JSON.parse(decompressedState);

        const savedTree = normalizeDirectories(savedState?.tree);
        if (savedState && savedTree.length > 0) {
          // We have cached data - use it instead of re-fetching
          this.setState({
            info: {
              directories: toNonNegativeInteger(savedState.info?.directories),
              files: toNonNegativeInteger(savedState.info?.files),
              lockedDirectories: toNonNegativeInteger(savedState.info?.lockedDirectories),
              lockedFiles: toNonNegativeInteger(savedState.info?.lockedFiles),
            },
            separator: ['\\', '/'].includes(savedState.separator)
              ? savedState.separator
              : initialState.separator,
            selectedDirectory:
              normalizeDirectory(savedState.selectedDirectory) ||
              initialState.selectedDirectory,
            tree: savedTree,
            username:
              typeof savedState.username === 'string'
                ? savedState.username.slice(0, MAX_BROWSE_TEXT_CHARACTERS)
                : username,
            browseState: 'complete',
          });
          return true; // Indicate we loaded cached data
        }
      } catch {
        // ignore - will fetch fresh
      }
    }

    return false;
  };

  fetchStatus = async () => {
    const { browseState, username } = this.state;
    // Only poll status when actively browsing AND we have a username
    if (browseState === 'pending' && username) {
      try {
        const response = await users.getBrowseStatus({ username });
        const status =
          response?.data && typeof response.data === 'object'
            ? {
                percentComplete: Math.min(
                  100,
                  Math.max(0, Number(response.data.percentComplete) || 0),
                ),
              }
            : 0;
        if (this.isMountedFlag && username === this.state.username) {
          this.setState({
            browseStatus: status,
          });
        }
      } catch {
        // Ignore 404s during status polling
      }
    }
  };

  getDirectoryTree = ({ directories, separator }) => {
    const normalizedDirectories = normalizeDirectories(directories);
    const pathSeparator = ['\\', '/'].includes(separator)
      ? separator
      : initialState.separator;

    if (normalizedDirectories.length === 0) {
      return [];
    }

    // Optimise this process so we only:
    // - loop through all directories once
    // - do the split once
    // - future look ups are done from the Map
    const depthMap = new Map();
    for (const d of normalizedDirectories) {
      const directoryDepth = d.name.split(pathSeparator).length;
      if (!depthMap.has(directoryDepth)) {
        depthMap.set(directoryDepth, []);
      }

      depthMap.get(directoryDepth).push(d);
    }

    const depth = Math.min(...Array.from(depthMap.keys()));

    return depthMap
      .get(depth)
      .map((directory) =>
        this.getChildDirectories(
          depthMap,
          directory,
          pathSeparator,
          depth + 1,
        ),
      );
  };

  getChildDirectories = (depthMap, root, separator, depth) => {
    if (!depthMap.has(depth)) {
      return { ...root, children: [] };
    }

    const children = depthMap
      .get(depth)
      .filter((d) => d.name.startsWith(root.name + separator));

    return {
      ...root,
      children: children.map((c) =>
        this.getChildDirectories(depthMap, c, separator, depth + 1),
      ),
    };
  };

  selectDirectory = (directory) => {
    this.setState({ selectedDirectory: { ...directory, children: [] } }, () =>
      this.saveState(),
    );
  };

  handleDeselectDirectory = () => {
    this.setState({ selectedDirectory: initialState.selectedDirectory }, () =>
      this.saveState(),
    );
  };

  handleRefresh = () => {
    // Force re-fetch by clearing cache and browsing again
    const { username } = this.state;

    if (username) {
      // Clear the cached state for this user
      try {
        removeLocalStorageItem(`slskr-browse-state-${username}`);
      } catch {
        // ignore
      }

      // Re-browse
      this.browse();
    }
  };

  handleDownloadDirectory = (directory) => {
    const { separator, username } = this.state;

    // Collect all files recursively
    const collectFiles = (folder) => {
      const folderName = typeof folder?.name === 'string' ? folder.name : '';
      const pathSeparator = ['\\', '/'].includes(separator)
        ? separator
        : initialState.separator;
      let collected = asRecords(folder?.files)
        .filter((file) => typeof file.filename === 'string' && file.filename)
        .map((file) => ({
          filename: folderName + pathSeparator + file.filename,
          size: toNonNegativeInteger(file.size),
        }));

      if (Array.isArray(folder?.children)) {
        for (const child of folder.children) {
          collected = collected.concat(collectFiles(child));
        }
      }

      return collected;
    };

    const filesToDownload = collectFiles(directory);

    if (filesToDownload.length === 0) {
      toast.info(
        'No files found in directory: ' +
          (typeof directory?.name === 'string'
            ? directory.name
            : 'selected folder'),
      );
      return;
    }

    if (this.downloadInFlight) return;

    if (
      // eslint-disable-next-line no-alert
      window.confirm(
        'Download ' +
          filesToDownload.length +
          ' files from ' +
          (directory.name || 'selected folder') +
          '?',
      )
    ) {
      const requestId = ++this.downloadRequestId;
      this.downloadInFlight = true;
      this.setState({ downloadPending: true });
      void transfers
        .download({ files: filesToDownload, username })
        .then(() => {
          if (
            this.isMountedFlag &&
            requestId === this.downloadRequestId
          ) {
            toast.success(
              'Queued ' + filesToDownload.length + ' files for download',
            );
          }
        })
        .catch((error) => {
          if (
            this.isMountedFlag &&
            requestId === this.downloadRequestId
          ) {
            console.error(error);
            toast.error(
              'Failed to queue download: ' + getBrowseErrorMessage(error),
            );
          }
        })
        .finally(() => {
          if (
            this.isMountedFlag &&
            requestId === this.downloadRequestId
          ) {
            this.downloadInFlight = false;
            this.setState({ downloadPending: false });
          }
        });
    }
  };

  render() {
    const {
      browseError,
      browseState,
      browseStatus,
      downloadPending,
      info,
      selectedDirectory,
      separator,
      tree,
      userNote,
      username,
    } = this.state;
    const { locked, name } = selectedDirectory;
    const pending = browseState === 'pending';
    const finished = ['complete', 'error'].includes(browseState);
    const emptyTree = finished && tree.length === 0;

    const files = asRecords(selectedDirectory.files).map((f) => ({
      ...f,
      filename:
        (typeof name === 'string' ? name : '') +
        (['\\', '/'].includes(separator) ? separator : initialState.separator) +
        f.filename,
    }));

    return (
      <div className="search-container">
        <Segment
          className="browse-segment"
          raised
        >
          <div className="browse-segment-icon">
            <Icon
              name="folder open"
              size="big"
            />
          </div>
          <Input
            action={
              !pending && (
                <Popup
                  content={
                    browseState === 'idle'
                      ? "Browse this Soulseek user's shared files."
                      : 'Clear this browse result and enter another username.'
                  }
                  position="top center"
                  trigger={
                    <Button
                      aria-label={
                        browseState === 'idle'
                          ? 'Browse user files'
                          : 'Clear browse result'
                      }
                      color={browseState === 'idle' ? undefined : 'red'}
                      icon={browseState === 'idle' ? 'search' : 'x'}
                      onClick={
                        browseState === 'idle' ? this.browse : this.clear
                      }
                    />
                  }
                />
              )
            }
            className="search-input"
            disabled={pending}
            input={
              <input
                data-lpignore="true"
                placeholder="Username"
                type="search"
              />
            }
            loading={pending}
            onKeyUp={(event) => (event.key === 'Enter' ? this.browse() : '')}
            placeholder="Username"
            ref={(input) => (this.inputtext = input)}
            size="big"
          />
        </Segment>
        {pending ? (
          <Loader
            active
            className="search-loader"
            inline="centered"
            size="big"
          >
            Downloaded {Math.round(browseStatus.percentComplete || 0)}% of
            Response
          </Loader>
        ) : (
          <div>
            {browseError ? (
              <span className="browse-error">
                Failed to browse {username}: {browseError}
              </span>
            ) : (
              <div className="browse-container">
                {emptyTree ? (
                  <PlaceholderSegment
                    caption="No user share to display"
                    icon="folder open"
                  />
                ) : (
                  <Card
                    className="browse-tree-card"
                    raised
                  >
                    <Card.Content>
                      <Card.Header
                        style={{
                          alignItems: 'center',
                          display: 'flex',
                          justifyContent: 'space-between',
                        }}
                      >
                        <span>
                          <Icon
                            color="green"
                            name="circle"
                          />
                          <UserCard username={username}>{username}</UserCard>
                          {userNote && (
                            <Icon
                              color={userNote.color || 'grey'}
                              name={userNote.icon || 'sticky note'}
                              style={{ marginLeft: '8px' }}
                              title={userNote.note}
                            />
                          )}
                          <UserNoteModal
                            onClose={() => this.fetchUserNote(username)}
                            trigger={
                              <Icon
                                color="grey"
                                link
                                name="pencil alternate"
                                size="small"
                                style={{ marginLeft: '4px', opacity: 0.5 }}
                              />
                            }
                            username={username}
                          />
                        </span>
                        <Icon
                          link
                          name="refresh"
                          onClick={this.handleRefresh}
                          title="Refresh user's file list"
                        />
                      </Card.Header>
                      <Card.Meta className="browse-meta">
                        {`${info.directories} directories, ${info.files} files`}
                        {info.lockedDirectories
                          ? ` (${info.lockedDirectories} locked directories, ${info.lockedFiles} locked files)`
                          : ''}
                      </Card.Meta>
                    </Card.Content>
                    <Card.Content>
                      <Segment className="browse-folderlist">
                        <DirectoryTree
                          downloadPending={downloadPending}
                          onDownload={this.handleDownloadDirectory}
                          onSelect={(_, value) => this.selectDirectory(value)}
                          selectedDirectoryName={name}
                          tree={tree}
                        />
                      </Segment>
                    </Card.Content>
                  </Card>
                )}
                {name && (
                  <Directory
                    files={files}
                    locked={locked}
                    marginTop={-20}
                    name={name}
                    onClose={this.handleDeselectDirectory}
                    username={username}
                  />
                )}
              </div>
            )}
          </div>
        )}
      </div>
    );
  }
}

export default BrowseSession;
