import * as identityAPI from '../../lib/identity';
import { toDisplayError } from '../../lib/errors';
import ErrorSegment from '../Shared/ErrorSegment';
import LoaderSegment from '../Shared/LoaderSegment';
import TooltipButton from '../Shared/TooltipButton';
import QRCode from 'qrcode';
import React, { Component } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Container,
  Header,
  Icon,
  Label,
  List,
  Message,
  Modal,
  Popup,
  Segment,
  Tab,
  Table,
} from 'semantic-ui-react';

const Button = TooltipButton;

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (item) => item && typeof item === 'object' && !Array.isArray(item),
  );

const normalizeContact = (contact, index) => ({
  ...contact,
  id: contact.id ?? `contact-${index}`,
  nickname: typeof contact.nickname === 'string' ? contact.nickname : '',
  peerId: String(contact.peerId ?? ''),
});

const normalizeNearbyPeer = (peer, index) => ({
  ...peer,
  displayName: String(peer.displayName ?? peer.peerId ?? 'Unknown peer'),
  endpoint: String(peer.endpoint ?? ''),
  peerCode: String(peer.peerCode ?? ''),
  peerId: String(peer.peerId ?? `nearby-${index}`),
});

const withNavigate = (WrappedComponent) => {
  const RoutedComponent = (props) => {
    const navigate = useNavigate();
    return (
      <WrappedComponent
        {...props}
        navigate={navigate}
      />
    );
  };

  RoutedComponent.displayName = `withNavigate(${WrappedComponent.displayName || WrappedComponent.name || 'Component'})`;
  return RoutedComponent;
};

class Contacts extends Component {
  state = {
    activeTab: 0,
    addFriendModalOpen: false,
    contacts: [],
    contactsError: null,
    createInviteModalOpen: false,
    error: null,
    inviteFriendCode: null,
    inviteLink: null,
    inviteQrDataUrl: null,
    loading: true,
    mutating: false,
    nearby: [],
    nearbyError: null,
    nearbyLoading: false,
  };

  constructor(props) {
    super(props);
    this.isMountedFlag = false;
    this.requestIds = {
      contacts: 0,
      invite: 0,
      nearby: 0,
    };
    this.mutationInFlight = false;
  }

  componentDidMount() {
    this.isMountedFlag = true;
    void this.loadContacts();
    void this.loadNearby();
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    Object.keys(this.requestIds).forEach((key) => {
      this.requestIds[key] += 1;
    });
  }

  loadContacts = async () => {
    const requestId = ++this.requestIds.contacts;
    try {
      if (this.isMountedFlag && requestId === this.requestIds.contacts) {
        this.setState({ contactsError: null, error: null, loading: true });
      }
      const response = await identityAPI.getContacts();
      if (this.isMountedFlag && requestId === this.requestIds.contacts) {
        this.setState({
          contacts: asRecords(response?.data).map(normalizeContact),
          contactsError: null,
          loading: false,
        });
      }
    } catch (error) {
      // A missing route is the only compatibility case. Authentication,
      // authorization, and server failures must remain visible to the user.
      if (error?.response?.status === 404) {
        if (this.isMountedFlag && requestId === this.requestIds.contacts) {
          this.setState({
            contacts: [],
            contactsError: null,
            error: null,
            loading: false,
          });
        }
      } else {
        if (this.isMountedFlag && requestId === this.requestIds.contacts) {
          const contactsError = toDisplayError(
            error,
            'Failed to load contacts',
          );
          this.setState({
            contactsError,
            error: contactsError,
            loading: false,
          });
        }
      }
    }
  };

  loadNearby = async () => {
    const requestId = ++this.requestIds.nearby;
    try {
      if (this.isMountedFlag && requestId === this.requestIds.nearby) {
        this.setState({ nearbyError: null, nearbyLoading: true });
      }
      const response = await identityAPI.getNearby();
      if (this.isMountedFlag && requestId === this.requestIds.nearby) {
        this.setState({
          nearby: asRecords(response?.data).map(normalizeNearbyPeer),
          nearbyError: null,
          nearbyLoading: false,
        });
      }
    } catch (error) {
      if (this.isMountedFlag && requestId === this.requestIds.nearby) {
        this.setState({
          nearbyError: toDisplayError(error, 'Nearby peer discovery unavailable'),
          nearbyLoading: false,
        });
      }
    }
  };

