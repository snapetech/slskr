import './Pods.css';
import { urlBase } from '../../config';
import { toDisplayError } from '../../lib/errors';
import * as pods from '../../lib/pods';
import { createPollingController } from '../../lib/usePolling';
import PlaceholderSegment from '../Shared/PlaceholderSegment';
import PodListenAlongPanel from '../Player/PodListenAlongPanel';
import PortForwarding from './PortForwarding';
import VpnGatewayConfig from './VpnGatewayConfig';
import React, { Component } from 'react';
import { toast } from 'react-toastify';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import {
  Button,
  Dimmer,
  Dropdown,
  Form,
  Header,
  Icon,
  Input,
  Label,
  List,
  Loader,
  Message,
  Modal,
  Popup,
  Segment,
  Tab,
} from 'semantic-ui-react';

const initialState = {
  activeChannelId: null,
  activeDetailTab: 0,
  activePodId: null,
  loading: false,
  members: [],
  messageInput: '',
  messages: {},
  podDetail: null,
  pods: [],
  createModalOpen: false,
  createDescription: '',
  createName: '',
  createTags: '',
  createVisibility: 'Unlisted',
  creatingPod: false,
  discoveryLoading: false,
  discoveryQuery: '',
  discoveryResults: [],
  leavingPod: false,
  savingPod: false,
  sendingMessage: false,
};

const GOLD_STAR_CLUB_POD_ID = 'pod:901d57a2c1bb4e5d90d57a2c1bb4e5d0';

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (record) => record && typeof record === 'object' && !Array.isArray(record),
  );

const normalizeChannel = (channel) => {
  const channelId = channel.channelId ?? channel.id;
  if (channelId === undefined || channelId === null || channelId === '') return null;
  return {
    ...channel,
    channelId: String(channelId),
    kind: String(channel.kind ?? channel.channelKind ?? 'General'),
    name: String(channel.name ?? channel.channelName ?? channelId),
  };
};

const normalizePod = (pod) => {
  const podId = pod.podId ?? pod.PodId ?? pod.id;
  if (podId === undefined || podId === null || podId === '') return null;
  return {
    ...pod,
    channels: asRecords(pod.channels).map(normalizeChannel).filter(Boolean),
    name: String(pod.name ?? pod.Name ?? podId),
    podId: String(podId),
    tags: (Array.isArray(pod.tags) ? pod.tags : Array.isArray(pod.Tags) ? pod.Tags : [])
      .filter((tag) => tag !== undefined && tag !== null)
      .map((tag) => String(tag)),
  };
};

const normalizeMessage = (message) => ({
  ...message,
  body: String(message.body ?? message.message ?? ''),
  senderPeerId: String(message.senderPeerId ?? message.username ?? message.sender ?? 'Unknown peer'),
  timestampUnixMs: Number.isFinite(Number(message.timestampUnixMs))
    ? Number(message.timestampUnixMs)
    : null,
});

const withRouter = (WrappedComponent) => {
  const RoutedComponent = (props) => {
    const location = useLocation();
    const navigate = useNavigate();
    const params = useParams();

    return (
      <WrappedComponent
        {...props}
        location={location}
        navigate={navigate}
        params={params}
      />
    );
  };

  RoutedComponent.displayName = `withRouter(${WrappedComponent.displayName || WrappedComponent.name || 'Component'})`;

  return RoutedComponent;
};

class Pods extends Component {
  constructor(props) {
    super(props);

    this.state = initialState;
    this.isMountedFlag = false;
    this.requestIds = {
      discovery: 0,
      messages: 0,
      podDetails: 0,
      pods: 0,
      selection: 0,
    };
    this.pollControllers = {
      messages: null,
      pods: null,
    };
    this.actionInFlight = {
      create: false,
      discover: false,
      leave: false,
      save: false,
      send: false,
    };
  }

