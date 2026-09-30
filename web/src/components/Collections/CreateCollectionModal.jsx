import React from 'react';
import { Button, Dropdown, Form, Modal } from 'semantic-ui-react';

const CreateCollectionModal = ({ actions, state, typeOptions }) => {
  const { creatingCollection, createModalOpen, newCollectionDescription, newCollectionTitle, newCollectionType } = state;
  const { onClose, onCreate, onDescriptionChange, onTitleChange, onTypeChange } = actions;

  return (
          <Modal
            onClose={onClose}
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
                    onChange={onTypeChange}
                    options={typeOptions}
                    selection
                    value={newCollectionType}
                  />
                </Form.Field>
                <Form.Input
                  data-testid="collections-title-input"
                  label="Title"
                  onChange={onTitleChange}
                  placeholder="Enter collection title"
                  value={newCollectionTitle}
                />
                <Form.TextArea
                  label="Description"
                  onChange={onDescriptionChange}
                  placeholder="Optional description"
                  value={newCollectionDescription}
                />
              </Form>
            </Modal.Content>
            <Modal.Actions>
              <Button
                onClick={onClose}
              >
                Cancel
              </Button>
              <Button
                data-testid="collections-create-submit"
                disabled={!newCollectionTitle.trim() || creatingCollection}
                loading={creatingCollection}
                onClick={onCreate}
                primary
              >
                Create
              </Button>
            </Modal.Actions>
          </Modal>
  );
};

export default CreateCollectionModal;
