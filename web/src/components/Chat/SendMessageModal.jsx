import './Chat.css';
import { toDisplayError } from '../../lib/errors';
import { useMountedRef } from '../../lib/useMountedRef';
import React, { useEffect, useRef, useState } from 'react';
import { Button, Form, Header, Icon, Input, Message, Modal } from 'semantic-ui-react';

const SendMessageModal = ({ initiateConversation, ...rest }) => {
  const [open, setOpen] = React.useState(false);
  const [username, setUsername] = React.useState('');
  const [message, setMessage] = React.useState('');
  const [error, setError] = useState('');
  const [sending, setSending] = useState(false);
  const usernameRef = useRef(null);
  const mountedRef = useMountedRef();

  useEffect(() => {
    if (open) {
      usernameRef.current?.focus();
    }
  }, [open]);

  const validInput = () => {
    return username.length > 0 && message.length > 0;
  };

  const sendMessage = async () => {
    if (!validInput() || sending || !mountedRef.current) {
      usernameRef.current?.focus();
      return;
    }

    setSending(true);
    setError('');
    try {
      await initiateConversation(username, message);
      if (mountedRef.current) {
        setOpen(false);
      }
    } catch (sendError) {
      if (mountedRef.current) {
        setError(toDisplayError(sendError, 'Failed to send message'));
      }
    } finally {
      if (mountedRef.current) {
        setSending(false);
      }
    }
  };

  return (
    <Modal
      onClose={() => setOpen(false)}
      onOpen={() => setOpen(true)}
      open={open}
      {...rest}
    >
      <Header>
        <Icon name="send" />
        <Modal.Content>Send Private Message</Modal.Content>
      </Header>
      <Modal.Content>
        <Form>
          <Form.Field>
            <Input
              onChange={(_event, data) => setUsername(data.value)}
              placeholder="Username"
              ref={usernameRef}
            />
          </Form.Field>
          <Form.Field>
            <Input
              onChange={(_event, data) => setMessage(data.value)}
              placeholder="Message"
            />
          </Form.Field>
        </Form>
        {error ? <Message negative>{error}</Message> : null}
      </Modal.Content>
      <Modal.Actions>
        <Button disabled={sending} onClick={() => setOpen(false)}>Cancel</Button>
        <Button
          disabled={!validInput() || sending}
          loading={sending}
          onClick={() => sendMessage()}
          positive
        >
          Send
        </Button>
      </Modal.Actions>
    </Modal>
  );
};

export default SendMessageModal;
