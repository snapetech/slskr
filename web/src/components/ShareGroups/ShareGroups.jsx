import * as collectionsAPI from '../../lib/collections';
import { toDisplayError } from '../../lib/errors';
import * as identityAPI from '../../lib/identity';
import { readOptionalApiResponse } from '../../lib/optionalApi';
import ErrorSegment from '../Shared/ErrorSegment';
import LoaderSegment from '../Shared/LoaderSegment';
import TooltipButton from '../Shared/TooltipButton';
import React, { Component } from 'react';
import {
  Container,
  Dropdown,
  Form,
  Header,
  Icon,
  Message,
  Modal,
  Segment,
  Table,
} from 'semantic-ui-react';

const Button = TooltipButton;

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (record) => record && typeof record === 'object' && !Array.isArray(record),
  );

const normalizeShareGroup = (group) => {
  const id = group.id ?? group.Id;
  if (id === undefined || id === null || id === '') return null;
  return {
    ...group,
    id,
    name: String(group.name ?? group.Name ?? 'Unnamed group'),
  };
};

const normalizeContact = (contact) => {
  const username = String(contact.username ?? contact.nickname ?? '').trim();
  if (!username) return null;
  return {
    ...contact,
    id: contact.id ?? username,
    username,
  };
};

const requireArrayData = (response, resource) => {
  if (!Array.isArray(response?.data)) {
    throw new Error(`Share groups API returned an invalid ${resource} response`);
  }

  return response.data;
};

export default class ShareGroups extends Component {
  isMountedFlag = false;
  operationInFlight = false;
  membersInFlight = false;

  requestIds = {
    data: 0,
    members: 0,
    operation: 0,
  };

  state = {
    addMemberModalOpen: false,
    contacts: [],
    createModalOpen: false,
    error: null,
    loading: true,
    newGroupName: '',
    selectedContactId: null,
    selectedGroup: null,
    selectedUserId: null,
    shareGroups: [],
    operationPending: false,
    viewingMembersGroupId: null,
  };

  componentDidMount() {
    this.isMountedFlag = true;
    this.loadData();
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    Object.keys(this.requestIds).forEach((key) => {
      this.requestIds[key] += 1;
    });
  }

  beginOperation = () => {
    if (!this.isMountedFlag || this.operationInFlight) return false;
    this.operationInFlight = true;
    this.setState({ operationPending: true });
    return true;
  };

  finishOperation = () => {
    this.operationInFlight = false;
    if (this.isMountedFlag) this.setState({ operationPending: false });
  };

  loadData = async () => {
    const requestId = ++this.requestIds.data;
    try {
      if (this.isMountedFlag && requestId === this.requestIds.data) {
        this.setState({ error: null, loading: true });
      }
      const [groupsRes, contactsRes] = await Promise.all([
        readOptionalApiResponse(() => collectionsAPI.getShareGroups()),
        readOptionalApiResponse(() => identityAPI.getContacts()),
      ]);
      if (this.isMountedFlag && requestId === this.requestIds.data) {
        this.setState({
          contacts: asRecords(contactsRes.data)
            .map(normalizeContact)
            .filter(Boolean),
          loading: false,
          shareGroups: asRecords(groupsRes.data)
            .map(normalizeShareGroup)
            .filter(Boolean),
        });
      }
    } catch (error) {
      if (this.isMountedFlag && requestId === this.requestIds.data) {
        this.setState({
          error: toDisplayError(error, 'Failed to load share groups'),
          loading: false,
        });
      }
    }
  };

