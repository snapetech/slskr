import React from 'react';
import { Button, Dropdown, Form, Message, Modal } from 'semantic-ui-react';

const CollectionAddItemModal = ({ actions, state }) => {
  const { addItemModalOpen, addingItem, itemSearchLoading, itemSearchQuery, itemSearchResults, selectedCollection } = state;
  const { onAddItem, onClose, onSearchInputChange, onSearchResultChange } = actions;

  return (
          <Modal
            onClose={onClose}
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
                    onChange={onSearchInputChange}
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
                      onChange={onSearchResultChange}
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
                onClick={onClose}
              >
                Cancel
              </Button>
              <Button
                data-testid="collection-add-item-submit"
                disabled={!itemSearchQuery.trim() || addingItem}
                loading={addingItem}
                onClick={onAddItem}
                primary
              >
                Add Item
              </Button>
            </Modal.Actions>
          </Modal>
  );
};

export default CollectionAddItemModal;