  componentDidMount() {
    this.isMountedFlag = true;
    const podId = this.props.params?.podId;
    const channelId = this.props.params?.channelId;

    this.setState(
      {
        activeChannelId: channelId || null,
        activePodId: podId || null,
      },
      async () => {
        if (!this.isMountedFlag) return;
        this.startPolling();
        const podsList = await this.fetchPods();
        if (!this.isMountedFlag) return;
        if (podId) {
          await this.selectPod(podId, channelId);
        } else if (podsList.length > 0) {
          const preferredPod =
            podsList.find((pod) => pod.podId === GOLD_STAR_CLUB_POD_ID) ||
            podsList[0];
          await this.selectPod(preferredPod.podId, null);
        }
      },
    );
  }

  componentDidUpdate(previousProps) {
    // Handle route changes
    const podId = this.props.params?.podId;
    const channelId = this.props.params?.channelId;
    const previousPodId = previousProps.params?.podId;
    const previousChannelId = previousProps.params?.channelId;

    if (
      this.isMountedFlag &&
      (podId !== previousPodId || channelId !== previousChannelId) &&
      podId
    ) {
      void this.selectPod(podId, channelId);
    }
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    Object.keys(this.requestIds).forEach((key) => {
      this.requestIds[key] += 1;
    });
    this.stopPolling();
  }

  startPolling = () => {
    if (!this.pollControllers.messages) {
      this.pollControllers.messages = createPollingController(
        this.fetchMessages,
        2_000,
        { immediate: false },
      );
    }
    if (!this.pollControllers.pods) {
      this.pollControllers.pods = createPollingController(this.fetchPods, 5_000, {
        immediate: false,
      });
    }
  };

  stopPolling = () => {
    this.pollControllers.messages?.stop();
    this.pollControllers.pods?.stop();
    this.pollControllers.messages = null;
    this.pollControllers.pods = null;
  };

  beginAction = (action) => {
    if (!this.isMountedFlag || this.actionInFlight[action]) return false;
    this.actionInFlight[action] = true;
    return true;
  };

  finishAction = (action) => {
    this.actionInFlight[action] = false;
  };

  fetchPods = async () => {
    const requestId = ++this.requestIds.pods;
    try {
      const podsList = await pods.list();
      const normalizedPods = asRecords(podsList).map(normalizePod).filter(Boolean);
      if (this.isMountedFlag && requestId === this.requestIds.pods) {
        this.setState({ pods: normalizedPods });
      }
      return normalizedPods;
    } catch (error) {
      console.error('Failed to fetch pods:', error);
      if (this.isMountedFlag && requestId === this.requestIds.pods) {
        this.setState({ pods: [] });
      }
      return [];
    }
  };

  getLocalPeerId = () => {
    return this.props.state?.user?.username || 'local-peer';
  };

  fetchPodDetail = async (podId) => {
    const requestId = ++this.requestIds.podDetails;
    try {
      const [detailResponse, membersResponse] = await Promise.all([
        pods.get(podId),
        pods.getMembers(podId),
      ]);
      const detail = normalizePod(detailResponse);
      const members = asRecords(membersResponse);
      if (this.isMountedFlag && requestId === this.requestIds.podDetails) {
        this.setState({ members, podDetail: detail });
      }
      return detail;
    } catch (error) {
      console.error('Failed to fetch pod detail:', error);
      return null;
    }
  };