  beginMutation = () => {
    if (!this.isMountedFlag || this.mutationInFlight) return false;
    this.mutationInFlight = true;
    this.setState({ mutating: true });
    return true;
  };

  finishMutation = () => {
    this.mutationInFlight = false;
    if (this.isMountedFlag) this.setState({ mutating: false });
  };

  handleAddFromInvite = async (inviteLink, nickname) => {
    if (!this.beginMutation()) return;
    try {
      await identityAPI.addContactFromInvite({ inviteLink, nickname });
      if (!this.isMountedFlag) return;
      this.setState({ addFriendModalOpen: false });
      await this.loadContacts();
    } catch (error) {
      if (this.isMountedFlag) {
        this.setState({
          error: toDisplayError(error, 'Failed to add contact from invite'),
        });
      }
    } finally {
      this.finishMutation();
    }
  };

  handleAddFromDiscovery = async (peerId, nickname) => {
    if (!this.beginMutation()) return;
    try {
      await identityAPI.addContactFromDiscovery({ nickname, peerId });
      if (!this.isMountedFlag) return;
      await this.loadContacts();
    } catch (error) {
      if (this.isMountedFlag) {
        this.setState({
          error: toDisplayError(error, 'Failed to add discovered contact'),
        });
      }
    } finally {
      this.finishMutation();
    }
  };