  handleViewMembers = async (groupId) => {
    if (!this.isMountedFlag || this.membersInFlight) return;
    this.membersInFlight = true;
    const requestId = ++this.requestIds.members;
    this.setState({ viewingMembersGroupId: groupId });
    try {
      const membersRes = await collectionsAPI.getShareGroupMembers(
        groupId,
        true,
      );
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.members
      ) {
        return;
      }
      const members = requireArrayData(membersRes, 'member list');
      window.alert(
        `Members:\n${members
          .filter((member) => member && typeof member === 'object')
          .map((member) => member.contactNickname || member.username || member.userId || 'Unknown member')
          .join('\n')}`,
      );
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.members
      ) {
        console.error('[ShareGroups] Failed to load group members:', error);
        this.setState({ error: toDisplayError(error, 'Failed to load group members') });
      }
    } finally {
      this.membersInFlight = false;
      if (this.isMountedFlag) this.setState({ viewingMembersGroupId: null });
    }
  };

  handleCreateGroup = async () => {
    if (!this.beginOperation()) return;
    const requestId = ++this.requestIds.operation;
    try {
      await collectionsAPI.createShareGroup({ name: this.state.newGroupName });
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.operation
      ) {
        return;
      }
      this.setState({ createModalOpen: false, error: null, newGroupName: '' });
      await this.loadData();
    } catch (error) {
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.operation
      ) {
        return;
      }
      console.error('[ShareGroups] Create group error:', error);
      this.setState({
        error: toDisplayError(error, 'Failed to create share group'),
      });
    } finally {
      this.finishOperation();
    }
  };

  handleAddMember = async () => {
    if (!this.state.selectedGroup || !this.beginOperation()) return;

    const requestId = ++this.requestIds.operation;
    try {
      // POST /sharegroups/:id/members only ever reads a "username" field
      // (verified against the controller's own test suite) — there is no
      // peerId-aware membership path on this endpoint. The legacy-username
      // input is the one input that's actually a Soulseek username; wire it
      // through under the field name the backend expects.
      const data = {
        username: this.state.selectedUserId || this.state.selectedContactId,
      };

      await collectionsAPI.addShareGroupMember(
        this.state.selectedGroup.id,
        data,
      );
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.operation
      ) {
        return;
      }
      this.setState({
        addMemberModalOpen: false,
        error: null,
        selectedContactId: null,
        selectedUserId: null,
      });
      await this.loadData();
    } catch (error) {
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.operation
      ) {
        return;
      }
      this.setState({
        error: toDisplayError(error, 'Failed to add member'),
      });
    } finally {
      this.finishOperation();
    }
  };

  handleDeleteGroup = async (id) => {
    if (!window.confirm('Delete this share group?')) return;
    if (!this.beginOperation()) return;
    const requestId = ++this.requestIds.operation;
    try {
      await collectionsAPI.deleteShareGroup(id);
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.operation
      ) {
        return;
      }
      await this.loadData();
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.operation
      ) {
        this.setState({ error: toDisplayError(error, 'Failed to delete share group') });
      }
    } finally {
      this.finishOperation();
    }
  };

  handleRemoveMember = async (groupId, userId) => {
    if (!window.confirm('Remove this member?')) return;
    if (!this.beginOperation()) return;
    const requestId = ++this.requestIds.operation;
    try {
      await collectionsAPI.removeShareGroupMember(groupId, userId);
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.operation
      ) {
        return;
      }
      await this.loadData();
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.operation
      ) {
        this.setState({ error: toDisplayError(error, 'Failed to remove group member') });
      }
    } finally {
      this.finishOperation();
    }
  };

  render() {
    const {
      addMemberModalOpen,
      contacts,
      createModalOpen,
      error,
      loading,
      newGroupName,
      selectedContactId,
      selectedGroup,
      selectedUserId,
      shareGroups,
      operationPending,
      viewingMembersGroupId,
    } = this.state;

    const contactOptions = contacts.map((c) => ({
      contact: c,
      key: c.id,
      text: c.username,
      value: c.username,
    }));

    if (loading) return <LoaderSegment />;

    return (
      <Container>
        <Header as="h1">
          <Icon name="users" />
          <Header.Content>
            Share Groups
            <Header.Subheader>
              Manage groups for sharing collections
            </Header.Subheader>
          </Header.Content>
        </Header>

        {error && <ErrorSegment caption={error} />}

        <div style={{ marginBottom: '1em' }}>
          <Button
            data-testid="groups-create"
            onClick={() => this.setState({ createModalOpen: true })}
            primary
            tooltip="Create a named group that can be granted access to shared collections."
          >
            <Icon name="plus" />
            Create Group
          </Button>
        </div>

        {error && shareGroups.length === 0 ? null : shareGroups.length === 0 ? (
          <Segment placeholder>
            <Header icon>
              <Icon name="users" />
              No share groups yet
            </Header>
            <Button
              onClick={() => this.setState({ createModalOpen: true })}
              primary
              tooltip="Create the first share group for collection permissions."
            >
              Create Your First Group
            </Button>
          </Segment>
        ) : (
          <Table>
            <Table.Header>
              <Table.Row>
                <Table.HeaderCell>Name</Table.HeaderCell>
                <Table.HeaderCell>Members</Table.HeaderCell>
                <Table.HeaderCell>Created</Table.HeaderCell>
                <Table.HeaderCell>Actions</Table.HeaderCell>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {shareGroups.map((group) => (
                <Table.Row
                  data-testid={`group-row-${group.name}`}
                  key={group.id}
                >
                  <Table.Cell>{group.name}</Table.Cell>
                  <Table.Cell>
                    <Button
                      onClick={() => this.handleViewMembers(group.id)}
                      disabled={operationPending || this.membersInFlight}
                      loading={viewingMembersGroupId === group.id}
                      size="small"
                      tooltip="Show the contacts or users currently assigned to this group."
                    >
                      View Members
                    </Button>
                  </Table.Cell>
                  <Table.Cell>
                    {new Date(group.createdAt).toLocaleDateString()}
                  </Table.Cell>
                  <Table.Cell>
                    <Button
                      data-testid="group-add-member"
                      onClick={() =>
                        this.setState({
                          addMemberModalOpen: true,
                          selectedGroup: group,
                        })
                      }
                      primary
                      size="small"
                      tooltip="Add a contact or Soulseek username to this share group."
                    >
                      Add Member
                    </Button>
                    <Button
                      negative
                      onClick={() => this.handleDeleteGroup(group.id)}
                      disabled={operationPending}
                      size="small"
                      tooltip="Delete this share group and remove its collection access."
                    >
                      Delete
                    </Button>
                  </Table.Cell>
                </Table.Row>
              ))}
            </Table.Body>
          </Table>
        )}

        {/* Create Group Modal */}
        <Modal
          onClose={() =>
            this.setState({ createModalOpen: false, newGroupName: '' })
          }
          open={createModalOpen}
        >
          <Modal.Header>Create Share Group</Modal.Header>
          <Modal.Content>
            <Form>
              <Form.Input
                data-testid="groups-name-input"
                label="Group Name"
                onChange={(e) =>
                  this.setState({ newGroupName: e.target.value })
                }
                placeholder="Enter group name"
                value={newGroupName}
              />
            </Form>
          </Modal.Content>
          <Modal.Actions>
            <Button
              onClick={() =>
                this.setState({ createModalOpen: false, newGroupName: '' })
              }
              tooltip="Close this dialog without creating a share group."
            >
              Cancel
            </Button>
            <Button
              data-testid="groups-create-submit"
              disabled={!newGroupName.trim() || operationPending}
              loading={operationPending}
              onClick={this.handleCreateGroup}
              primary
              tooltip="Create this share group."
            >
              Create
            </Button>
          </Modal.Actions>
        </Modal>

        {/* Add Member Modal */}
        <Modal
          onClose={() =>
            this.setState({
              addMemberModalOpen: false,
              selectedContactId: null,
              selectedGroup: null,
              selectedUserId: null,
            })
          }
          open={addMemberModalOpen}
        >
          <Modal.Header>Add Member to {selectedGroup?.name}</Modal.Header>
          <Modal.Content>
            {contacts.length > 0 ? (
              <Form>
                <Form.Field>
                  <label>Add from Contacts</label>
                  <Dropdown
                    data-testid="group-member-picker"
                    fluid
                    onChange={(e, { value }) =>
                      this.setState({
                        selectedContactId: value,
                      })
                    }
                    options={contactOptions}
                    placeholder="Select a contact"
                    search
                    selection
                    value={selectedContactId}
                  />
                </Form.Field>
                <Message info>
                  <p>Or enter a Soulseek username (legacy):</p>
                  <Form.Input
                    onChange={(e) =>
                      this.setState({
                        selectedUserId: e.target.value,
                      })
                    }
                    placeholder="Soulseek username"
                    value={selectedUserId}
                  />
                </Message>
              </Form>
            ) : (
              <Form>
                <Form.Field>
                  <label>Soulseek Username (legacy)</label>
                  <Form.Input
                    onChange={(e) =>
                      this.setState({ selectedUserId: e.target.value })
                    }
                    placeholder="Enter username"
                    value={selectedUserId}
                  />
                </Form.Field>
                <Message warning>
                  No contacts available. Add contacts from the Contacts page to
                  use friend-based sharing.
                </Message>
              </Form>
            )}
          </Modal.Content>
          <Modal.Actions>
            <Button
              onClick={() =>
                this.setState({
                  addMemberModalOpen: false,
                  selectedContactId: null,
                  selectedGroup: null,
                  selectedUserId: null,
                })
              }
              tooltip="Close this dialog without adding a member."
            >
              Cancel
            </Button>
            <Button
              data-testid="group-member-add-submit"
              disabled={(!selectedContactId && !selectedUserId) || operationPending}
              loading={operationPending}
              onClick={this.handleAddMember}
              primary
              tooltip="Add the selected contact or username to this share group."
            >
              Add Member
            </Button>
          </Modal.Actions>
        </Modal>
      </Container>
    );
  }
}
