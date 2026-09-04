import * as collectionsAPI from '../../lib/collections';
import { toDisplayError } from '../../lib/errors';
import ErrorSegment from '../Shared/ErrorSegment';
import LoaderSegment from '../Shared/LoaderSegment';
import React, { Component } from 'react';
import { safeOpenBlank } from '../../lib/safeOpen';
import { toast } from 'react-toastify';
import {
  Button,
  Container,
  Header,
  Icon,
  Label,
  Modal,
  Segment,
  Table,
} from 'semantic-ui-react';

const isAuthOrFeatureErrorStatus = (status) =>
  status === 401 || status === 403 || status === 404 || status === 400;

export default class SharedWithMe extends Component {
  state = {
    backfilling: false,
    backfillResult: null,
    error: null,
    loading: true,
    manifest: null,
    manifestLoading: false,
    manifestModalOpen: false,
    selectedShare: null,
    shares: [],
  };

  isMountedFlag = false;

  requestIds = {
    backfill: 0,
    load: 0,
    manifest: 0,
    stream: 0,
  };

  componentDidMount() {
    this.isMountedFlag = true;
    void this.loadData();
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    Object.keys(this.requestIds).forEach((key) => {
      this.requestIds[key] += 1;
    });
  }

  loadData = async () => {
    const requestId = ++this.requestIds.load;
    try {
      if (this.isMountedFlag && requestId === this.requestIds.load) {
        this.setState({ error: null, loading: true });
      }
      const sharesRes = await collectionsAPI.getIncomingShares().catch((error) => {
        if (isAuthOrFeatureErrorStatus(error.response?.status)) {
          return { data: [] };
        }
        throw error;
      });
      if (this.isMountedFlag && requestId === this.requestIds.load) {
        this.setState({
          loading: false,
          shares: Array.isArray(sharesRes.data) ? sharesRes.data : [],
        });
      }
    } catch (error) {
      if (this.isMountedFlag && requestId === this.requestIds.load) {
        this.setState({
          error: isAuthOrFeatureErrorStatus(error.response?.status)
            ? null
            : toDisplayError(error),
          loading: false,
        });
      }
    }
  };

