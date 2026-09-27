import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import PodWorkflowNotice from './PodWorkflowNotice';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import {
  Card,
  Grid,
  Header,
  Icon,
  Input,
  Message,
} from 'semantic-ui-react';

const useMountedState = (mountedRef, initialValue) => {
  const [value, setValue] = React.useState(initialValue);
  const setMountedValue = React.useCallback(
    (nextValue) => {
      if (mountedRef.current) {
        setValue(nextValue);
      }
    },
    [mountedRef],
  );

  return [value, setMountedValue];
};

const PodContentLinkingPanel = ({ contentId, setContentId, visible = true }) => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Pod Content Linking states
  const [contentValidation, setContentValidation] = useMountedState(mountedRef, null);
  const [contentMetadata, setContentMetadata] = useMountedState(mountedRef, null);
  const [contentSearchQuery, setContentSearchQuery] = useMountedState(mountedRef, '');
  const [contentSearchResults, setContentSearchResults] = useMountedState(mountedRef, []);
  const [contentSearchError, setContentSearchError] = useMountedState(mountedRef, null);
  const [contentValidationLoading, setContentValidationLoading] =
    useMountedState(mountedRef, false);
  const [contentMetadataLoading, setContentMetadataLoading] = useMountedState(mountedRef, false);
  const [contentSearchLoading, setContentSearchLoading] = useMountedState(mountedRef, false);
  const [createPodLoading, setCreatePodLoading] = useMountedState(mountedRef, false);
  const [newPodName, setNewPodName] = useMountedState(mountedRef, '');
  const [newPodVisibility, setNewPodVisibility] = useMountedState(mountedRef, 'Unlisted');


  const handleValidateContentId = async () => {
    if (!contentId.trim()) {
      toast.error('Content ID is required');
      return;
    }

    try {
      setContentValidationLoading(true);
      setContentValidation(null);
      setContentMetadata(null);
      const result = await mediacore.validateContentIdForPod(contentId.trim());
      setContentValidation(result);

      // If valid, automatically fetch metadata
      if (result.isValid) {
        await handleGetContentMetadata();
      }
    } catch (error_) {
      setContentValidation({ error: toDisplayError(error_), isValid: false });
      toast.error(`Failed to validate content ID: ${toDisplayError(error_)}`);
    } finally {
      setContentValidationLoading(false);
    }
  };

  const handleGetContentMetadata = async () => {
    if (!contentId.trim()) return;

    try {
      setContentMetadataLoading(true);
      const metadata = await mediacore.getContentMetadata(contentId.trim());
      setContentMetadata(metadata);

      // Auto-fill pod name if empty
      if (!newPodName.trim() && metadata) {
        setNewPodName(`${metadata.artist} - ${metadata.title}`);
      }
    } catch (error_) {
      toast.error(`Failed to get content metadata: ${toDisplayError(error_)}`);
      setContentMetadata(null);
    } finally {
      setContentMetadataLoading(false);
    }
  };

  const handleSearchContent = async () => {
    if (!contentSearchQuery.trim()) return;

    try {
      setContentSearchLoading(true);
      setContentSearchError(null);
      setContentSearchResults([]);
      const results = await mediacore.searchContent(
        contentSearchQuery.trim(),
        null,
        10,
      );
      setContentSearchResults(results);
    } catch (error_) {
      toast.error(`Failed to search content: ${toDisplayError(error_)}`);
      setContentSearchError(toDisplayError(error_, 'Failed to search content'));
      setContentSearchResults([]);
    } finally {
      setContentSearchLoading(false);
    }
  };

  const handleCreateContentLinkedPod = async () => {
    if (!contentId.trim()) {
      toast.error('Content ID is required');
      return;
    }

    if (!newPodName.trim()) {
      toast.error('Pod name is required');
      return;
    }

    if (!contentValidation?.isValid) {
      toast.error('Please validate the content ID first');
      return;
    }

    try {
      setCreatePodLoading(true);
      const podRequest = {
        channels: [
          {
            channelId: 'general',
            kind: 'General',
            name: 'General',
          },
        ],

        contentId: contentId.trim(),

        externalBindings: [],
        // Auto-generate
        name: newPodName.trim(),
        podId: '',
        tags: [],
        visibility: newPodVisibility,
      };

      const createdPod = await mediacore.createContentLinkedPod(podRequest);
      toast.success(`Pod "${createdPod.name}" created successfully!`);

      // Reset form
      setContentId('');
      setContentValidation(null);
      setContentMetadata(null);
      setNewPodName('');
      setContentSearchQuery('');
      setContentSearchResults([]);
    } catch (error_) {
      toast.error(`Failed to create pod: ${toDisplayError(error_)}`);
    } finally {
      setCreatePodLoading(false);
    }
  };

  const selectContentFromSearch = (contentItem) => {
    setContentId(contentItem.contentId);
    setContentSearchQuery('');
    setContentSearchResults([]);
  };


  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-content-linking" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="linkify" />
                Pod Content Linking
              </Card.Header>
              <Card.Description>
                Create pods linked to specific content (music, videos, etc.) for
                focused discussions
              </Card.Description>
              <PodWorkflowNotice title="Can create content-linked pods">
                Content search and validation are read-only. Creating a
                content-linked pod can publish content identifiers and pod
                metadata depending on visibility settings.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Content Linking */}
            <Card.Content>
              <Header size="small">Content Search & Validation</Header>

              {/* Content Search */}
              <Input
                action={
                  <Button
                    color="blue"
                    disabled={!contentSearchQuery.trim()}
                    loading={contentSearchLoading}
                    onClick={() => handleSearchContent()}
                  >
                    Search
                  </Button>
                }
                onChange={(e) => setContentSearchQuery(e.target.value)}
                placeholder="Search for content (artist, album, movie, etc.)"
                style={{ marginBottom: '1em', width: '100%' }}
                value={contentSearchQuery}
              />

              {/* Search Results */}
              {contentSearchError && (
                <Message
                  data-testid="content-search-error"
                  negative
                  size="small"
                >
                  {contentSearchError}
                </Message>
              )}

              {contentSearchResults.length > 0 && (
                <div style={{ marginBottom: '1em' }}>
                  <Header size="tiny">Search Results</Header>
                  {contentSearchResults.map((item, index) => (
                    <Card
                      key={index}
                      onClick={() => selectContentFromSearch(item)}
                      style={{ cursor: 'pointer', marginBottom: '0.5em' }}
                    >
                      <Card.Content style={{ padding: '0.5em' }}>
                        <strong>{item.title}</strong>
                        {item.subtitle && <div>{item.subtitle}</div>}
                        <small>
                          {item.domain} • {item.type}
                        </small>
                      </Card.Content>
                    </Card>
                  ))}
                </div>
              )}

              {/* Content Validation */}
              <Input
                action={
                  <Button
                    color="green"
                    disabled={!contentId.trim()}
                    loading={contentValidationLoading}
                    onClick={() => handleValidateContentId()}
                  >
                    Validate
                  </Button>
                }
                onChange={(e) => setContentId(e.target.value)}
                placeholder="Content ID (e.g., content:audio:album:mb-release-id)"
                style={{ marginBottom: '1em', width: '100%' }}
                value={contentId}
              />

              {/* Validation Result */}
              {contentValidation && (
                <Message
                  negative={!contentValidation.isValid}
                  positive={contentValidation.isValid}
                  size="small"
                  style={{ marginBottom: '1em' }}
                >
                  <Message.Header>
                    {contentValidation.isValid
                      ? '✓ Valid Content ID'
                      : '✗ Invalid Content ID'}
                  </Message.Header>
                  {!contentValidation.isValid &&
                    contentValidation.errorMessage && (
                      <p>{contentValidation.errorMessage}</p>
                    )}
                </Message>
              )}

              {/* Content Metadata */}
              {contentMetadata && (
                <Message
                  info
                  size="small"
                  style={{ marginBottom: '1em' }}
                >
                  <Message.Header>Content Metadata</Message.Header>
                  <p>
                    <strong>Title:</strong> {contentMetadata.title}
                    <br />
                    <strong>Artist:</strong> {contentMetadata.artist}
                    <br />
                    <strong>Type:</strong> {contentMetadata.type} (
                    {contentMetadata.domain})
                  </p>
                </Message>
              )}

              {/* Pod Creation */}
              {contentValidation?.isValid && (
                <div>
                  <Header size="small">Create Content-Linked Pod</Header>

                  <Input
                    onChange={(e) => setNewPodName(e.target.value)}
                    placeholder="Pod name (auto-filled from content)"
                    style={{ marginBottom: '1em', width: '100%' }}
                    value={newPodName}
                  />

                  <div style={{ marginBottom: '1em' }}>
                    <label style={{ marginRight: '1em' }}>Visibility:</label>
                    <select
                      onChange={(e) => setNewPodVisibility(e.target.value)}
                      value={newPodVisibility}
                    >
                      <option value="Unlisted">Unlisted</option>
                      <option value="Listed">Listed</option>
                      <option value="Private">Private</option>
                    </select>
                  </div>

                  <Button
                    color="teal"
                    disabled={!newPodName.trim()}
                    loading={createPodLoading}
                    onClick={() => handleCreateContentLinkedPod()}
                  >
                    Create Content-Linked Pod
                  </Button>
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

  );
};

export default React.memo(PodContentLinkingPanel);
