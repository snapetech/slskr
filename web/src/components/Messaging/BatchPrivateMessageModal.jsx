import React from 'react';
import { Button, Form, Icon, Message, Modal } from 'semantic-ui-react';

const BatchPrivateMessageModal = ({ actions, state }) => {
  const { batchMessage, batchModalOpen, batchSending, batchUsernames } = state;
  const { sendBatchMessage, setBatchMessage, setBatchModalOpen, setBatchUsernames } = actions;

  return (
          <Modal
            onClose={() => setBatchModalOpen(false)}
            open={batchModalOpen}
            size="small"
          >
            <Modal.Header>Batch Private Message</Modal.Header>
            <Modal.Content>
              <Message info>
                Sends through Soulseek's multi-recipient private-message command and stores one local conversation per recipient.
              </Message>
              <Form>
                <Form.TextArea
                  aria-label="Batch private-message recipients"
                  label="Recipients"
                  onChange={(event) => setBatchUsernames(event.target.value)}
                  placeholder="alice, bob, carol"
                  value={batchUsernames}
                />
                <Form.TextArea
                  aria-label="Batch private-message body"
                  label="Message"
                  onChange={(event) => setBatchMessage(event.target.value)}
                  placeholder="Message"
                  value={batchMessage}
                />
              </Form>
            </Modal.Content>
            <Modal.Actions>
              <Button onClick={() => setBatchModalOpen(false)}>
                Cancel
              </Button>
              <Button
                disabled={
                  batchSending ||
                  !batchMessage.trim() ||
                  !batchUsernames.trim()
                }
                loading={batchSending}
                onClick={sendBatchMessage}
                primary
              >
                <Icon name="send" />
                Send
              </Button>
            </Modal.Actions>
          </Modal>
  );
};

export default BatchPrivateMessageModal;
