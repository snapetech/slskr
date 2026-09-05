import * as collectionsAPI from '../../lib/collections';
import { toDisplayError } from '../../lib/errors';
import { readOptionalApiResponse } from '../../lib/optionalApi';
import PlayCollectionItemButton from '../Player/PlayCollectionItemButton';
import ErrorSegment from '../Shared/ErrorSegment';
import LoaderSegment from '../Shared/LoaderSegment';
import React, { Component } from 'react';
import {
  Button,
  Container,
  Dropdown,
  Form,
  Header,
  Icon,
  Message,
  Modal,
  Popup,
  Segment,
  Table,
} from 'semantic-ui-react';

export default class Collections extends Component {
  constructor(props) {
    super(props);

    this.state = {
      addItemModalOpen: false,
      addingItem: false,
      collections: [],
      creatingCollection: false,
      creatingShare: false,
      createModalOpen: false,
      deletingCollectionId: null,
      error: null,
      itemSearchLoading: false,
      itemSearchQuery: '',
      itemSearchResults: [],
      loading: true,
      newCollectionDescription: '',
      newCollectionTitle: '',
      newCollectionType: 'Playlist',
      selectedCollection: null,
      selectedCollectionItems: [],
      removingItemId: null,
      shareAllowDownload: true,
      shareAllowStream: true,
      shareAudienceId: null,
      shareGroups: [],
      shareGroupsLoading: false,
      shareModalOpen: false,
      shares: [],
    };
    this.isMountedFlag = false;
    this.requestIds = {
      addItem: 0,
      collectionItems: 0,
      collections: 0,
      createCollection: 0,
      createShare: 0,
      deleteCollection: 0,
      removeItem: 0,
      search: 0,
      shareGroups: 0,
      shares: 0,
      selection: 0,
    };
  }

  componentDidMount() {
    this.isMountedFlag = true;
    void this.loadData();
    void this.loadShareGroups();
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    Object.keys(this.requestIds).forEach((key) => {
      this.requestIds[key] += 1;
    });
  }

  loadData = async () => {
    const requestId = ++this.requestIds.collections;
    try {
      if (this.isMountedFlag && requestId === this.requestIds.collections) {
        this.setState({ error: null, loading: true });
      }
      const response = await readOptionalApiResponse(() => collectionsAPI.getCollections());
      if (this.isMountedFlag && requestId === this.requestIds.collections) {
        this.setState({
          collections: Array.isArray(response.data) ? response.data : [],
          loading: false,
        });
      }
    } catch (error) {
      const errorMessage = toDisplayError(error, 'Failed to load collections');

      if (this.isMountedFlag && requestId === this.requestIds.collections) {
        this.setState({
          error: errorMessage,
          loading: false,
        });
      }
    }
  };

  loadShareGroups = async () => {
    const requestId = ++this.requestIds.shareGroups;
    try {
      if (this.isMountedFlag && requestId === this.requestIds.shareGroups) {
        this.setState({ shareGroupsLoading: true });
      }
      const response = await readOptionalApiResponse(() => collectionsAPI.getShareGroups());
      const shareGroups = Array.isArray(response.data) ? response.data : [];
      if (this.isMountedFlag && requestId === this.requestIds.shareGroups) {
        this.setState((previousState) => ({
          shareAudienceId:
            previousState.shareAudienceId ?? shareGroups[0]?.id ?? null,
          shareGroups,
          shareGroupsLoading: false,
        }));
      }
    } catch (error) {
      if (this.isMountedFlag && requestId === this.requestIds.shareGroups) {
        this.setState({
          error: toDisplayError(error, 'Failed to load share groups'),
          shareGroupsLoading: false,
        });
      }
    }
  };

  loadShares = async (collectionId) => {
    const requestId = ++this.requestIds.shares;
    try {
      if (!collectionId) {
        if (this.isMountedFlag && requestId === this.requestIds.shares) {
          this.setState({ shares: [] });
        }
        return;
      }

      const response = await readOptionalApiResponse(
        () => collectionsAPI.getSharesByCollection(collectionId),
      );
      if (this.isMountedFlag && requestId === this.requestIds.shares) {
        this.setState({
          shares: Array.isArray(response.data) ? response.data : [],
        });
      }
    } catch (error) {
      if (this.isMountedFlag && requestId === this.requestIds.shares) {
        this.setState({
          error: toDisplayError(error, 'Failed to load collection shares'),
          shares: [],
        });
      }
    }
  };