  handleViewManifest = async (share) => {
    if (!this.isMountedFlag) return;
    const requestId = ++this.requestIds.manifest;
    this.setState({
      manifest: null,
      manifestLoading: true,
      manifestModalOpen: true,
      selectedShare: share,
    });
    try {
      // Fetched live from the owner's own node — it enforces the token's
      // current validity (including expiry) and its up-to-date permissions,
      // rather than trusting what was true when the share was announced.
      const manifest = await collectionsAPI.fetchRemoteShareManifest(
        share.ownerEndpoint,
        share.shareGrantId,
        share.token,
      );
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.manifest
      ) {
        this.setState({
          manifest: {
            ...manifest,
            items: Array.isArray(manifest?.items) ? manifest.items : [],
          },
          manifestLoading: false,
        });
      }
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.manifest
      ) {
        this.setState({
          error: toDisplayError(error, 'Failed to load manifest'),
          manifestLoading: false,
        });
      }
    }
  };

  handleStreamItem = async (contentId) => {
    const { selectedShare } = this.state;
    if (!selectedShare || !this.isMountedFlag) return;
    const requestId = ++this.requestIds.stream;
    const selectedShareId = selectedShare.id || selectedShare.shareGrantId;

    try {
      const ticket = await collectionsAPI.createRemoteShareStreamTicket(
        selectedShare.ownerEndpoint,
        contentId,
        selectedShare.token,
      );
      if (!ticket) throw new Error('Stream ticket missing from response');
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.stream ||
        (this.state.selectedShare?.id || this.state.selectedShare?.shareGrantId) !== selectedShareId
      ) {
        return;
      }
      safeOpenBlank(
        collectionsAPI.buildRemoteShareStreamUrl(
          selectedShare.ownerEndpoint,
          contentId,
          ticket,
        ),
      );
    } catch (error) {
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.stream
      ) {
        return;
      }
      const message = toDisplayError(error, 'Failed to start stream');
      this.setState({ error: message });
      toast.error(message);
    }
  };

  handleBackfill = async () => {
    const { selectedShare } = this.state;
    if (!selectedShare || !this.isMountedFlag) return;
    const requestId = ++this.requestIds.backfill;
    const selectedShareId = selectedShare.id || selectedShare.shareGrantId;

    try {
      this.setState({ backfilling: true, backfillResult: null, error: null });
      const result = await collectionsAPI.remoteBackfillShare(
        selectedShare.ownerEndpoint,
        selectedShare.shareGrantId,
      );
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.backfill ||
        (this.state.selectedShare?.id || this.state.selectedShare?.shareGrantId) !== selectedShareId
      ) {
        return;
      }
      this.setState({ backfilling: false, backfillResult: result });
      toast.success(result?.message || 'Backfill requested');
    } catch (error) {
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.backfill
      ) {
        return;
      }
      const errorMessage = toDisplayError(error, 'Failed to start backfill');
      this.setState({
        backfilling: false,
        backfillResult: null,
        error: errorMessage,
      });
      toast.error(errorMessage);
    }
  };

  render() {
    const {
      error,
      loading,
      manifest,
      manifestLoading,
      manifestModalOpen,
      selectedShare,
      shares,
    } = this.state;

    if (loading) return <LoaderSegment />;

    return (
      <Container>
        <Header as="h1">
          <Icon name="share" />
          <Header.Content>
            Shared with Me
            <Header.Subheader>Collections shared with you</Header.Subheader>
          </Header.Content>
        </Header>

        {error && <ErrorSegment caption={error} />}

        {shares.length === 0 ? (
          <Segment placeholder>
            <Header icon>
              <Icon name="inbox" />
              No shares yet
            </Header>
            <p>Collections shared with you will appear here.</p>
          </Segment>
        ) : (
          <Table>
            <Table.Header>
              <Table.Row>
                <Table.HeaderCell>Collection</Table.HeaderCell>
                <Table.HeaderCell>Shared By</Table.HeaderCell>
                <Table.HeaderCell>Type</Table.HeaderCell>
                <Table.HeaderCell>Permissions</Table.HeaderCell>
                <Table.HeaderCell>Actions</Table.HeaderCell>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {shares.map((share) => (
                <Table.Row
                  data-testid={`incoming-share-row-${share.collectionTitle || 'Untitled'}`}
                  key={share.id}
                >
                  <Table.Cell>
                    <strong>{share.collectionTitle || 'Untitled'}</strong>
                    {share.collectionDescription && (
                      <div
                        style={{
                          color: '#666',
                          fontSize: '0.9em',
                          marginTop: '0.25em',
                        }}
                      >
                        {share.collectionDescription}
                      </div>
                    )}
                  </Table.Cell>
                  <Table.Cell>{share.ownerUserId || 'Unknown'}</Table.Cell>
                  <Table.Cell>{share.collectionType || 'ShareList'}</Table.Cell>
                  <Table.Cell>
                    {collectionsAPI.shareGrantAllows(share.permissions, 'stream') && (
                      <Label color="green">Stream</Label>
                    )}
                    {collectionsAPI.shareGrantAllows(share.permissions, 'download') && (
                      <Label color="blue">Download</Label>
                    )}
                    {collectionsAPI.shareGrantAllows(share.permissions, 'reshare') && (
                      <Label>Reshare</Label>
                    )}
                  </Table.Cell>
                  <Table.Cell>
                    <Button
                      data-testid="incoming-share-open"
                      onClick={() => this.handleViewManifest(share)}
                      primary
                      size="small"
                    >
                      View Contents
                    </Button>
                  </Table.Cell>
                </Table.Row>
              ))}
            </Table.Body>
          </Table>
        )}

        {/* Manifest Modal */}
        <Modal
          onClose={() => {
            this.requestIds.manifest += 1;
            this.requestIds.stream += 1;
            this.requestIds.backfill += 1;
            this.setState({
              manifest: null,
              manifestModalOpen: false,
              selectedShare: null,
            });
          }}
          open={manifestModalOpen}
          size="large"
        >
          <Modal.Header>
            {selectedShare?.collectionTitle ||
              manifest?.collection?.title ||
              'Collection Contents'}
          </Modal.Header>
          <Modal.Content>
            {manifestLoading ? (
              <LoaderSegment />
            ) : manifest ? (
              <div data-testid="shared-manifest">
                {manifest.collection?.description && (
                  <p style={{ marginBottom: '1em' }}>
                    {manifest.collection.description}
                  </p>
                )}
                {manifest.items && manifest.items.length > 0 ? (
                  <Table>
                    <Table.Header>
                      <Table.Row>
                        <Table.HeaderCell>Content ID</Table.HeaderCell>
                        <Table.HeaderCell>Media Kind</Table.HeaderCell>
                        <Table.HeaderCell>Actions</Table.HeaderCell>
                      </Table.Row>
                    </Table.Header>
                    <Table.Body>
                      {manifest.items.map((item, index) => {
                        // Extract sha256 prefix from contentId (format: "sha256:...")
                        const sha256Prefix = item.contentId?.startsWith(
                          'sha256:',
                        )
                          ? item.contentId.slice(7, 15) // First 8 chars of hash
                          : item.contentId?.slice(0, 8) || `item-${index}`;
                        return (
                          <Table.Row
                            data-testid={`incoming-item-row-${sha256Prefix}`}
                            key={index}
                          >
                            <Table.Cell>
                              <code style={{ fontSize: '0.85em' }}>
                                {item.fileName ||
                                  item.contentId?.slice(0, 32) ||
                                  'Unknown'}
                              </code>
                            </Table.Cell>
                            <Table.Cell>
                              {item.mediaKind || 'Unknown'}
                            </Table.Cell>
                            <Table.Cell>
                              {collectionsAPI.shareGrantAllows(
                                manifest.permissions,
                                'stream',
                              ) && (
                                <Button
                                  data-testid={`incoming-stream-${sha256Prefix}`}
                                  onClick={() =>
                                    this.handleStreamItem(item.contentId)
                                  }
                                  primary
                                  size="small"
                                >
                                  <Icon name="play" />
                                  Stream
                                </Button>
                              )}
                            </Table.Cell>
                          </Table.Row>
                        );
                      })}
                    </Table.Body>
                  </Table>
                ) : (
                  <Segment placeholder>
                    <Header icon>
                      <Icon name="file outline" />
                      No items in this collection
                    </Header>
                  </Segment>
                )}
              </div>
            ) : (
              <ErrorSegment error="Failed to load manifest" />
            )}
          </Modal.Content>
          <Modal.Actions>
            {collectionsAPI.shareGrantAllows(manifest?.permissions, 'download') && (
              <Button
                data-testid="incoming-backfill"
                disabled={this.state.backfilling}
                loading={this.state.backfilling}
                onClick={this.handleBackfill}
                primary
              >
                <Icon name="download" />
                Backfill All
              </Button>
            )}
            {this.state.backfillResult && (
              <span
                style={{ color: '#666', fontSize: '0.9em', marginRight: '1em' }}
              >
                {this.state.backfillResult.message ||
                  `${this.state.backfillResult.backfilled ?? 0} items`}
              </span>
            )}
            <Button
              onClick={() =>
                this.setState({
                  backfillResult: null,
                  manifest: null,
                  manifestModalOpen: false,
                  selectedShare: null,
                })
              }
            >
              Close
            </Button>
          </Modal.Actions>
        </Modal>
      </Container>
    );
  }
}