  handleCreateInvite = async () => {
    if (!this.beginMutation()) return;
    const requestId = ++this.requestIds.invite;
    try {
      const response = await identityAPI.createInvite({ expiresInHours: 24 });
      const responseData =
        response?.data && typeof response.data === 'object'
          ? response.data
          : {};
      const inviteLink =
        typeof responseData.inviteLink === 'string'
          ? responseData.inviteLink
          : '';
      const inviteFriendCode =
        responseData.friendCode == null
          ? null
          : String(responseData.friendCode);
      const inviteQrDataUrl = inviteLink
        ? await QRCode.toDataURL(inviteLink, {
            errorCorrectionLevel: 'M',
            margin: 2,
            scale: 6,
          })
        : null;
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.invite
      ) {
        return;
      }
      this.setState({
        createInviteModalOpen: true,
        error: null,
        inviteFriendCode,
        inviteLink,
        inviteQrDataUrl,
      });
    } catch (error) {
      console.error('[Contacts] Create invite error:', error);
      const status = error?.response?.status;
      const url = error?.response?.config?.url || '';
      const errorMessage =
        status === 400
          ? 'Request failed. Check the CSRF token, local identity, and invite configuration.'
          : status === 401
            ? 'Authentication required. Please refresh the page.'
            : status === 403
              ? 'Not authorized.'
              : status === 404
                ? url.includes('/api/v0/api/v0')
                  ? `Endpoint not found: ${url} (possible route mismatch)`
                  : 'Identity & Friends is disabled, or the invite endpoint was not found.'
                : status >= 500
                  ? 'Server error. Please check server logs.'
                  : toDisplayError(error, 'Failed to create invite');

      if (
        this.isMountedFlag &&
        requestId === this.requestIds.invite
      ) {
        this.setState({
          createInviteModalOpen: false,
          error:
            errorMessage ||
            'Failed to create invite. Please ensure Identity & Friends is enabled and configured.',
          inviteQrDataUrl: null,
        });
      }
    } finally {
      this.finishMutation();
    }
  };

  handleDeleteContact = async (id) => {
    if (!window.confirm('Delete this contact?')) return;
    if (!this.beginMutation()) return;
    try {
      await identityAPI.deleteContact(id);
      if (!this.isMountedFlag) return;
      await this.loadContacts();
    } catch (error) {
      if (this.isMountedFlag) {
        this.setState({
          error: toDisplayError(error, 'Failed to delete contact'),
        });
      }
    } finally {
      this.finishMutation();
    }
  };

  openChat = (contact) => {
    const username = contact.nickname || contact.peerId;
    this.props.navigate('/chat', { state: { user: username } });
  };

  browseContact = (contact) => {
    const username = contact.nickname || contact.peerId;
    this.props.navigate('/browse', { state: { user: username } });
  };

  render() {
    const {
      activeTab,
      addFriendModalOpen,
      contacts,
      contactsError,
      createInviteModalOpen,
      error,
      inviteFriendCode,
      inviteLink,
      inviteQrDataUrl,
      loading,
      mutating,
      nearby,
      nearbyError,
      nearbyLoading,
    } = this.state;

    const panes = [
      {
        menuItem: 'All Contacts',
        render: () => (
          <Tab.Pane>
            {loading ? (
              <LoaderSegment />
            ) : contactsError && contacts.length === 0 ? (
              <Segment placeholder>
                <Header icon>
                  <Icon name="users" />
                  Contacts unavailable
                </Header>
              </Segment>
            ) : contacts.length === 0 ? (
              <Segment placeholder>
                <Header icon>
                  <Icon name="users" />
                  No contacts yet
                </Header>
                <Button
                  as="button"
                  data-testid="contacts-create-invite-empty"
                  disabled={mutating}
                  onClick={this.handleCreateInvite}
                  primary
                >
                  Create Invite
                </Button>
              </Segment>
            ) : (
              <Table>
                <Table.Header>
                  <Table.Row>
                    <Table.HeaderCell>Nickname</Table.HeaderCell>
                    <Table.HeaderCell>Peer ID</Table.HeaderCell>
                    <Table.HeaderCell>Verified</Table.HeaderCell>
                    <Table.HeaderCell>Last Seen</Table.HeaderCell>
                    <Table.HeaderCell>Actions</Table.HeaderCell>
                  </Table.Row>
                </Table.Header>
                <Table.Body>
                  {contacts.map((contact) => (
                    <Table.Row
                      data-testid={`contact-row-${contact.nickname || String(contact.peerId ?? '').slice(0, 8)}`}
                      key={contact.id || contact.peerId}
                    >
                      <Table.Cell>{contact.nickname || 'Unnamed'}</Table.Cell>
                      <Table.Cell>
                        <code style={{ fontSize: '0.85em' }}>
                          {String(contact.peerId ?? '').slice(0, 16)}...
                        </code>
                      </Table.Cell>
                      <Table.Cell>
                        {contact.verified ? (
                          <Label color="green">Verified</Label>
                        ) : (
                          <Label>Unverified</Label>
                        )}
                      </Table.Cell>
                      <Table.Cell>
                        {contact.lastSeen
                          ? new Date(contact.lastSeen).toLocaleString()
                          : 'Never'}
                      </Table.Cell>
                      <Table.Cell>
                        <Button.Group size="small">
                          <Popup
                            content="Open a private chat with this contact."
                            trigger={
                              <Button
                                icon="chat"
                                onClick={() => this.openChat(contact)}
                              />
                            }
                          />
                          <Popup
                            content="Browse this contact's shared files."
                            trigger={
                              <Button
                                icon="folder open"
                                onClick={() => this.browseContact(contact)}
                              />
                            }
                          />
                          <Popup
                            content="Remove this saved contact."
                            trigger={
                              <Button
                                disabled={mutating}
                                icon="trash"
                                negative
                                onClick={() =>
                                  this.handleDeleteContact(contact.id)
                                }
                              />
                            }
                          />
                        </Button.Group>
                      </Table.Cell>
                    </Table.Row>
                  ))}
                </Table.Body>
              </Table>
            )}
          </Tab.Pane>
        ),
      },
      {
        menuItem: 'Nearby',
        render: () => (
          <Tab.Pane>
            {nearbyError && (
              <Message
                data-testid="contacts-nearby-load-error"
                error
              >
                <Message.Header>Nearby peer discovery unavailable</Message.Header>
                <p>{nearbyError}</p>
                <p>Showing the last successfully discovered peers when available.</p>
              </Message>
            )}
            {nearbyLoading ? (
              <LoaderSegment />
            ) : nearbyError && nearby.length === 0 ? (
              <Segment placeholder>
                <Header icon>
                  <Icon name="wifi" />
                  Nearby peers unavailable
                </Header>
                <Button disabled={nearbyLoading} onClick={this.loadNearby}>
                  Retry Discovery
                </Button>
              </Segment>
            ) : nearby.length === 0 ? (
              <Segment placeholder>
                <Header icon>
                  <Icon name="wifi" />
                  No nearby peers found
                </Header>
                <p>Make sure you're on the same network and mDNS is working.</p>
                <Button disabled={nearbyLoading} onClick={this.loadNearby}>
                  Refresh
                </Button>
              </Segment>
            ) : (
              <List
                divided
                relaxed
              >
                {nearby.map((peer, index) => (
                  <List.Item key={index}>
                    <List.Content>
                        <List.Header>{String(peer.displayName)}</List.Header>
                        <List.Description>
                        Code: <code>{String(peer.peerCode)}</code>
                        <br />
                        Endpoint: {String(peer.endpoint)}
                      </List.Description>
                        <Button
                          disabled={mutating}
                          onClick={() => {
                          const nickname = prompt(
                            'Enter nickname for this contact:',
                          );
                          if (nickname) {
                            this.handleAddFromDiscovery(peer.peerId, nickname);
                          }
                        }}
                        primary
                        size="small"
                        style={{ marginTop: '0.5em' }}
                      >
                        Add Contact
                      </Button>
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            )}
          </Tab.Pane>
        ),
      },
    ];

    return (
      <div data-testid="contacts-root">
        <Container>
          <Header as="h1">
            <Icon name="address book" />
            <Header.Content>
              Contacts
              <Header.Subheader>Manage your peer contacts</Header.Subheader>
            </Header.Content>
          </Header>

          {error && <ErrorSegment caption={error} />}

          <div style={{ marginBottom: '1em' }}>
            {/* Always render Create Invite button - not conditional on loading state */}
            <Button
              as="button"
              data-testid="contacts-create-invite"
              disabled={mutating}
              onClick={this.handleCreateInvite}
              primary
            >
              <Icon name="plus" />
              Create Invite
            </Button>
            <Button
              as="button"
              data-testid="contacts-add-friend"
              disabled={mutating}
              onClick={() => this.setState({ addFriendModalOpen: true })}
            >
              <Icon name="user plus" />
              Add Friend
            </Button>
            <Button disabled={nearbyLoading} onClick={this.loadNearby}>
              <Icon name="refresh" />
              Refresh Nearby
            </Button>
          </div>

          <Tab
            activeIndex={activeTab}
            onTabChange={(e, { activeIndex }) =>
              this.setState({ activeTab: activeIndex })
            }
            panes={panes}
            renderActiveOnly={false}
          />

          {/* Add Friend Modal */}
          <Modal
            onClose={() => this.setState({ addFriendModalOpen: false })}
            open={addFriendModalOpen}
          >
            <Modal.Header>Add Friend from Invite</Modal.Header>
            <Modal.Content>
              <p>Paste an invite link:</p>
              <AddFriendForm
                onSubmit={(inviteLink, nickname) => {
                  this.handleAddFromInvite(inviteLink, nickname);
                }}
              />
            </Modal.Content>
          </Modal>

          {/* Create Invite Modal */}
          <Modal
            onClose={() => this.setState({ createInviteModalOpen: false })}
            open={createInviteModalOpen}
          >
            <Modal.Header>Invite Created</Modal.Header>
            <Modal.Content>
              <p>Share this invite link:</p>
              <div style={{ marginBottom: '1em' }}>
                <input
                  data-testid="contacts-invite-output"
                  onClick={(e) => e.target.select()}
                  readOnly
                  style={{ padding: '0.5em', width: '100%' }}
                  value={inviteLink || ''}
                />
              </div>
              {inviteFriendCode && (
                <p>
                  Friend Code:{' '}
                  <code data-testid="contacts-invite-friend-code">
                    {inviteFriendCode}
                  </code>
                </p>
              )}
              {inviteQrDataUrl && (
                <Segment
                  basic
                  compact
                  textAlign="center"
                >
                  <img
                    alt="QR invite code"
                    data-testid="contacts-invite-qr"
                    src={inviteQrDataUrl}
                    style={{
                      height: 192,
                      imageRendering: 'pixelated',
                      width: 192,
                    }}
                  />
                </Segment>
              )}
            </Modal.Content>
            <Modal.Actions>
              <Button
                onClick={() => this.setState({ createInviteModalOpen: false })}
              >
                Close
              </Button>
            </Modal.Actions>
          </Modal>
        </Container>
      </div>
    );
  }
}

class AddFriendForm extends Component {
  fileInputRef = React.createRef();

  state = {
    inviteLink: '',
    nickname: '',
    scanError: null,
    scanning: false,
  };

  componentDidMount() {
    this.isMountedFlag = true;
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    this.scanRequestId = (this.scanRequestId || 0) + 1;
  }

  handleQrFileSelected = async (event) => {
    const file = event.target.files?.[0];
    if (!file) return;

    this.scanRequestId = (this.scanRequestId || 0) + 1;
    const requestId = this.scanRequestId;
    this.setState({ scanError: null, scanning: true });
    try {
      if (!('BarcodeDetector' in window) || !window.createImageBitmap) {
        throw new Error(
          'This browser does not support QR scanning from images yet.',
        );
      }

      const detector = new window.BarcodeDetector({ formats: ['qr_code'] });
      const bitmap = await window.createImageBitmap(file);
      try {
        const codes = await detector.detect(bitmap);
        const inviteLink = codes.find((code) =>
          code.rawValue?.startsWith('slskr://invite/'),
        )?.rawValue;

        if (!inviteLink) {
          throw new Error('No slskr invite QR code was found in that image.');
        }

        if (
          this.isMountedFlag &&
          requestId === this.scanRequestId
        ) {
          this.setState({ inviteLink, scanError: null });
        }
      } finally {
        bitmap.close?.();
      }
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.scanRequestId
      ) {
        this.setState({ scanError: toDisplayError(error, 'QR scan failed.') });
      }
    } finally {
      event.target.value = '';
      if (
        this.isMountedFlag &&
        requestId === this.scanRequestId
      ) {
        this.setState({ scanning: false });
      }
    }
  };

  handleSubmit = (e) => {
    e.preventDefault();
    if (this.state.inviteLink && this.state.nickname) {
      this.props.onSubmit(this.state.inviteLink, this.state.nickname);
    }
  };

  render() {
    return (
      <form onSubmit={this.handleSubmit}>
        <div style={{ marginBottom: '1em' }}>
          <label>Invite Link:</label>
          <input
            data-testid="contacts-add-invite-input"
            onChange={(e) => this.setState({ inviteLink: e.target.value })}
            placeholder="slskr://invite/..."
            style={{ padding: '0.5em', width: '100%' }}
            type="text"
            value={this.state.inviteLink}
          />
          <input
            accept="image/*"
            data-testid="contacts-add-invite-qr-file"
            onChange={this.handleQrFileSelected}
            ref={this.fileInputRef}
            style={{ display: 'none' }}
            type="file"
          />
          <Button
            data-testid="contacts-scan-invite-qr"
            disabled={this.state.scanning}
            icon
            onClick={() => this.fileInputRef.current?.click()}
            title="Scan a QR invite image from your camera or photo library."
            type="button"
          >
            <Icon name="qrcode" />
          </Button>
          {this.state.scanError && (
            <p
              data-testid="contacts-qr-scan-error"
              style={{ color: '#9f3a38', marginTop: '0.5em' }}
            >
              {this.state.scanError}
            </p>
          )}
        </div>
        <div style={{ marginBottom: '1em' }}>
          <label>Nickname:</label>
          <input
            data-testid="contacts-contact-nickname"
            onChange={(e) => this.setState({ nickname: e.target.value })}
            placeholder="Friend's name"
            style={{ padding: '0.5em', width: '100%' }}
            type="text"
            value={this.state.nickname}
          />
        </div>
        <Button
          data-testid="contacts-add-invite-submit"
          primary
          type="submit"
        >
          Add Contact
        </Button>
      </form>
    );
  }
}

export default withNavigate(Contacts);