  loadCollectionItems = async (collectionId) => {
    const requestId = ++this.requestIds.collectionItems;
    try {
      const response = await collectionsAPI.getCollectionItems(collectionId);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.collectionItems
      ) {
        this.setState({
          selectedCollectionItems: Array.isArray(response.data)
            ? response.data
            : [],
        });
      }
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.collectionItems
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to load collection items'),
          selectedCollectionItems: [],
        });
      }
    }
  };

  handleCreateCollection = async () => {
    if (!this.isMountedFlag || this.state.creatingCollection) return;
    const requestId = ++this.requestIds.createCollection;
    this.setState({ creatingCollection: true, error: null });

    try {
      await collectionsAPI.createCollection({
        description: this.state.newCollectionDescription || undefined,
        title: this.state.newCollectionTitle,
        type: this.state.newCollectionType,
      });
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.createCollection
      ) {
        return;
      }
      this.setState({
        createModalOpen: false,
        error: null,
        newCollectionDescription: '',
        newCollectionTitle: '',
        newCollectionType: 'Playlist',
      });
      await this.loadData();
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.createCollection
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to create collection'),
        });
      }
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.createCollection
      ) {
        this.setState({ creatingCollection: false });
      }
    }
  };

  handleDeleteCollection = async (id) => {
    if (
      !this.isMountedFlag ||
      this.state.deletingCollectionId !== null
    ) {
      return;
    }
    // eslint-disable-next-line no-alert
    if (!window.confirm('Delete this collection?')) return;
    const requestId = ++this.requestIds.deleteCollection;
    this.setState({ deletingCollectionId: id, error: null });

    try {
      await collectionsAPI.deleteCollection(id);
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.deleteCollection
      ) {
        return;
      }
      await this.loadData();
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.deleteCollection &&
        this.state.selectedCollection?.id === id
      ) {
        this.setState({
          selectedCollection: null,
          selectedCollectionItems: [],
        });
      }
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.deleteCollection
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to delete collection'),
        });
      }
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.deleteCollection
      ) {
        this.setState({ deletingCollectionId: null });
      }
    }
  };

  handleSelectCollection = async (collection) => {
    const selectionId = ++this.requestIds.selection;
    if (!this.isMountedFlag) return;
    this.setState({
      error: null,
      selectedCollection: collection,
      selectedCollectionItems: [],
      shares: [],
    });
    await this.loadCollectionItems(collection.id);
    if (
      !this.isMountedFlag ||
      selectionId !== this.requestIds.selection
    ) {
      return;
    }
    await this.loadShares(collection.id);
    if (
      this.isMountedFlag &&
      selectionId === this.requestIds.selection &&
      this.state.selectedCollection?.id === collection.id
    ) {
      this.setState({ error: null });
    }
  };

  handleSearchItems = async (query) => {
    const requestId = ++this.requestIds.search;
    if (!query || query.length < 2) {
      if (this.isMountedFlag && requestId === this.requestIds.search) {
        this.setState({ itemSearchLoading: false, itemSearchResults: [] });
      }
      return;
    }

    if (this.isMountedFlag && requestId === this.requestIds.search) {
      this.setState({ itemSearchLoading: true });
    }
    try {
      const response = await collectionsAPI.searchLibraryItems(query, null, 20);
      const items = Array.isArray(response.data?.items)
        ? response.data.items
        : [];
      if (this.isMountedFlag && requestId === this.requestIds.search) {
        this.setState({
          itemSearchLoading: false,
          itemSearchResults: items,
        });
      }
    } catch (error) {
      console.error('[Collections] Search error:', error);
      if (this.isMountedFlag && requestId === this.requestIds.search) {
        this.setState({
          error: toDisplayError(error, 'Failed to search library items'),
          itemSearchLoading: false,
          itemSearchResults: [],
        });
      }
    }
  };

  handleAddItem = async () => {
    const selectedCollection = this.state.selectedCollection;
    if (!selectedCollection || this.state.addingItem) return;

    const requestId = ++this.requestIds.addItem;
    const itemSearchQuery = this.state.itemSearchQuery.trim();

    // Use selected search result if available, otherwise use query as fallback
    const selectedResult = this.state.itemSearchResults.find(
      (item) => item.contentId === itemSearchQuery,
    );
    const contentId = selectedResult
      ? selectedResult.contentId
      : itemSearchQuery;
    const mediaKind = selectedResult ? selectedResult.mediaKind : 'File'; // Default fallback

    if (!contentId) return;
    this.setState({ addingItem: true, error: null });

    try {
      await collectionsAPI.addCollectionItem(selectedCollection.id, {
        bytes: selectedResult?.bytes,
        contentId,
        fileName: selectedResult?.fileName,
        mediaKind,
        sha256: selectedResult?.sha256,
      });
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.addItem ||
        this.state.selectedCollection?.id !== selectedCollection.id
      ) {
        return;
      }
      this.setState({
        addItemModalOpen: false,
        error: null,
        itemSearchQuery: '',
        itemSearchResults: [],
      });
      await this.loadCollectionItems(selectedCollection.id);
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.addItem
      ) {
        this.setState({ error: toDisplayError(error, 'Failed to add item') });
      }
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.addItem
      ) {
        this.setState({ addingItem: false });
      }
    }
  };

  handleRemoveItem = async (itemId) => {
    const selectedCollection = this.state.selectedCollection;
    if (
      !selectedCollection ||
      !this.isMountedFlag ||
      this.state.removingItemId !== null
    ) {
      return;
    }

    const requestId = ++this.requestIds.removeItem;
    this.setState({ error: null, removingItemId: itemId });

    try {
      await collectionsAPI.removeCollectionItem(itemId);
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.removeItem ||
        this.state.selectedCollection?.id !== selectedCollection.id
      ) {
        return;
      }
      await this.loadCollectionItems(selectedCollection.id);
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.removeItem
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to remove collection item'),
        });
      }
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.removeItem
      ) {
        this.setState({ removingItemId: null });
      }
    }
  };

  handleOpenShareModal = () => {
    const { shareGroups } = this.state;
    this.setState({
      shareAllowDownload: true,
      shareAllowStream: true,
      shareAudienceId: shareGroups[0]?.id ?? null,
      shareModalOpen: true,
    });
  };

  handleCreateShare = async () => {
    const {
      selectedCollection,
      shareAllowDownload,
      shareAllowStream,
      shareAudienceId,
      shareGroups,
    } = this.state;
    if (
      !selectedCollection ||
      !shareAudienceId ||
      this.state.creatingShare
    ) {
      return;
    }

    // The backend has no "share with a group" primitive: /api/share-grants
    // only ever grants one Soulseek username access to one collection. A
    // "ShareGroup" audience is resolved client-side into its member
    // usernames and fanned out into one grant per member.
    const audienceGroup = shareGroups.find(
      (group) => String(group.id) === String(shareAudienceId),
    );
    const usernames = [
      ...new Set(
        (audienceGroup?.members ?? [])
          .map((member) =>
            typeof member?.username === 'string'
              ? member.username.trim()
              : '',
          )
          .filter(Boolean),
      ),
    ];
    if (usernames.length === 0) {
      this.setState({ error: 'Selected group has no members to share with.' });
      return;
    }

    // /api/share-grants only accepts collection_id + username at creation;
    // the permission tokens it recognizes (download, stream) are only
    // settable via a follow-up PUT.
    const permissions = [
      shareAllowDownload && 'download',
      shareAllowStream && 'stream',
    ].filter(Boolean).join(',');

    const requestId = ++this.requestIds.createShare;
    this.setState({ creatingShare: true, error: null });

    try {
      const created = await Promise.all(
        usernames.map((username) =>
          collectionsAPI.createShare({
            collectionId: selectedCollection.id,
            username,
          })),
      );
      if (permissions) {
        const shareIds = created
          .map((response) => response?.data?.id)
          .filter(Boolean);
        if (shareIds.length !== created.length) {
          throw new Error('Share creation returned an invalid grant');
        }
        await Promise.all(
          created.map((response) =>
            collectionsAPI.updateShare(response.data.id, { permissions })),
        );
      }
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.createShare ||
        this.state.selectedCollection?.id !== selectedCollection.id
      ) {
        return;
      }
      this.setState({ error: null, shareModalOpen: false });
      await this.loadShares(selectedCollection.id);
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.createShare
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to share collection'),
        });
      }
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.createShare
      ) {
        this.setState({ creatingShare: false });
      }
    }
  };

  render() {
    const {
      addItemModalOpen,
      addingItem,
      collections,
      creatingCollection,
      creatingShare,
      createModalOpen,
      deletingCollectionId,
      error,
      itemSearchLoading,
      itemSearchQuery,
      itemSearchResults,
      loading,
      newCollectionDescription,
      newCollectionTitle,
      newCollectionType,
      selectedCollection,
      selectedCollectionItems,
      removingItemId,
      shareAllowDownload,
      shareAllowStream,
      shareAudienceId,
      shareGroups,
      shareGroupsLoading,
      shareModalOpen,
      shares,
    } = this.state;

    if (loading) return <LoaderSegment />;

    const typeOptions = [
      { key: 'Playlist', text: 'Playlist', value: 'Playlist' },
      { key: 'ShareList', text: 'Share List', value: 'ShareList' },
    ];

    const collectionShares = shares;

    return (
      <div data-testid="collections-root">
        <Container>
          <Header as="h1">
            <Icon name="list" />
            <Header.Content>
              Collections
              <Header.Subheader>
                Manage your playlists and share lists
              </Header.Subheader>
            </Header.Content>
          </Header>

          {error && <ErrorSegment caption={error} />}

          <div style={{ marginBottom: '1em' }}>
            <Button
              data-testid="collections-create"
              onClick={() => this.setState({ createModalOpen: true })}
              primary
            >
              <Icon name="plus" />
              Create Collection
            </Button>
          </div>

          {collections.length === 0 ? (
            <Segment placeholder>
              <Header icon>
                <Icon name="list" />
                No collections yet
              </Header>
              <Button
                data-testid="collections-create-empty"
                onClick={() => this.setState({ createModalOpen: true })}
                primary
              >
                Create Collection
              </Button>
            </Segment>
          ) : (
            <Table celled>
              <Table.Header>
                <Table.Row>
                  <Table.HeaderCell>Title</Table.HeaderCell>
                  <Table.HeaderCell>Type</Table.HeaderCell>
                  <Table.HeaderCell>Items</Table.HeaderCell>
                  <Table.HeaderCell>Actions</Table.HeaderCell>
                </Table.Row>
              </Table.Header>
              <Table.Body>
                {collections.map((collection) => (
                  <Table.Row
                    data-testid={`collection-row-${collection.title}`}
                    key={collection.id}
                    onClick={() => this.handleSelectCollection(collection)}
                    style={{ cursor: 'pointer' }}
                  >
                    <Table.Cell>{collection.title}</Table.Cell>
                    <Table.Cell>{collection.type || 'Playlist'}</Table.Cell>
                    <Table.Cell>{collection.itemCount || 0}</Table.Cell>
                    <Table.Cell>
                      <Button
                        negative
                        onClick={(event) => {
                          event.stopPropagation();
                          this.handleDeleteCollection(collection.id);
                        }}
                        loading={deletingCollectionId === collection.id}
                        disabled={deletingCollectionId !== null}
                        size="small"
                      >
                        Delete
                      </Button>
                    </Table.Cell>
                  </Table.Row>
                ))}
              </Table.Body>
            </Table>
          )}

          {selectedCollection && (
            <Segment style={{ marginTop: '2em' }}>
              <Header as="h2">
                {selectedCollection.title}
                <Header.Subheader>
                  {selectedCollection.type || 'Playlist'}
                </Header.Subheader>
              </Header>

              <div style={{ marginBottom: '1em' }}>
                <Button
                  data-testid="collection-add-item"
                  onClick={() => this.setState({ addItemModalOpen: true })}
                  primary
                >
                  <Icon name="plus" />
                  Add Item
                </Button>
                <Button
                  data-testid="share-create"
                  onClick={this.handleOpenShareModal}
                  style={{ marginLeft: '0.5em' }}
                >
                  <Icon name="share alternate" />
                  Share Collection
                </Button>
              </div>

              <div data-testid="collection-items-table">
                {selectedCollectionItems.length === 0 ? (
                  <Message info>No items in this collection yet.</Message>
                ) : (
                  <Table>
                    <Table.Header>
                      <Table.Row>
                        <Table.HeaderCell>File Name</Table.HeaderCell>
                        <Table.HeaderCell>Media Kind</Table.HeaderCell>
                        <Table.HeaderCell>Actions</Table.HeaderCell>
                      </Table.Row>
                    </Table.Header>
                    <Table.Body>
                      {selectedCollectionItems.map((item, index) => (
                        <Table.Row
                          data-testid={`collection-item-row-${index}`}
                          key={item.id}
                        >
                          <Table.Cell>
                            {item.fileName || item.contentId || 'N/A'}
                          </Table.Cell>
                          <Table.Cell>{item.mediaKind || 'Unknown'}</Table.Cell>
                          <Table.Cell>
                            <PlayCollectionItemButton item={item} />
                            <Popup
                              content="Remove this item from the collection without deleting the shared file."
                              trigger={
                                <Button
                                  data-testid={`collection-item-remove-${index}`}
                                  negative
                                  disabled={removingItemId !== null}
                                  loading={removingItemId === item.id}
                                  onClick={() => this.handleRemoveItem(item.id)}
                                  size="small"
                                >
                                  Remove
                                </Button>
                              }
                            />
                          </Table.Cell>
                        </Table.Row>
                      ))}
                    </Table.Body>
                  </Table>
                )}
              </div>

              <Segment
                data-testid="shares-list"
                style={{ marginTop: '1em' }}
              >
                <Header as="h4">Shares</Header>
                {collectionShares.length === 0 ? (
                  <Message info>No shares yet.</Message>
                ) : (
                  <Table>
                    <Table.Header>
                      <Table.Row>
                        <Table.HeaderCell>Collection</Table.HeaderCell>
                        <Table.HeaderCell>Audience</Table.HeaderCell>
                        <Table.HeaderCell>Stream</Table.HeaderCell>
                        <Table.HeaderCell>Download</Table.HeaderCell>
                      </Table.Row>
                    </Table.Header>
                    <Table.Body>
                      {collectionShares.map((share) => (
                        <Table.Row key={share.id}>
                          <Table.Cell>
                            {share.collection?.title ||
                              selectedCollection.title}
                          </Table.Cell>
                          <Table.Cell>{share.username}</Table.Cell>
                          <Table.Cell>
                            {collectionsAPI.shareGrantAllows(share.permissions, 'stream') ? 'Yes' : 'No'}
                          </Table.Cell>
                          <Table.Cell>
                            {collectionsAPI.shareGrantAllows(share.permissions, 'download') ? 'Yes' : 'No'}
                          </Table.Cell>
                        </Table.Row>
                      ))}
                    </Table.Body>
                  </Table>
                )}
              </Segment>
            </Segment>
          )}

          {/* Create Collection Modal */}
          <Modal
            onClose={() =>
              this.setState({
                createModalOpen: false,
                newCollectionDescription: '',
                newCollectionTitle: '',
                newCollectionType: 'Playlist',
              })
            }
            open={createModalOpen}
          >
            <Modal.Header>Create Collection</Modal.Header>
            <Modal.Content>
              <Form>
                <Form.Field>
                  <label htmlFor="collection-type">Type</label>
                  <Dropdown
                    data-testid="collections-type-select"
                    id="collection-type"
                    onChange={(event, { value }) =>
                      this.setState({ newCollectionType: value })
                    }
                    options={typeOptions}
                    selection
                    value={newCollectionType}
                  />
                </Form.Field>
                <Form.Input
                  data-testid="collections-title-input"
                  label="Title"
                  onChange={(event) =>
                    this.setState({ newCollectionTitle: event.target.value })
                  }
                  placeholder="Enter collection title"
                  value={newCollectionTitle}
                />
                <Form.TextArea
                  label="Description"
                  onChange={(event) =>
                    this.setState({
                      newCollectionDescription: event.target.value,
                    })
                  }
                  placeholder="Optional description"
                  value={newCollectionDescription}
                />
              </Form>
            </Modal.Content>
            <Modal.Actions>
              <Button
                onClick={() =>
                  this.setState({
                    createModalOpen: false,
                    newCollectionDescription: '',
                    newCollectionTitle: '',
                    newCollectionType: 'Playlist',
                  })
                }
              >
                Cancel
              </Button>
              <Button
                data-testid="collections-create-submit"
                disabled={!newCollectionTitle.trim() || creatingCollection}
                loading={creatingCollection}
                onClick={this.handleCreateCollection}
                primary
              >
                Create
              </Button>
            </Modal.Actions>
          </Modal>

          {/* Share Collection Modal */}
          <Modal
            onClose={() => this.setState({ shareModalOpen: false })}
            open={shareModalOpen}
          >
            <Modal.Header>Share Collection</Modal.Header>
            <Modal.Content>
              {shareGroupsLoading ? (
                <LoaderSegment />
              ) : shareGroups.length === 0 ? (
                <Message warning>No share groups available.</Message>
              ) : (
                <Form>
                  <Form.Field>
                    <label htmlFor="share-group">Share Group</label>
                    <Dropdown
                      data-testid="share-audience-picker"
                      id="share-group"
                      onChange={(event, { value }) =>
                        this.setState({ shareAudienceId: value })
                      }
                      options={shareGroups.map((group) => ({
                        key: group.id,
                        text: group.name,
                        value: group.id,
                      }))}
                      selection
                      value={shareAudienceId}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label htmlFor="share-allow-stream">Allow streaming</label>
                    <input
                      checked={shareAllowStream}
                      data-testid="share-policy-stream"
                      id="share-allow-stream"
                      onChange={(event) =>
                        this.setState({
                          shareAllowStream: event.target.checked,
                        })
                      }
                      type="checkbox"
                    />
                  </Form.Field>
                  <Form.Field>
                    <label htmlFor="share-allow-download">Allow download</label>
                    <input
                      checked={shareAllowDownload}
                      data-testid="share-policy-download"
                      id="share-allow-download"
                      onChange={(event) =>
                        this.setState({
                          shareAllowDownload: event.target.checked,
                        })
                      }
                      type="checkbox"
                    />
                  </Form.Field>
                </Form>
              )}
            </Modal.Content>
            <Modal.Actions>
              <Button
                disabled={creatingShare}
                onClick={() => this.setState({ shareModalOpen: false })}
              >
                Cancel
              </Button>
              <Button
                data-testid="share-create-submit"
                disabled={!shareAudienceId || creatingShare}
                loading={creatingShare}
                onClick={this.handleCreateShare}
                primary
              >
                Share
              </Button>
            </Modal.Actions>
          </Modal>

          {/* Add Item Modal */}
          <Modal
            onClose={() =>
              this.setState({
                addItemModalOpen: false,
                itemSearchQuery: '',
                itemSearchResults: [],
              })
            }
            open={addItemModalOpen}
          >
            <Modal.Header>Add Item to {selectedCollection?.title}</Modal.Header>
            <Modal.Content>
              <Form>
                <Form.Field>
                  <label htmlFor="collection-item-search">
                    Search for item
                  </label>
                  <Form.Input
                    data-testid="collection-item-search-input"
                    id="collection-item-search"
                    label="Search for item"
                    loading={itemSearchLoading}
                    onChange={(event) => {
                      const query = event.target.value;
                      this.setState({ itemSearchQuery: query });
                      this.handleSearchItems(query);
                    }}
                    placeholder="Search by filename (e.g., sintel, aria, treasure)..."
                    value={itemSearchQuery}
                  />
                </Form.Field>
                {itemSearchResults.length > 0 && (
                  <Form.Field>
                    <label htmlFor="collection-item-results">
                      Search Results
                    </label>
                    <Dropdown
                      data-testid="collection-item-results"
                      fluid
                      id="collection-item-results"
                      onChange={(event, { value }) => {
                        this.setState({ itemSearchQuery: value });
                      }}
                      options={itemSearchResults.map((item, index) => ({
                        key: item.contentId || index,
                        text: `${item.fileName || item.path} (${item.mediaKind || 'File'})`,
                        value: item.contentId,
                      }))}
                      placeholder="Select an item from search results"
                      search
                      selection
                    />
                  </Form.Field>
                )}
                {itemSearchQuery &&
                  itemSearchResults.length === 0 &&
                  !itemSearchLoading && (
                    <Message info>
                      No results found. You can still add the search query as a
                      content ID.
                    </Message>
                  )}
              </Form>
            </Modal.Content>
            <Modal.Actions>
              <Button
                onClick={() =>
                  this.setState({
                    addItemModalOpen: false,
                    itemSearchQuery: '',
                    itemSearchResults: [],
                  })
                }
              >
                Cancel
              </Button>
              <Button
                data-testid="collection-add-item-submit"
                disabled={!itemSearchQuery.trim() || addingItem}
                loading={addingItem}
                onClick={this.handleAddItem}
                primary
              >
                Add Item
              </Button>
            </Modal.Actions>
          </Modal>
        </Container>
      </div>
    );
  }
}
