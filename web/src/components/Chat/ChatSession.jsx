import './Chat.css';
import * as chat from '../../lib/chat';
import { createPollingController } from '../../lib/usePolling';
import { toDisplayError } from '../../lib/errors';
import PlaceholderSegment from '../Shared/PlaceholderSegment';
import UserCard from '../Shared/UserCard';
import React, { Component, createRef } from 'react';
import {
  Card,
  Dimmer,
  Icon,
  Input,
  List,
  Loader,
  Ref,
  Segment,
} from 'semantic-ui-react';

class ChatSession extends Component {
  constructor(props) {
    super(props);

    this.state = {
      conversation: null,
      error: null,
      loading: false,
      message: '',
    };

    this.listRef = createRef();
    this.messageRef = undefined;
    this.pollController = null;
    this.isMountedFlag = false;
    this.requestIds = {
      conversation: 0,
      send: 0,
    };
  }

  componentDidMount() {
    this.isMountedFlag = true;
    if (this.props.active === false) {
      return;
    }

    this.startPolling();
  }

  startPolling = () => {
    if (this.pollController) {
      return;
    }

    this.pollController = createPollingController(
      this.fetchConversation,
      5_000,
    );
  };

  stopPolling = () => {
    if (!this.pollController) {
      return;
    }

    this.pollController.stop();
    this.pollController = null;
  };

  componentDidUpdate(previousProps) {
    // If username changed, fetch new conversation
    if (
      previousProps.username !== this.props.username ||
      (!previousProps.active && this.props.active)
    ) {
      const usernameChanged = previousProps.username !== this.props.username;
      this.setState(usernameChanged ? { message: '' } : {}, () => {
        if (!this.isMountedFlag) return;
        if (this.pollController) {
          this.pollController.refresh();
        } else {
          this.fetchConversation();
        }
        if (this.props.active !== false) {
          this.focusInput();
        }
      });
    }

    if (previousProps.active === false && this.props.active !== false) {
      this.startPolling();
    }

    if (previousProps.active !== false && this.props.active === false) {
      this.stopPolling();
    }
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    this.requestIds.conversation += 1;
    this.requestIds.send += 1;
    this.stopPolling();
  }

  fetchConversation = async () => {
    const { username } = this.props;
    const requestId = ++this.requestIds.conversation;
    const requestedUsername = username;
    if (!this.isMountedFlag) return;

    if (!username) {
      this.setState({ conversation: null, loading: false });
      return;
    }

    this.setState({ loading: true });

    try {
      const conversation = await chat.get({ username });
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.conversation ||
        this.props.username !== requestedUsername
      ) {
        return;
      }

      // Acknowledge unread messages only when the user is looking at this tab.
      if (this.props.active !== false && conversation?.hasUnAcknowledgedMessages) {
        try {
          await chat.acknowledge({ username });
        } catch (acknowledgeError) {
          // A failed acknowledgement must not discard a conversation that was
          // fetched successfully; the next active poll can retry it.
          console.debug('Failed to acknowledge conversation:', acknowledgeError);
        }
      }

      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.conversation ||
        this.props.username !== requestedUsername
      ) {
        return;
      }