  fetchMessages = async (
    podId = this.state.activePodId,
    channelId = this.state.activeChannelId,
  ) => {
    if (!podId || !channelId) {
      return;
    }

    const requestId = ++this.requestIds.messages;
    const messageKey = `${podId}:${channelId}`;

    try {
      const channelMessages = await pods.getMessages(
        podId,
        channelId,
      );
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.messages &&
        this.state.activePodId === podId &&
        this.state.activeChannelId === channelId
      ) {
        this.setState((previousState) => ({
          messages: {
            ...previousState.messages,
            [messageKey]: asRecords(channelMessages).map(normalizeMessage),
          },
        }));
      }
    } catch (error) {
      console.error('Failed to fetch messages:', error);
    }
  };

  getChannelIndex = (podDetail, channelId) =>
    Math.max(
      0,
      Array.isArray(podDetail?.channels)
        ? Math.max(0, podDetail.channels.findIndex((channel) => channel.channelId === channelId))
        : 0,
    );

  selectPod = async (podId, channelId = null) => {
    const selectionId = ++this.requestIds.selection;

    // Avoid redundant updates
    if (
      this.state.activePodId === podId &&
      this.state.activeChannelId === channelId &&
      this.state.podDetail?.podId === podId
    ) {
      return;
    }

    if (!this.isMountedFlag) return;
    this.setState({ activePodId: podId, loading: true });

    const podDetail = await this.fetchPodDetail(podId);
    if (
      !this.isMountedFlag ||
      selectionId !== this.requestIds.selection ||
      !podDetail
    ) {
      if (
        this.isMountedFlag &&
        selectionId === this.requestIds.selection
      ) {
        this.setState({ loading: false });
      }
      return;
    }

    // Select first channel if none specified
    if (!channelId && podDetail?.channels?.length > 0) {
      channelId = podDetail.channels[0].channelId;
    }

    if (
      !this.isMountedFlag ||
      selectionId !== this.requestIds.selection
    ) {
      return;
    }
    await new Promise((resolve) => {
      this.setState(
        {
          activeChannelId: channelId,
          activeDetailTab: this.getChannelIndex(podDetail, channelId),
          loading: false,
        },
        resolve,
      );
    });
    if (
      !this.isMountedFlag ||
      selectionId !== this.requestIds.selection
    ) {
      return;
    }

    // Update URL only if different from current route
    const currentPodId = this.props.params?.podId;
    const currentChannelId = this.props.params?.channelId;
    if (podId !== currentPodId || channelId !== currentChannelId) {
      if (channelId) {
        this.props.navigate(`${urlBase}/pods/${podId}/channels/${channelId}`);
      } else {
        this.props.navigate(`${urlBase}/pods/${podId}`);
      }
    }

    // Fetch messages for selected channel
    if (channelId) {
      await this.fetchMessages(podId, channelId);
    }
  };

  handleDetailTabChange = async (_event, { activeIndex }) => {
    const { activePodId, podDetail } = this.state;
    const channel = podDetail?.channels?.[activeIndex];

    if (!channel || !activePodId) {
      return;
    }

    const selectionId = ++this.requestIds.selection;
    await new Promise((resolve) => {
      this.setState(
        {
          activeChannelId: channel.channelId,
          activeDetailTab: activeIndex,
        },
        resolve,
      );
    });

    if (!this.isMountedFlag || selectionId !== this.requestIds.selection) {
      return;
    }

    const currentPodId = this.props.params?.podId;
    const currentChannelId = this.props.params?.channelId;

    if (
      activePodId !== currentPodId ||
      channel.channelId !== currentChannelId
    ) {
      this.props.navigate(
        `${urlBase}/pods/${activePodId}/channels/${channel.channelId}`,
      );
    }

    await this.fetchMessages(activePodId, channel.channelId);
  };

  handleSendMessage = async () => {
    const { activeChannelId, activePodId, messageInput } = this.state;
    const { state: applicationState } = this.props;

    if (
      !activePodId ||
      !activeChannelId ||
      !messageInput.trim() ||
      !this.beginAction('send')
    ) {
      return;
    }

    // Get peerId from application state (username)
    const senderPeerId = applicationState?.user?.username || 'local-peer';

    this.setState({ sendingMessage: true });
    try {
      await pods.sendMessage(
        activePodId,
        activeChannelId,
        messageInput,
        senderPeerId,
      );
      if (
        this.isMountedFlag &&
        this.state.activePodId === activePodId &&
        this.state.activeChannelId === activeChannelId &&
        this.state.messageInput === messageInput
      ) {
        this.setState({ messageInput: '' });
      }
      // Messages will be refreshed by the shared non-overlapping poller.
    } catch (error) {
      console.error('Failed to send message:', error);
      toast.error(`Failed to send message: ${toDisplayError(error)}`);
    } finally {
      this.finishAction('send');
      if (this.isMountedFlag) this.setState({ sendingMessage: false });
    }
  };

  handleOpenCreatePod = () => {
    this.setState({
      createDescription: '',
      createModalOpen: true,
      createName: '',
      createTags: '',
      createVisibility: 'Unlisted',
    });
  };

  handleCreatePod = async () => {
    const {
      createDescription,
      createName,
      createTags,
      createVisibility,
    } = this.state;
    const name = createName.trim();
    if (!name || !this.beginAction('create')) return;

    this.setState({ creatingPod: true });
    try {
      const newPod = await pods.create({
        channels: [
          {
            channelId: 'general',
            kind: 'General',
            name: 'General',
          },
        ],
        description: createDescription.trim() || null,
        externalBindings: [],
        name,
        tags: createTags
          .split(',')
          .map((tag) => tag.trim())
          .filter(Boolean),
        visibility: createVisibility,
      }, this.getLocalPeerId());

      if (!this.isMountedFlag) return;
      const createdPod = normalizePod(newPod);
      if (!createdPod?.podId) throw new Error('The server returned no pod ID.');
      this.setState({ createModalOpen: false });
      await this.fetchPods();
      await this.selectPod(createdPod.podId);
    } catch (error) {
      console.error('Failed to create pod:', error);
      toast.error(`Failed to create pod: ${toDisplayError(error)}`);
    } finally {
      this.finishAction('create');
      if (this.isMountedFlag) this.setState({ creatingPod: false });
    }
  };

  handleDiscoverPods = async () => {
    if (!this.isMountedFlag || !this.beginAction('discover')) return;
    const requestId = ++this.requestIds.discovery;
    const query = this.state.discoveryQuery.trim();
    this.setState({ discoveryLoading: true });

    try {
      const discovered = query
        ? await pods.discoverByName(query)
        : await pods.discoverAll(50);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.discovery
      ) {
        this.setState({
          discoveryResults: asRecords(discovered).map(normalizePod).filter(Boolean),
        });
      }
    } catch (error) {
      console.error('Failed to discover pods:', error);
      toast.error(`Failed to discover pods: ${toDisplayError(error)}`);
    } finally {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.discovery
      ) {
        this.setState({ discoveryLoading: false });
      }
      this.finishAction('discover');
    }
  };

  handleSaveDiscoveredPod = async (pod) => {
    const podId = pod.podId || pod.PodId;
    const name = pod.name || pod.Name || podId;
    const tags = Array.isArray(pod.tags) ? pod.tags : Array.isArray(pod.Tags) ? pod.Tags : [];
    const visibility = pod.visibility || pod.Visibility || 'Unlisted';
    const focusContentId = pod.focusContentId || pod.FocusContentId || null;

    if (!podId || !this.beginAction('save')) return;

    this.setState({ savingPod: true });
    try {
      const savedPod = await pods.create({
        channels: [
          {
            channelId: 'general',
            kind: 'General',
            name: 'General',
          },
        ],
        externalBindings: [],
        focusContentId,
        name,
        podId,
        tags,
        visibility,
      }, this.getLocalPeerId());

      if (!this.isMountedFlag) return;
      toast.success(`Saved pod ${name}`);
      await this.fetchPods();
      const normalizedSavedPod = normalizePod(savedPod);
      if (normalizedSavedPod?.podId) await this.selectPod(normalizedSavedPod.podId);
    } catch (error) {
      console.error('Failed to save discovered pod:', error);
      toast.error(`Failed to save pod: ${toDisplayError(error)}`);
    } finally {
      this.finishAction('save');
      if (this.isMountedFlag) this.setState({ savingPod: false });
    }
  };

  handleLeaveActivePod = async () => {
    const { activePodId, podDetail } = this.state;
    const peerId = this.getLocalPeerId();
    const podName = podDetail?.name || activePodId;

    if (!activePodId || !peerId) return;

    if (
      podDetail?.podId === 'pod:901d57a2c1bb4e5d90d57a2c1bb4e5d0' &&
      !window.confirm(
        'Leaving Gold Star Club is irrevocable. You will not be able to rejoin or recover Gold Star status later. Leave anyway?',
      )
    ) {
      return;
    }

    if (!this.beginAction('leave')) return;

    this.setState({ leavingPod: true });
    try {
      await pods.leave(activePodId, peerId);
      if (!this.isMountedFlag || this.state.activePodId !== activePodId) {
        return;
      }
      toast.success(`Left ${podName}`);
      const remainingPods = await this.fetchPods();
      if (!this.isMountedFlag) return;
      const nextPod = remainingPods.find((pod) => pod.podId !== activePodId);
      if (nextPod) {
        await this.selectPod(nextPod.podId);
      } else {
        this.setState({
          activeChannelId: null,
          activePodId: null,
          members: [],
          messages: {},
          podDetail: null,
        });
      }
    } catch (error) {
      console.error('Failed to leave pod:', error);
      toast.error(`Failed to leave pod: ${toDisplayError(error)}`);
    } finally {
      this.finishAction('leave');
      if (this.isMountedFlag) this.setState({ leavingPod: false });
    }
  };

  render() {
    const {
      activeChannelId,
      activePodId,
      creatingPod,
      leavingPod,
      loading,
      members,
      messageInput,
      messages,
      podDetail,
      pods: podsList,
      savingPod,
      sendingMessage,
      createDescription,
      createModalOpen,
      createName,
      createTags,
      createVisibility,
      discoveryLoading,
      discoveryQuery,
      discoveryResults,
    } = this.state;

    const currentMessages =
      activePodId && activeChannelId
        ? asRecords(messages[`${activePodId}:${activeChannelId}`]).map(normalizeMessage)
        : [];
    const localPeerId = this.getLocalPeerId();
    const isMember = members.some(
      (member) =>
        (member.peerId || member.username || member.PeerId) === localPeerId,
    );
    const isGoldStarClub = podDetail?.podId === GOLD_STAR_CLUB_POD_ID;
    const activeChannel = podDetail?.channels?.find(
      (channel) => channel.channelId === activeChannelId,
    );

    return (
      <div className="pods-workspace">
        {/* Pod List Sidebar */}
        <Segment className="pods-sidebar">
          <div className="pods-sidebar-header">
            <h3>Pods</h3>
            <Popup
              content="Create a durable pod with a default channel. It is saved by the daemon and restored after restart."
              trigger={
                <Button
                  icon="plus"
                  onClick={this.handleOpenCreatePod}
                  size="small"
                />
              }
            />
          </div>
          <Input
            action={
              <Popup
                content="Find listed pods through the pod discovery index."
                trigger={
                  <Button
                    disabled={discoveryLoading}
                    icon="search"
                    loading={discoveryLoading}
                    onClick={this.handleDiscoverPods}
                  />
                }
              />
            }
            fluid
            onChange={(e) =>
              this.setState({ discoveryQuery: e.target.value })
            }
            onKeyUp={(e) => {
              if (e.key === 'Enter') {
                this.handleDiscoverPods();
              }
            }}
            placeholder="Find pods..."
            size="small"
            value={discoveryQuery}
          />
          {discoveryResults.length > 0 && (
            <Segment className="pod-discovery-results">
              <Header
                as="h5"
                dividing
              >
                Discovered
              </Header>
              <List selection>
                {discoveryResults.slice(0, 6).map((pod) => {
                  const podId = pod.podId || pod.PodId;
                  const name = pod.name || pod.Name || podId;
                  const tags = pod.tags || pod.Tags || [];
                  const local = podsList.some((item) => item.podId === podId);

                  return (
                    <List.Item
                      key={podId}
                      onClick={() => local && this.selectPod(podId)}
                    >
                      <List.Content>
                        <List.Header>
                          {name}
                          {local && (
                            <Label
                              color="green"
                              size="mini"
                            >
                              saved
                            </Label>
                          )}
                        </List.Header>
                        <List.Description>
                          {tags.length > 0 ? tags.join(', ') : podId}
                        </List.Description>
                      </List.Content>
                      {!local && (
                        <List.Content floated="right">
                          <Popup
                            content="Save this discovered pod locally so it appears in your pod list after restarts."
                            trigger={
                              <Button
                                basic
                                disabled={savingPod}
                                icon="save"
                                loading={savingPod}
                                onClick={(event) => {
                                  event.stopPropagation();
                                  this.handleSaveDiscoveredPod(pod);
                                }}
                                size="mini"
                              />
                            }
                          />
                        </List.Content>
                      )}
                    </List.Item>
                  );
                })}
              </List>
            </Segment>
          )}
          {podsList.length === 0 ? (
            <PlaceholderSegment
              caption="No pods yet"
              icon="users"
            />
          ) : (
            <List selection>
              {podsList.map((pod) => (
                <List.Item
                  active={pod.podId === activePodId}
                  key={pod.podId}
                  onClick={() => this.selectPod(pod.podId)}
                >
                  <List.Content>
                    <List.Header>{pod.name || pod.podId}</List.Header>
                    <List.Description>
                      {pod.tags?.join(', ') || 'No tags'}
                    </List.Description>
                  </List.Content>
                </List.Item>
              ))}
            </List>
          )}
        </Segment>

        {/* Pod Detail */}
        <Segment className="pod-detail">
          {loading ? (
            <Dimmer active>
              <Loader />
            </Dimmer>
          ) : !podDetail ? (
            <PlaceholderSegment
              caption="Select a pod to view details"
              icon="users"
            />
          ) : (
            <>
              <div className="pod-detail-header">
                <h2>{podDetail.name || podDetail.podId}</h2>
                <div className="pod-detail-meta">
                  <span>
                    <strong>Members:</strong> {members.length}
                  </span>
                  <span>
                    <strong>Channels:</strong> {podDetail.channels?.length || 0}
                  </span>
                  <span>
                    <strong>Visibility:</strong> {podDetail.visibility}
                  </span>
                </div>
                {podDetail.description && <p>{podDetail.description}</p>}
                {isGoldStarClub && (
                  <Message warning>
                    <Icon name="star" />
                    Gold Star Club membership is limited to the first 250 nodes. Leaving this pod permanently revokes local Gold Star status. There are no rejoins.
                  </Message>
                )}
                {podDetail.tags?.length > 0 && (
                  <div className="pod-tag-list">
                    {podDetail.tags.map((tag) => (
                      <Label
                        key={tag}
                        size="small"
                      >
                        {tag}
                      </Label>
                    ))}
                  </div>
                )}
                {isMember && (
                  <Popup
                    content={
                      isGoldStarClub
                        ? 'Permanently leave Gold Star Club. This is irrevocable and cannot be undone.'
                        : 'Leave this pod with the current user.'
                    }
                    trigger={
                      <Button
                        disabled={leavingPod}
                        icon
                        labelPosition="left"
                        loading={leavingPod}
                        negative={isGoldStarClub}
                        onClick={this.handleLeaveActivePod}
                        size="small"
                      >
                        <Icon name="sign-out" />
                        {isGoldStarClub ? 'Revoke Gold Star' : 'Leave Pod'}
                      </Button>
                    }
                  />
                )}
              </div>
              {activeChannelId && activeChannel?.kind !== 'Direct' && (
                <PodListenAlongPanel
                  channelId={activeChannelId}
                  compact
                  podId={activePodId}
                  user={this.props.state?.user?.username}
                />
              )}

              {podDetail.channels?.length > 0 ? (
                <>
                  <div className="pod-channel-selector">
                    {podDetail.channels.map((channel, index) => (
                      <Button
                        active={channel.channelId === activeChannelId}
                        icon={channel.kind === 'Direct' ? 'comments' : 'comment alternate'}
                        key={channel.channelId}
                        labelPosition="left"
                        onClick={() =>
                          this.handleDetailTabChange(null, { activeIndex: index })
                        }
                        size="small"
                      >
                        {channel.name || channel.channelId}
                      </Button>
                    ))}
                  </div>
                  <Segment className="pod-channel-chat">
                    <div className="pod-channel-heading">
                      <div>
                        <Header
                          as="h3"
                          content={activeChannel?.name || activeChannelId}
                        />
                        <span className="pod-channel-subtitle">
                          {activeChannel?.kind === 'Direct'
                            ? 'Direct pod channel'
                            : `${activeChannel?.kind || 'Pod'} channel`}
                        </span>
                      </div>
                      <Label basic>
                        {currentMessages.length}{' '}
                        {currentMessages.length === 1 ? 'message' : 'messages'}
                      </Label>
                    </div>
                    <Segment className="pod-message-history">
                      {currentMessages.length === 0 ? (
                        <PlaceholderSegment
                          caption="No messages yet"
                          icon="comments"
                        />
                      ) : (
                        <List relaxed="very">
                          {currentMessages.map((message, index) => (
                              <List.Item key={index}>
                                <List.Content>
                                  <List.Header>
                                  {String(message.senderPeerId ?? 'Unknown peer')}
                                  <span
                                    style={{
                                      color: '#999',
                                      fontSize: '0.8em',
                                      marginLeft: '10px',
                                    }}
                                  >
                                    {message.timestampUnixMs
                                      ? new Date(message.timestampUnixMs).toLocaleTimeString()
                                      : ''}
                                  </span>
                                </List.Header>
                                <List.Description>
                                  {String(message.body ?? '')}
                                </List.Description>
                              </List.Content>
                            </List.Item>
                          ))}
                        </List>
                      )}
                    </Segment>
                    <Segment className="pod-message-composer">
                      <Input
                        action={
                          <Popup
                            content="Send this message to the active pod channel."
                            trigger={
                              <Button
                                disabled={!messageInput.trim() || sendingMessage}
                                icon="send"
                                loading={sendingMessage}
                                onClick={this.handleSendMessage}
                                primary
                              />
                            }
                          />
                        }
                        fluid
                        onChange={(e) =>
                          this.setState({ messageInput: e.target.value })
                        }
                        onKeyPress={(e) => {
                          if (e.key === 'Enter') {
                            this.handleSendMessage();
                          }
                        }}
                        placeholder="Type a message..."
                        value={messageInput}
                      />
                    </Segment>
                  </Segment>
                  <Tab
                    menu={{ pointing: true }}
                    panes={[
                      {
                        menuItem: {
                          content: 'VPN Gateway',
                          icon: 'shield',
                          key: 'vpn-gateway',
                        },
                        render: () => (
                          <Tab.Pane>
                            <VpnGatewayConfig
                              podDetail={podDetail}
                              podId={activePodId}
                            />
                          </Tab.Pane>
                        ),
                      },
                      {
                        menuItem: {
                          content: 'Port Forwarding',
                          icon: 'exchange',
                          key: 'port-forwarding',
                        },
                        render: () => (
                          <Tab.Pane>
                            <PortForwarding />
                          </Tab.Pane>
                        ),
                      },
                    ]}
                    renderActiveOnly={false}
                  />
                </>
              ) : (
                <PlaceholderSegment
                  caption="No channels available"
                  icon="comments"
                />
              )}
            </>
          )}
        </Segment>
        <Modal
          onClose={() => this.setState({ createModalOpen: false })}
          open={createModalOpen}
          size="small"
        >
          <Modal.Header>Create Pod</Modal.Header>
          <Modal.Content>
            <Form>
              <Form.Field>
                <label>Name</label>
                <Input
                  autoFocus
                  onChange={(e) =>
                    this.setState({ createName: e.target.value })
                  }
                  placeholder="listening circle, label crate, private crew"
                  value={createName}
                />
              </Form.Field>
              <Form.TextArea
                label="Description"
                onChange={(e, { value }) =>
                  this.setState({ createDescription: value })
                }
                placeholder="What this pod is for"
                value={createDescription}
              />
              <Form.Field>
                <label>Tags</label>
                <Input
                  onChange={(e) =>
                    this.setState({ createTags: e.target.value })
                  }
                  placeholder="ambient, friends, vinyl"
                  value={createTags}
                />
              </Form.Field>
              <Form.Field>
                <label>Visibility</label>
                <Dropdown
                  fluid
                  onChange={(e, { value }) =>
                    this.setState({ createVisibility: value })
                  }
                  options={[
                    { key: 'unlisted', text: 'Unlisted', value: 'Unlisted' },
                    { key: 'listed', text: 'Listed', value: 'Listed' },
                    { key: 'private', text: 'Private', value: 'Private' },
                  ]}
                  selection
                  value={createVisibility}
                />
              </Form.Field>
              <Message info>
                <Icon name="save" />
                Pods are stored by the server, so the list and messages survive browser reloads and daemon restarts.
              </Message>
            </Form>
          </Modal.Content>
          <Modal.Actions>
            <Button onClick={() => this.setState({ createModalOpen: false })}>
              Cancel
            </Button>
            <Button
              disabled={!createName.trim() || creatingPod}
              loading={creatingPod}
              onClick={this.handleCreatePod}
              primary
            >
              Create
            </Button>
          </Modal.Actions>
        </Modal>
      </div>
    );
  }
}

export default withRouter(Pods);
