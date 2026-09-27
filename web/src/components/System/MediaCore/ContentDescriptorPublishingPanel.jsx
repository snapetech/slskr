import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import {
  Card,
  Form,
  Grid,
  Header,
  Icon,
  Input,
  Label,
  List,
  Message,
  TextArea,
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

const ContentDescriptorPublishingPanel = () => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [publishContentId, setPublishContentId] = useMountedState(mountedRef, '');
  const [publishCodec, setPublishCodec] = useMountedState(mountedRef, 'mp3');
  const [publishSize, setPublishSize] = useMountedState(mountedRef, 1_024);
  const [batchContentIds, setBatchContentIds] = useMountedState(mountedRef, '');
  const [updateTargetId, setUpdateTargetId] = useMountedState(mountedRef, '');
  const [updateCodec, setUpdateCodec] = useMountedState(mountedRef, '');
  const [updateSize, setUpdateSize] = useMountedState(mountedRef, '');
  const [updateConfidence, setUpdateConfidence] = useMountedState(mountedRef, '');
  const [publishResult, setPublishResult] = useMountedState(mountedRef, null);
  const [batchPublishResult, setBatchPublishResult] = useMountedState(mountedRef, null);
  const [updateResult, setUpdateResult] = useMountedState(mountedRef, null);
  const [republishResult, setRepublishResult] = useMountedState(mountedRef, null);
  const [publishingStats, setPublishingStats] = useMountedState(mountedRef, null);
  const [publishingDescriptor, setPublishingDescriptor] = useMountedState(mountedRef, false);
  const [publishingBatch, setPublishingBatch] = useMountedState(mountedRef, false);
  const [updatingDescriptor, setUpdatingDescriptor] = useMountedState(mountedRef, false);
  const [republishing, setRepublishing] = useMountedState(mountedRef, false);
  const [loadingStats, setLoadingStats] = useMountedState(mountedRef, false);

  const handlePublishDescriptor = async () => {
    if (!publishContentId.trim()) return;

    try {
      setPublishingDescriptor(true);
      setPublishResult(null);

      const descriptor = {
        codec: publishCodec.trim(),
        confidence: 0.8,
        contentId: publishContentId.trim(),
        sizeBytes: Number.parseInt(publishSize),
      };

      const result = await mediacore.publishContentDescriptor(descriptor);
      setPublishResult(result);
    } catch (error_) {
      setPublishResult({ error: toDisplayError(error_) });
    } finally {
      setPublishingDescriptor(false);
    }
  };

  const handlePublishBatch = async () => {
    const contentIds = batchContentIds
      .split('\n')
      .map((id) => id.trim())
      .filter(Boolean);
    if (!contentIds.length) return;

    try {
      setPublishingBatch(true);
      setBatchPublishResult(null);

      // Create mock descriptors for each ContentID
      const descriptors = contentIds.map((contentId) => ({
        // 1MB mock
        codec: 'mock',

        confidence: 0.8,
        contentId,
        sizeBytes: 1_024 * 1_024,
      }));

      const result =
        await mediacore.publishContentDescriptorsBatch(descriptors);
      setBatchPublishResult(result);
    } catch (error_) {
      setBatchPublishResult({ error: toDisplayError(error_) });
    } finally {
      setPublishingBatch(false);
    }
  };

  const handleUpdateDescriptor = async () => {
    if (!updateTargetId.trim()) return;

    try {
      setUpdatingDescriptor(true);
      setUpdateResult(null);

      const updates = {};
      if (updateCodec.trim()) updates.newCodec = updateCodec.trim();
      if (updateSize.trim()) updates.newSizeBytes = Number.parseInt(updateSize);
      if (updateConfidence.trim())
        updates.newConfidence = Number.parseFloat(updateConfidence);

      if (Object.keys(updates).length === 0) {
        throw new Error('At least one update field is required');
      }

      const result = await mediacore.updateContentDescriptor(
        updateTargetId.trim(),
        updates,
      );
      setUpdateResult(result);
    } catch (error_) {
      setUpdateResult({ error: toDisplayError(error_) });
    } finally {
      setUpdatingDescriptor(false);
    }
  };

  const handleRepublishExpiring = async () => {
    try {
      setRepublishing(true);
      setRepublishResult(null);
      const result = await mediacore.republishExpiringDescriptors();
      setRepublishResult(result);
    } catch (error_) {
      setRepublishResult({ error: toDisplayError(error_) });
    } finally {
      setRepublishing(false);
    }
  };

  const handleLoadPublishingStats = async () => {
    try {
      setLoadingStats(true);
      setPublishingStats(null);
      const result = await mediacore.getPublishingStats();
      setPublishingStats(result);
    } catch (error_) {
      setPublishingStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingStats(false);
    }
  };

  return (
    <>
        {/* Content Descriptor Publishing */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="cloud upload" />
                Publish Content Descriptor
              </Card.Header>
              <Card.Description>
                Publish a content descriptor to the DHT with versioning support
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>ContentID</label>
                  <Input
                    onChange={(e) => setPublishContentId(e.target.value)}
                    placeholder="content:audio:track:mb-12345"
                    value={publishContentId}
                  />
                </Form.Field>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Codec</label>
                    <Input
                      onChange={(e) => setPublishCodec(e.target.value)}
                      placeholder="mp3, flac, etc."
                      value={publishCodec}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Size (bytes)</label>
                    <Input
                      onChange={(e) => setPublishSize(e.target.value)}
                      type="number"
                      value={publishSize}
                    />
                  </Form.Field>
                </Form.Group>
                <Button
                  disabled={!publishContentId.trim() || publishingDescriptor}
                  loading={publishingDescriptor}
                  onClick={handlePublishDescriptor}
                  primary
                >
                  Publish Descriptor
                </Button>
              </Form>

              {publishResult && (
                <div style={{ marginTop: '1em' }}>
                  {publishResult.error ? (
                    <Message error>
                      <p>{publishResult.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>Published Successfully</Message.Header>
                      <p>
                        <strong>ContentID:</strong> {publishResult.contentId}
                        <br />
                        <strong>Version:</strong> {publishResult.version}
                        <br />
                        <strong>TTL:</strong> {publishResult.ttl?.totalMinutes}{' '}
                        minutes
                        <br />
                        <strong>Was Updated:</strong>{' '}
                        {publishResult.wasUpdated ? 'Yes' : 'No'}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Batch Publishing */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="list" />
                Batch Publish Descriptors
              </Card.Header>
              <Card.Description>
                Publish multiple content descriptors simultaneously
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>ContentIDs (one per line)</label>
                  <TextArea
                    onChange={(e) => setBatchContentIds(e.target.value)}
                    placeholder="content:audio:track:mb-12345&#10;content:video:movie:imdb-tt0111161&#10;..."
                    rows={6}
                    value={batchContentIds}
                  />
                </Form.Field>
                <Button
                  disabled={!batchContentIds.trim() || publishingBatch}
                  loading={publishingBatch}
                  onClick={handlePublishBatch}
                  primary
                >
                  Publish Batch
                </Button>
              </Form>

              {batchPublishResult && (
                <div style={{ marginTop: '1em' }}>
                  {batchPublishResult.error ? (
                    <Message error>
                      <p>{batchPublishResult.error}</p>
                    </Message>
                  ) : (
                    <Message info>
                      <Message.Header>Batch Publish Results</Message.Header>
                      <p>
                        <strong>Total Requested:</strong>{' '}
                        {batchPublishResult.totalRequested}
                        <br />
                        <strong>Successfully Published:</strong>{' '}
                        {batchPublishResult.successfullyPublished}
                        <br />
                        <strong>Failed:</strong>{' '}
                        {batchPublishResult.failedToPublish}
                        <br />
                        <strong>Skipped:</strong> {batchPublishResult.skipped}
                        <br />
                        <strong>Duration:</strong>{' '}
                        {batchPublishResult.totalDuration?.totalSeconds.toFixed(
                          2,
                        )}
                        s
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Descriptor Updates */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="edit" />
                Update Descriptor
              </Card.Header>
              <Card.Description>
                Update metadata for an existing published descriptor
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>Target ContentID</label>
                  <Input
                    onChange={(e) => setUpdateTargetId(e.target.value)}
                    placeholder="ContentID to update"
                    value={updateTargetId}
                  />
                </Form.Field>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>New Codec</label>
                    <Input
                      onChange={(e) => setUpdateCodec(e.target.value)}
                      placeholder="Leave empty to keep current"
                      value={updateCodec}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>New Size (bytes)</label>
                    <Input
                      onChange={(e) => setUpdateSize(e.target.value)}
                      placeholder="Leave empty to keep current"
                      value={updateSize}
                    />
                  </Form.Field>
                </Form.Group>
                <Form.Field>
                  <label>New Confidence (0.0-1.0)</label>
                  <Input
                    onChange={(e) => setUpdateConfidence(e.target.value)}
                    placeholder="Leave empty to keep current"
                    value={updateConfidence}
                  />
                </Form.Field>
                <Button
                  disabled={!updateTargetId.trim() || updatingDescriptor}
                  loading={updatingDescriptor}
                  onClick={handleUpdateDescriptor}
                  primary
                >
                  Update Descriptor
                </Button>
              </Form>

              {updateResult && (
                <div style={{ marginTop: '1em' }}>
                  {updateResult.error ? (
                    <Message error>
                      <p>{updateResult.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>Update Successful</Message.Header>
                      <p>
                        <strong>ContentID:</strong> {updateResult.contentId}
                        <br />
                        <strong>Version:</strong> {updateResult.previousVersion}{' '}
                        → {updateResult.newVersion}
                        <br />
                        <strong>Updates Applied:</strong>{' '}
                        {updateResult.appliedUpdates?.join(', ') || 'none'}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Publishing Management */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="cogs" />
                Publishing Management
              </Card.Header>
              <Card.Description>
                Manage published descriptors and monitor publishing status
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button.Group fluid>
                <Button
                  disabled={republishing}
                  loading={republishing}
                  onClick={handleRepublishExpiring}
                >
                  Republish Expiring
                </Button>
                <Button
                  disabled={loadingStats}
                  loading={loadingStats}
                  onClick={handleLoadPublishingStats}
                >
                  Load Stats
                </Button>
              </Button.Group>

              {/* Republish Results */}
              {republishResult && (
                <div style={{ marginTop: '1em' }}>
                  {republishResult.error ? (
                    <Message error>
                      <p>{republishResult.error}</p>
                    </Message>
                  ) : (
                    <Message info>
                      <Message.Header>Republish Results</Message.Header>
                      <p>
                        <strong>Checked:</strong> {republishResult.totalChecked}
                        <br />
                        <strong>Republished:</strong>{' '}
                        {republishResult.republished}
                        <br />
                        <strong>Failed:</strong> {republishResult.failed}
                        <br />
                        <strong>Still Valid:</strong>{' '}
                        {republishResult.stillValid}
                        <br />
                        <strong>Duration:</strong>{' '}
                        {republishResult.duration?.totalSeconds.toFixed(2)}s
                      </p>
                    </Message>
                  )}
                </div>
              )}

              {/* Publishing Stats */}
              {publishingStats && (
                <div style={{ marginTop: '1em' }}>
                  {publishingStats.error ? (
                    <Message error>
                      <p>{publishingStats.error}</p>
                    </Message>
                  ) : (
                    <Message>
                      <Message.Header>Publishing Statistics</Message.Header>
                      <p>
                        <strong>Total Published:</strong>{' '}
                        {publishingStats.totalPublishedDescriptors}
                        <br />
                        <strong>Active Publications:</strong>{' '}
                        {publishingStats.activePublications}
                        <br />
                        <strong>Expiring Soon:</strong>{' '}
                        {publishingStats.expiringSoon}
                        <br />
                        <strong>Average TTL:</strong>{' '}
                        {publishingStats.averageTtlHours?.toFixed(1)} hours
                        <br />
                        <strong>Total Storage:</strong>{' '}
                        {(
                          publishingStats.totalStorageBytes /
                          1_024 /
                          1_024
                        )?.toFixed(1)}{' '}
                        MB
                      </p>
                      {publishingStats.publicationsByDomain &&
                        Object.keys(publishingStats.publicationsByDomain)
                          .length > 0 && (
                          <div style={{ marginTop: '0.5em' }}>
                            <strong>By Domain:</strong>
                            {Object.entries(
                              publishingStats.publicationsByDomain,
                            ).map(([domain, count]) => (
                              <Label
                                key={domain}
                                size="tiny"
                                style={{ margin: '0.1em' }}
                              >
                                {domain}: {count}
                              </Label>
                            ))}
                          </div>
                        )}
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

    </>
  );
};

export default React.memo(ContentDescriptorPublishingPanel);