      this.setState({ conversation, error: null, loading: false }, () => {
        if (!this.isMountedFlag) return;
        // Scroll to bottom
        try {
          if (this.listRef.current?.lastChild) {
            this.listRef.current.lastChild.scrollIntoView();
          }
        } catch {
          // no-op
        }
      });
    } catch (error) {
      console.error('Failed to fetch conversation:', error);
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.conversation &&
        this.props.username === requestedUsername
      ) {
        this.setState({
          conversation: null,
          error: toDisplayError(error, 'Failed to load conversation'),
          loading: false,
        });
      }
    }
  };

  sendMessage = async (message) => {
    const { username } = this.props;
    if (!username || !message) return;

    const requestId = ++this.requestIds.send;
    const requestedUsername = username;
    try {
      await chat.send({ message, username });
      if (
        !this.isMountedFlag ||
        requestId !== this.requestIds.send ||
        this.props.username !== requestedUsername
      ) {
        return;
      }
      this.setState({ error: null, message: '' });

      // Refresh to show new message
      if (this.pollController) {
        await this.pollController.refresh();
      } else {
        await this.fetchConversation();
      }
    } catch (error) {
      if (
        this.isMountedFlag &&
        requestId === this.requestIds.send &&
        this.props.username === requestedUsername
      ) {
        this.setState({ error: toDisplayError(error, 'Failed to send message') });
      }
    }
  };

  sendReply = async () => {
    const { message } = this.state;
    if (!message || !message.trim()) return;

    await this.sendMessage(message.trim());
  };

  validInput = () => {
    const { username } = this.props;
    const { message } = this.state;
    return username && message && message.trim().length > 0;
  };

  focusInput = () => {
    if (this.messageRef?.current) {
      this.messageRef.current.focus();
    }
  };

  handleFocusInput = () => {
    this.focusInput();
  };

  formatTimestamp = (timestamp) => {
    const numericTimestamp = Number(timestamp);
    const date = new Date(
      Number.isFinite(numericTimestamp) && numericTimestamp > 0
        ? numericTimestamp < 10_000_000_000
          ? numericTimestamp * 1_000
          : numericTimestamp
        : timestamp,
    );
    if (Number.isNaN(date.getTime())) {
      return 'Unknown time';
    }

    const dtfUS = new Intl.DateTimeFormat('en', {
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
      month: 'numeric',
    });

    return dtfUS.format(date);
  };

  deleteConversation = async () => {
    const { onDelete, username } = this.props;
    if (!username || !this.isMountedFlag) return;

    if (onDelete) {
      await onDelete(username);
    }
  };

  render() {
    const { user, username } = this.props;
    const { conversation, error, loading, message } = this.state;
    const messages = conversation?.messages || [];

    if (!username) {
      return (
        <PlaceholderSegment
          caption="Select a conversation or start a new chat"
          icon="comment"
        />
      );
    }

    return (
      <Card
        className="chat-active-card"
        raised
      >
        <Card.Content onClick={this.handleFocusInput}>
          <Card.Header>
            <Icon
              color="green"
              name="circle"
            />
            <UserCard username={username}>{username}</UserCard>
            <Icon
              className="close-button"
              color="red"
              link
              name="close"
              onClick={this.deleteConversation}
            />
          </Card.Header>
          <div className="chat">
            {error ? <Segment negative>{error}</Segment> : null}
            {loading ? (
              <Dimmer
                active
                inverted
              >
                <Loader inverted />
              </Dimmer>
            ) : (
              <Segment.Group>
                <Segment className="chat-history">
                  <Ref innerRef={this.listRef}>
                    <List>
                      {messages.map((message) => (
                        <List.Content
                          className={`chat-message ${message.direction === 'Out' ? 'chat-message-self' : ''}`}
                          key={`${message.timestamp}+${message.message}`}
                        >
                          <span className="chat-message-time">
                            {this.formatTimestamp(message.timestamp)}
                          </span>
                          <span className="chat-message-name">
                            {message.direction === 'Out'
                              ? user?.username || 'You'
                              : message.username}
                            :{' '}
                          </span>
                          <span className="chat-message-message">
                            {message.message}
                          </span>
                        </List.Content>
                      ))}
                      <List.Content id="chat-history-scroll-anchor" />
                    </List>
                  </Ref>
                </Segment>
                <Segment className="chat-input">
                  <Input
                    action={{
                      className: 'chat-message-button',
                      disabled: !this.validInput(),
                      icon: (
                        <Icon
                          color="green"
                          name="send"
                        />
                      ),
                      onClick: this.sendReply,
                    }}
                    fluid
                    input={
                      <input
                        autoComplete="off"
                        data-lpignore="true"
                        id="chat-message-input"
                        type="text"
                      />
                    }
                    onChange={(event, { value }) =>
                      this.setState({ message: value })
                    }
                    onKeyUp={(event) =>
                      event.key === 'Enter' ? this.sendReply() : ''
                    }
                    ref={(input) => (this.messageRef = input && input.inputRef)}
                    transparent
                    value={message}
                  />
                </Segment>
              </Segment.Group>
            )}
          </div>
        </Card.Content>
      </Card>
    );
  }
}

export default ChatSession;
