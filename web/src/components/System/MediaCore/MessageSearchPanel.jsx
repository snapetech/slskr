import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { memo, useCallback, useState } from 'react';
import { toast } from 'react-toastify';
import { Button, Header, Input, Message } from 'semantic-ui-react';

const useMountedState = (mountedRef, initialValue) => {
  const [value, setValue] = useState(initialValue);
  const setMountedValue = useCallback(
    (nextValue) => {
      if (mountedRef.current) {
        setValue(nextValue);
      }
    },
    [mountedRef],
  );

  return [value, setMountedValue];
};

const MessageSearchPanel = () => {
  const mountedRef = useMountedRef();
  const [searchQuery, setSearchQuery] = useMountedState(mountedRef, '');
  const [searchResults, setSearchResults] = useMountedState(mountedRef, null);
  const [searchResultsQuery, setSearchResultsQuery] = useMountedState(
    mountedRef,
    null,
  );
  const [searchError, setSearchError] = useMountedState(mountedRef, null);
  const [searchLoading, setSearchLoading] = useMountedState(mountedRef, false);

  const handleSearchMessages = async () => {
    const requestedQuery = searchQuery.trim();
    if (!requestedQuery) return;

    try {
      setSearchLoading(true);
      setSearchError(null);
      const result = await mediacore.searchMessages(
        'all',
        requestedQuery,
        null,
        50,
      ); // Search all pods
      if (!Array.isArray(result)) {
        throw new Error('Invalid message search response');
      }
      setSearchResults(result);
      setSearchResultsQuery(requestedQuery);
    } catch (error_) {
      setSearchError(toDisplayError(error_, 'Failed to search messages'));
      toast.error(`Failed to search messages: ${toDisplayError(error_)}`);
    } finally {
      setSearchLoading(false);
    }
  };

  const hasCurrentSearchResults =
    Array.isArray(searchResults) && searchResultsQuery === searchQuery.trim();

  return (
    <>
      <Header size="small">Message Search</Header>
      <Input
        action={
          <Button
            color="green"
            disabled={!searchQuery.trim()}
            loading={searchLoading}
            onClick={() => handleSearchMessages()}
          >
            Search
          </Button>
        }
        onChange={(e) => {
          const nextQuery = e.target.value;
          setSearchQuery(nextQuery);
          if (nextQuery.trim() !== searchQuery.trim()) {
            setSearchError(null);
            setSearchResultsQuery(null);
          }
        }}
        placeholder="Search messages..."
        style={{ marginBottom: '1em', width: '100%' }}
        value={searchQuery}
      />

      {searchError && (
        <Message data-testid="message-search-error" negative size="small">
          {searchError}
          {hasCurrentSearchResults && searchResults.length > 0 && (
            <div>Showing last successfully loaded results.</div>
          )}
        </Message>
      )}

      {hasCurrentSearchResults && searchResults.length > 0 && (
        <Message size="small">
          <Message.Header>
            Search Results ({searchResults.length})
          </Message.Header>
          <div style={{ maxHeight: '300px', overflowY: 'auto' }}>
            {searchResults.map((message, index) => (
              <div
                key={index}
                style={{
                  border: '1px solid #ddd',
                  borderRadius: '4px',
                  marginBottom: '0.5em',
                  padding: '0.5em',
                }}
              >
                <small style={{ color: '#666' }}>
                  {new Date(message.timestampUnixMs).toLocaleString()} •{' '}
                  {message.senderPeerId} • {message.channelId}
                </small>
                <div style={{ marginTop: '0.25em' }}>{message.body}</div>
              </div>
            ))}
          </div>
        </Message>
      )}

      {hasCurrentSearchResults &&
        searchResults.length === 0 &&
        searchQuery &&
        !searchError && (
          <Message size="small" warning>
            No messages found matching "{searchQuery}"
          </Message>
        )}
    </>
  );
};

export default memo(MessageSearchPanel);
