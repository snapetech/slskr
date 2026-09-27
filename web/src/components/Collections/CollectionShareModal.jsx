import LoaderSegment from '../Shared/LoaderSegment';
import React from 'react';
import { Button, Dropdown, Form, Message, Modal } from 'semantic-ui-react';

const CollectionShareModal = ({ actions, state }) => {
  const { creatingShare, shareAllowDownload, shareAllowStream, shareAudienceId, shareGroups, shareGroupsLoading, shareModalOpen } = state;
  const { onAllowDownloadChange, onAllowStreamChange, onAudienceChange, onClose, onCreateShare } = actions;

  return (
          <Modal
            onClose={onClose}
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
                      onChange={onAudienceChange}
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
                      onChange={onAllowStreamChange}
                      type="checkbox"
                    />
                  </Form.Field>
                  <Form.Field>
                    <label htmlFor="share-allow-download">Allow download</label>
                    <input
                      checked={shareAllowDownload}
                      data-testid="share-policy-download"
                      id="share-allow-download"
                      onChange={onAllowDownloadChange}
                      type="checkbox"
                    />
                  </Form.Field>
                </Form>
              )}
            </Modal.Content>
            <Modal.Actions>
              <Button
                disabled={creatingShare}
                onClick={onClose}
              >
                Cancel
              </Button>
              <Button
                data-testid="share-create-submit"
                disabled={!shareAudienceId || creatingShare}
                loading={creatingShare}
                onClick={onCreateShare}
                primary
              >
                Share
              </Button>
            </Modal.Actions>
          </Modal>
  );
};

export default CollectionShareModal;
