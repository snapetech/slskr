import * as collectionsAPI from '../../lib/collections';
import { toDisplayError } from '../../lib/errors';
import { useMountedRef } from '../../lib/useMountedRef';
import React, { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Button,
  Header,
  Icon,
  Input,
  Message,
  Modal,
  Popup,
  Segment,
  Table,
} from 'semantic-ui-react';

const playerBrowserPageSize = 80;

const requireArrayData = (response, resource) => {
  if (!Array.isArray(response?.data)) {
    throw new Error(`Player API returned an invalid ${resource} response`);
  }

  return response.data;
};

const requireBrowserData = (response) => {
  const data = response?.data;
  if (
    !data ||
    typeof data !== 'object' ||
    Array.isArray(data) ||
    !Array.isArray(data.breadcrumbs) ||
    !Array.isArray(data.directories) ||
    !Array.isArray(data.files) ||
    typeof data.hasMore !== 'boolean'
  ) {
    throw new Error('Player API returned an invalid library browser response');
  }

  return data;
};

const getBrowserRequestKey = (path, query, offset) =>
  JSON.stringify([path, query, offset]);

const PlayerLauncher = ({ compact = false, onPlayItem }) => {
  const navigate = useNavigate();
  const mountedRef = useMountedRef();
  const collectionItemsRequestIdRef = useRef(0);
  const [collections, setCollections] = useState([]);
  const [collectionsError, setCollectionsError] = useState('');
  const [collectionsOpen, setCollectionsOpen] = useState(false);
  const [selectedCollection, setSelectedCollection] = useState(null);
  const [collectionItems, setCollectionItems] = useState([]);
  const [collectionItemsError, setCollectionItemsError] = useState('');
  const [collectionItemsLoading, setCollectionItemsLoading] = useState(false);
  const [items, setItems] = useState([]);
  const [browserDirectories, setBrowserDirectories] = useState([]);
  const [browserBreadcrumbs, setBrowserBreadcrumbs] = useState([]);
  const [browserHasMore, setBrowserHasMore] = useState(false);
  const [browserOffset, setBrowserOffset] = useState(0);
  const [browserPath, setBrowserPath] = useState('');
  const [browserError, setBrowserError] = useState('');
  const [browserStats, setBrowserStats] = useState({
    duplicatesRemoved: 0,
    totalDirectories: 0,
    totalFiles: 0,
  });
  const [browserLoadedKey, setBrowserLoadedKey] = useState(null);
  const [filesOpen, setFilesOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [itemsLoading, setItemsLoading] = useState(false);

  useEffect(() => {
    let canceled = false;
    collectionsAPI
      .getCollections()
      .then((response) => {
        if (!canceled && mountedRef.current) {
          setCollections(requireArrayData(response, 'collection list'));
          setCollectionsError('');
        }
      })
      .catch((error) => {
        if (!canceled && mountedRef.current) {
          setCollectionsError(
            toDisplayError(error, 'Failed to load collections'),
          );
        }
      });

    return () => {
      canceled = true;
    };
  }, [mountedRef]);

  useEffect(() => {
    if (!filesOpen) return undefined;

    const requestKey = getBrowserRequestKey(browserPath, query, browserOffset);

    if (query && query.length < 2) {
      setItems([]);
      setBrowserDirectories([]);
      setBrowserBreadcrumbs([]);
      setBrowserHasMore(false);
      setBrowserStats({ duplicatesRemoved: 0, totalDirectories: 0, totalFiles: 0 });
      setBrowserError('');
      setBrowserLoadedKey(requestKey);
      setItemsLoading(false);
      return undefined;
    }

    let canceled = false;
    const timeoutId = window.setTimeout(() => {
      setItemsLoading(true);
      collectionsAPI
        .browseLibraryItems({
          kinds: 'Audio',
          limit: playerBrowserPageSize,
          offset: browserOffset,
          path: browserPath,
          query,
        })
        .then((response) => {
          if (!canceled) {
            const data = requireBrowserData(response);
            setItems(data.files);
            setBrowserDirectories(data.directories);
            setBrowserBreadcrumbs(data.breadcrumbs);
            setBrowserHasMore(data.hasMore);
            setBrowserStats({
              duplicatesRemoved: data.duplicatesRemoved || 0,
              totalDirectories: data.totalDirectories || 0,
              totalFiles: data.totalFiles || 0,
            });
            setBrowserLoadedKey(requestKey);
            setBrowserError('');
          }
        })
        .catch((error) => {
          if (!canceled) {
            setBrowserError(
              toDisplayError(error, 'Failed to browse local audio'),
            );
          }
        })
        .finally(() => {
          if (!canceled) setItemsLoading(false);
        });
    }, query ? 200 : 0);

    return () => {
      canceled = true;
      window.clearTimeout(timeoutId);
    };
  }, [browserOffset, browserPath, filesOpen, query]);

  const selectCollection = (collection) => {
    if (!mountedRef.current) return;
    const requestId = ++collectionItemsRequestIdRef.current;
    setSelectedCollection(collection);
    setCollectionItemsError('');
    setCollectionItemsLoading(true);
    collectionsAPI
      .getCollectionItems(collection.id)
      .then((response) => {
        if (
          mountedRef.current &&
          collectionItemsRequestIdRef.current === requestId
        ) {
          setCollectionItems(requireArrayData(response, 'collection items'));
          setCollectionItemsError('');
        }
      })
      .catch((error) => {
        if (
          mountedRef.current &&
          collectionItemsRequestIdRef.current === requestId
        ) {
          setCollectionItemsError(
            toDisplayError(error, 'Failed to load collection items'),
          );
        }
      })
      .finally(() => {
        if (
          mountedRef.current &&
          collectionItemsRequestIdRef.current === requestId
        ) {
          setCollectionItemsLoading(false);
        }
      });
  };

  const openCollections = () => {
    if (!mountedRef.current) return;
    setCollectionsOpen(true);
  };

  const closeCollections = () => {
    collectionItemsRequestIdRef.current += 1;
    setCollectionsOpen(false);
  };

  const closeFileBrowser = () => {
    setBrowserLoadedKey(null);
    setItemsLoading(false);
    setFilesOpen(false);
  };

  const playAndClose = (item) => {
    onPlayItem(item);
    closeFileBrowser();
    closeCollections();
  };

  const openFileBrowser = () => {
    setBrowserLoadedKey(null);
    setBrowserError('');
    setItemsLoading(false);
    setBrowserOffset(0);
    setBrowserPath('');
    setFilesOpen(true);
    setQuery('');
  };

  const openBrowserPath = (path) => {
    setBrowserLoadedKey(null);
    setBrowserError('');
    setBrowserOffset(0);
    setBrowserPath(path || '');
    setQuery('');
  };

  const updateBrowserQuery = (value) => {
    setBrowserLoadedKey(null);
    setBrowserError('');
    setBrowserOffset(0);
    setQuery(value || '');
  };

  const updateBrowserOffset = (offset) => {
    setBrowserLoadedKey(null);
    setBrowserError('');
    setBrowserOffset(Math.max(0, offset));
  };

  const currentBrowserRequestKey = getBrowserRequestKey(
    browserPath,
    query,
    browserOffset,
  );
  const hasCurrentBrowserData = browserLoadedKey === currentBrowserRequestKey;
  const visibleBrowserItems = hasCurrentBrowserData ? items : [];
  const visibleBrowserDirectories = hasCurrentBrowserData
    ? browserDirectories
    : [];
  const visibleBrowserBreadcrumbs = hasCurrentBrowserData
    ? browserBreadcrumbs
    : [];
  const visibleBrowserStats = hasCurrentBrowserData
    ? browserStats
    : { duplicatesRemoved: 0, totalDirectories: 0, totalFiles: 0 };

  const shownFileCount = Math.min(
    browserOffset + visibleBrowserItems.length,
    visibleBrowserStats.totalFiles,
  );

  return (
    <div className="player-launcher">
      <Popup
        content="Browse your collections and play an item from a playlist or share list."
        trigger={
          <Button
            aria-label="Open collections browser"
            className="player-library-button"
            compact
            data-testid="player-open-collections-browser"
            icon
            labelPosition={compact ? undefined : 'left'}
            onClick={openCollections}
            size="small"
            title="Open collections browser"
          >
            <Icon name="list" />
            {compact ? null : 'Collections'}
          </Button>
        }
      />
      <Popup
        content="Browse shared and downloaded local audio that slskr can stream in this browser."
        trigger={
          <Button
            aria-label="Open local audio file browser"
            className="player-library-button"
            compact
            data-testid="player-open-file-browser"
            icon
            labelPosition={compact ? undefined : 'left'}
            onClick={openFileBrowser}
            size="small"
            title="Open local audio file browser"
          >
            <Icon name="folder open" />
            {compact ? null : 'Files'}
          </Button>
        }
      />

      <Modal
        className="player-browser-modal"
        data-testid="player-collection-browser-modal"
        onClose={closeCollections}
        open={collectionsOpen}
        size="large"
      >
        <Modal.Header>Choose from Collections</Modal.Header>
        <Modal.Content>
          <div className="player-browser-grid">
            <Segment className="player-browser-panel">
              <Header as="h4">Collections</Header>
              {collectionsError ? (
                <Message negative>{collectionsError}</Message>
              ) : collections.length === 0 ? (
                <Message info>No collections found.</Message>
              ) : (
                <Table compact selectable>
                  <Table.Body>
                    {collections.map((collection) => (
                      <Table.Row
                        active={selectedCollection?.id === collection.id}
                        data-testid={`player-collection-row-${collection.id}`}
                        key={collection.id}
                        onClick={() => selectCollection(collection)}
                      >
                        <Table.Cell>
                          <strong>{collection.title}</strong>
                          <div className="player-picker-meta">
                            {collection.type || 'Playlist'}
                          </div>
                        </Table.Cell>
                      </Table.Row>
                    ))}
                  </Table.Body>
                </Table>
              )}
            </Segment>
            <Segment className="player-browser-panel">
              <Header as="h4">
                {selectedCollection?.title || 'Collection Items'}
              </Header>
              {!selectedCollection ? (
                <Message info>Select a collection to see its tracks.</Message>
              ) : collectionItemsLoading ? (
                <Message info>Loading collection items...</Message>
              ) : collectionItemsError ? (
                <Message negative>{collectionItemsError}</Message>
              ) : collectionItems.length === 0 ? (
                <Message info>No playable items in this collection.</Message>
              ) : (
                <Table compact>
                  <Table.Header>
                    <Table.Row>
                      <Table.HeaderCell>Track</Table.HeaderCell>
                      <Table.HeaderCell collapsing>Action</Table.HeaderCell>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {collectionItems.map((item) => (
                      <Table.Row key={item.id || item.contentId}>
                        <Table.Cell>
                          <strong>
                            {item.fileName || item.title || item.contentId}
                          </strong>
                          <div className="player-picker-meta">
                            {item.mediaKind || 'Audio'}
                          </div>
                        </Table.Cell>
                        <Table.Cell collapsing>
                          <Popup
                            content="Play this collection item in the browser player."
                            trigger={
                              <Button
                                data-testid={`player-play-collection-item-${item.contentId}`}
                                icon
                                onClick={() => playAndClose(item)}
                                size="small"
                              >
                                <Icon name="play" />
                              </Button>
                            }
                          />
                        </Table.Cell>
                      </Table.Row>
                    ))}
                  </Table.Body>
                </Table>
              )}
            </Segment>
          </div>
        </Modal.Content>
        <Modal.Actions>
          <Popup
            content="Open the full Collections page to create, edit, or share collections."
            trigger={
              <Button
                data-testid="player-manage-collections"
                onClick={() => {
                  closeCollections();
                  navigate('/collections');
                }}
              >
                <Icon name="external alternate" />
                Manage Collections
              </Button>
            }
          />
          <Popup
            content="Close the collection picker without changing playback."
            trigger={
              <Button onClick={closeCollections}>Close</Button>
            }
          />
        </Modal.Actions>
      </Modal>

      <Modal
        className="player-browser-modal"
        data-testid="player-file-browser-modal"
        onClose={closeFileBrowser}
        open={filesOpen}
        size="fullscreen"
      >
        <Modal.Header>Browse Local Audio Library</Modal.Header>
        <Modal.Content>
          <div className="player-file-explorer">
            <div className="player-file-explorer-toolbar">
              <Input
                data-testid="player-file-browser-search"
                fluid
                icon="search"
                onChange={(_, { value }) => updateBrowserQuery(value)}
                placeholder="Search all audio by file, artist folder, album folder, or path"
                value={query}
              />
              <div className="player-file-explorer-counts">
                {itemsLoading
                  ? 'Loading...'
                  : `${shownFileCount} of ${visibleBrowserStats.totalFiles} tracks`}
                {visibleBrowserStats.duplicatesRemoved > 0
                  ? `, ${visibleBrowserStats.duplicatesRemoved} duplicates collapsed`
                  : ''}
              </div>
            </div>
            {browserError && <Message negative>{browserError}</Message>}

            <div className="player-file-explorer-breadcrumbs">
              {(visibleBrowserBreadcrumbs.length > 0
                ? visibleBrowserBreadcrumbs
                : [{ name: 'Library', path: '' }]).map((breadcrumb, index) => (
                  <React.Fragment key={breadcrumb.path || 'library'}>
                    {index > 0 ? <Icon name="angle right" /> : null}
                    <button
                      className="player-file-breadcrumb"
                      data-testid={`player-file-breadcrumb-${index}`}
                      onClick={() => openBrowserPath(breadcrumb.path)}
                      title={`Open ${breadcrumb.name}`}
                      type="button"
                    >
                      {breadcrumb.name}
                    </button>
                  </React.Fragment>
              ))}
            </div>

            <div className="player-file-explorer-body">
              <aside className="player-file-explorer-folders">
                <div className="player-file-explorer-section-title">
                  Folders
                </div>
                {!hasCurrentBrowserData ? (
                  browserError ? null : <Message info compact>Loading folders...</Message>
                ) : query ? (
                  <Message info compact>
                    Clear search to browse folders.
                  </Message>
                ) : visibleBrowserDirectories.length === 0 ? (
                  <Message info compact>
                    No child folders here.
                  </Message>
                ) : (
                  visibleBrowserDirectories.map((directory) => (
                    <button
                      className="player-file-folder-row"
                      data-testid={`player-file-folder-${directory.path}`}
                      key={directory.path}
                      onClick={() => openBrowserPath(directory.path)}
                      title={`Open ${directory.name}`}
                      type="button"
                    >
                      <Icon name="folder" />
                      <span>
                        <strong>{directory.name}</strong>
                        <small>
                          {directory.fileCount} tracks
                          {directory.childDirectoryCount
                            ? `, ${directory.childDirectoryCount} folders`
                            : ''}
                        </small>
                      </span>
                    </button>
                  ))
                )}
              </aside>

              <section className="player-file-explorer-files">
                <div className="player-file-explorer-section-title">
                  {query ? 'Search Results' : browserPath || 'Library Root'}
                </div>
                {itemsLoading ? (
                  <Message info>Loading audio files...</Message>
                ) : !hasCurrentBrowserData ? null : visibleBrowserItems.length === 0 ? (
                  <Message info>
                    {query && query.length < 2
                      ? 'Type at least two characters to search.'
                      : 'No local audio files found here.'}
                  </Message>
                ) : (
                  <Table compact selectable>
                    <Table.Header>
                      <Table.Row>
                        <Table.HeaderCell>Track</Table.HeaderCell>
                        <Table.HeaderCell>Location</Table.HeaderCell>
                        <Table.HeaderCell collapsing>Copies</Table.HeaderCell>
                        <Table.HeaderCell collapsing>Action</Table.HeaderCell>
                      </Table.Row>
                    </Table.Header>
                    <Table.Body>
                      {visibleBrowserItems.map((item) => (
                        <Table.Row
                          data-testid={`player-file-row-${item.contentId}`}
                          key={`${item.contentId}-${item.path}`}
                          onDoubleClick={() => playAndClose(item)}
                        >
                          <Table.Cell>
                            <strong>{item.fileName || item.contentId}</strong>
                            <div className="player-picker-meta">
                              {item.mediaKind || 'Audio'}
                              {item.bytes ? ` - ${Math.round(item.bytes / 1024 / 1024)} MB` : ''}
                            </div>
                          </Table.Cell>
                          <Table.Cell>
                            <span className="player-file-path">{item.path}</span>
                          </Table.Cell>
                          <Table.Cell collapsing>
                            {item.duplicateCount > 1 ? item.duplicateCount : ''}
                          </Table.Cell>
                          <Table.Cell collapsing>
                            <Popup
                              content="Play this local file in the browser player."
                              trigger={
                                <Button
                                  aria-label={`Play ${item.fileName || item.contentId}`}
                                  data-testid={`player-play-file-${item.contentId}`}
                                  icon
                                  onClick={() => playAndClose(item)}
                                  size="small"
                                  title={`Play ${item.fileName || item.contentId}`}
                                >
                                  <Icon name="play" />
                                </Button>
                              }
                            />
                          </Table.Cell>
                        </Table.Row>
                      ))}
                    </Table.Body>
                  </Table>
                )}
                <div className="player-file-explorer-pager">
                  <Popup
                    content="Move to the previous page of files in this folder or search."
                    trigger={
                      <Button
                        disabled={browserOffset === 0 || itemsLoading}
                        onClick={() => updateBrowserOffset(
                          browserOffset - playerBrowserPageSize,
                        )}
                        size="small"
                      >
                        <Icon name="angle left" />
                        Previous
                      </Button>
                    }
                  />
                  <Popup
                    content="Move to the next page of files in this folder or search."
                    trigger={
                      <Button
                        disabled={!hasCurrentBrowserData || !browserHasMore || itemsLoading}
                        onClick={() => updateBrowserOffset(
                          browserOffset + playerBrowserPageSize,
                        )}
                        size="small"
                      >
                        Next
                        <Icon name="angle right" />
                      </Button>
                    }
                  />
                </div>
              </section>
            </div>
          </div>
        </Modal.Content>
        <Modal.Actions>
          <Popup
            content="Close the local file browser without changing playback."
            trigger={<Button onClick={closeFileBrowser}>Close</Button>}
          />
        </Modal.Actions>
      </Modal>
    </div>
  );
};

export default PlayerLauncher;
