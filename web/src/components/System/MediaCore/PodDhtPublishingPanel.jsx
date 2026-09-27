import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import PodWorkflowNotice from './PodWorkflowNotice';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import {
  Card,
  Form,
  Grid,
  Header,
  Icon,
  Label,
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

const PodDhtPublishingPanel = ({ visible = true }) => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [podToPublish, setPodToPublish] = useMountedState(mountedRef, '');
  const [publishingPod, setPublishingPod] = useMountedState(mountedRef, false);
  const [podPublishingResult, setPodPublishingResult] = useMountedState(mountedRef, null);
  const [podMetadataToRetrieve, setPodMetadataToRetrieve] = useMountedState(mountedRef, '');
  const [retrievingPodMetadata, setRetrievingPodMetadata] = useMountedState(mountedRef, false);
  const [podMetadataResult, setPodMetadataResult] = useMountedState(mountedRef, null);
  const [podToUnpublish, setPodToUnpublish] = useMountedState(mountedRef, '');
  const [unpublishingPod, setUnpublishingPod] = useMountedState(mountedRef, false);
  const [podUnpublishResult, setPodUnpublishResult] = useMountedState(mountedRef, null);
  const [podPublishingStats, setPodPublishingStats] = useMountedState(mountedRef, null);
  const [loadingPodStats, setLoadingPodStats] = useMountedState(mountedRef, false);

  const handlePublishPod = async () => {
    if (!podToPublish.trim()) {
      toast.warning('Please enter pod JSON data');
      return;
    }

    try {
      setPublishingPod(true);
      setPodPublishingResult(null);
      const pod = JSON.parse(podToPublish);
      const result = await mediacore.publishPod(pod);
      setPodPublishingResult(result);
      setPodToPublish('');
    } catch (error_) {
      setPodPublishingResult({ error: toDisplayError(error_) });
    } finally {
      setPublishingPod(false);
    }
  };

  const handleRetrievePodMetadata = async () => {
    if (!podMetadataToRetrieve.trim()) {
      toast.warning('Please enter a pod ID');
      return;
    }

    try {
      setRetrievingPodMetadata(true);
      setPodMetadataResult(null);
      const result = await mediacore.getPublishedPodMetadata(
        podMetadataToRetrieve,
      );
      setPodMetadataResult(result);
    } catch (error_) {
      setPodMetadataResult({ error: toDisplayError(error_) });
    } finally {
      setRetrievingPodMetadata(false);
    }
  };

  const handleUnpublishPod = async () => {
    if (!podToUnpublish.trim()) {
      toast.warning('Please enter a pod ID');
      return;
    }

    if (
      !confirm(`Are you sure you want to unpublish pod "${podToUnpublish}"?`)
    ) {
      return;
    }

    try {
      setUnpublishingPod(true);
      setPodUnpublishResult(null);
      const result = await mediacore.unpublishPod(podToUnpublish);
      setPodUnpublishResult(result);
      setPodToUnpublish('');
    } catch (error_) {
      setPodUnpublishResult({ error: toDisplayError(error_) });
    } finally {
      setUnpublishingPod(false);
    }
  };

  const handleLoadPodPublishingStats = async () => {
    try {
      setLoadingPodStats(true);
      setPodPublishingStats(null);
      const result = await mediacore.getPodPublishingStats();
      setPodPublishingStats(result);
    } catch (error_) {
      setPodPublishingStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingPodStats(false);
    }
  };

  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="podcore-dht-publishing" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="podcast" />
                PodCore DHT Publishing
              </Card.Header>
              <Card.Description>
                Publish and manage pod metadata on the decentralized DHT for
                discovery
              </Card.Description>
              <PodWorkflowNotice title="Publishes pod metadata">
                Publishing or updating a listed pod can make pod identifiers,
                tags, focus content IDs, and descriptive metadata discoverable
                by other mesh participants.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Publish Pod */}
            <Card.Content>
              <Header size="small">Publish Pod to DHT</Header>
              <Form>
                <Form.TextArea
                  label="Pod JSON"
                  onChange={(e) => setPodToPublish(e.target.value)}
                  placeholder='{"id": {"value": "pod:artist:mb:daft-punk-hash"}, "displayName": "Daft Punk Fans", "visibility": "Listed", "focusType": "ContentId", "focusContentId": {"domain": "audio", "type": "artist", "id": "daft-punk-hash"}, "tags": ["electronic", "french-house"], "createdAt": "2024-01-01T00:00:00Z", "createdBy": "alice", "metadata": {"description": "A community for Daft Punk fans", "memberCount": 150}}'
                  rows={6}
                  value={podToPublish}
                />
                <Button
                  disabled={publishingPod || !podToPublish.trim()}
                  loading={publishingPod}
                  onClick={handlePublishPod}
                  primary
                >
                  Publish Pod
                </Button>
              </Form>

              {podPublishingResult && (
                <div style={{ marginTop: '1em' }}>
                  {podPublishingResult.error ? (
                    <Message error>
                      <p>Failed to publish pod: {podPublishingResult.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>
                        Pod Published Successfully
                      </Message.Header>
                      <p>
                        <strong>Pod ID:</strong>{' '}
                        {podPublishingResult.podId?.value ||
                          podPublishingResult.podId}
                        <br />
                        <strong>DHT Key:</strong> {podPublishingResult.dhtKey}
                        <br />
                        <strong>Published:</strong>{' '}
                        {new Date(
                          podPublishingResult.publishedAt,
                        ).toLocaleString()}
                        <br />
                        <strong>Expires:</strong>{' '}
                        {new Date(
                          podPublishingResult.expiresAt,
                        ).toLocaleString()}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>

            <Card.Content>
              <Grid>
                <Grid.Column width={8}>
                  {/* Retrieve Pod Metadata */}
                  <Header size="small">Retrieve Pod Metadata</Header>
                  <Form>
                    <Form.Input
                      label="Pod ID"
                      onChange={(e) => setPodMetadataToRetrieve(e.target.value)}
                      placeholder="pod:artist:mb:daft-punk-hash"
                      value={podMetadataToRetrieve}
                    />
                    <Button
                      disabled={
                        retrievingPodMetadata || !podMetadataToRetrieve.trim()
                      }
                      fluid
                      loading={retrievingPodMetadata}
                      onClick={handleRetrievePodMetadata}
                    >
                      Retrieve Metadata
                    </Button>
                  </Form>

                  {podMetadataResult && (
                    <div style={{ marginTop: '1em' }}>
                      {podMetadataResult.error ? (
                        <Message error>
                          <p>
                            Failed to retrieve metadata:{' '}
                            {podMetadataResult.error}
                          </p>
                        </Message>
                      ) : podMetadataResult.found ? (
                        <Message success>
                          <Message.Header>
                            Pod Metadata Retrieved
                          </Message.Header>
                          <p>
                            <strong>Pod ID:</strong>{' '}
                            {podMetadataResult.podId?.value ||
                              podMetadataResult.podId}
                            <br />
                            <strong>Signature Valid:</strong>{' '}
                            {podMetadataResult.isValidSignature ? 'Yes' : 'No'}
                            <br />
                            <strong>Retrieved:</strong>{' '}
                            {new Date(
                              podMetadataResult.retrievedAt,
                            ).toLocaleString()}
                            <br />
                            <strong>Expires:</strong>{' '}
                            {new Date(
                              podMetadataResult.expiresAt,
                            ).toLocaleString()}
                            <br />
                            <strong>Display Name:</strong>{' '}
                            {podMetadataResult.publishedPod?.displayName}
                            <br />
                            <strong>Members:</strong>{' '}
                            {podMetadataResult.publishedPod?.metadata
                              ?.memberCount || 'Unknown'}
                          </p>
                        </Message>
                      ) : (
                        <Message warning>
                          <p>Pod not found in DHT</p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>

                <Grid.Column width={8}>
                  {/* Unpublish Pod */}
                  <Header size="small">Unpublish Pod from DHT</Header>
                  <Form>
                    <Form.Input
                      label="Pod ID"
                      onChange={(e) => setPodToUnpublish(e.target.value)}
                      placeholder="pod:artist:mb:daft-punk-hash"
                      value={podToUnpublish}
                    />
                    <Button
                      color="red"
                      disabled={unpublishingPod || !podToUnpublish.trim()}
                      fluid
                      loading={unpublishingPod}
                      onClick={handleUnpublishPod}
                    >
                      Unpublish Pod
                    </Button>
                  </Form>

                  {podUnpublishResult && (
                    <div style={{ marginTop: '1em' }}>
                      {podUnpublishResult.error ? (
                        <Message error>
                          <p>
                            Failed to unpublish pod: {podUnpublishResult.error}
                          </p>
                        </Message>
                      ) : (
                        <Message success>
                          <p>Pod unpublished successfully from DHT</p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>
              </Grid>
            </Card.Content>

            {/* Pod Publishing Statistics */}
            <Card.Content>
              <Button.Group fluid>
                <Button
                  disabled={loadingPodStats}
                  loading={loadingPodStats}
                  onClick={handleLoadPodPublishingStats}
                  primary
                >
                  Load Pod Publishing Stats
                </Button>
              </Button.Group>

              {podPublishingStats && !podPublishingStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Pod Publishing Statistics</Message.Header>
                    <p>
                      <strong>Total Published:</strong>{' '}
                      {podPublishingStats.totalPublished}
                      <br />
                      <strong>Active Publications:</strong>{' '}
                      {podPublishingStats.activePublications}
                      <br />
                      <strong>Expired Publications:</strong>{' '}
                      {podPublishingStats.expiredPublications}
                      <br />
                      <strong>Failed Publications:</strong>{' '}
                      {podPublishingStats.failedPublications}
                      <br />
                      <strong>Avg Publish Time:</strong>{' '}
                      {podPublishingStats.averagePublishTime
                        ? `${podPublishingStats.averagePublishTime.totalMilliseconds.toFixed(0)}ms`
                        : 'N/A'}
                      <br />
                      <strong>Last Operation:</strong>{' '}
                      {podPublishingStats.lastPublishOperation
                        ? new Date(
                            podPublishingStats.lastPublishOperation,
                          ).toLocaleString()
                        : 'Never'}
                    </p>
                    {podPublishingStats.publicationsByVisibility &&
                      Object.keys(podPublishingStats.publicationsByVisibility)
                        .length > 0 && (
                        <div style={{ marginTop: '0.5em' }}>
                          <strong>Publications by Visibility:</strong>
                          {Object.entries(
                            podPublishingStats.publicationsByVisibility,
                          ).map(([visibility, count]) => (
                            <Label
                              key={visibility}
                              size="tiny"
                              style={{ margin: '0.1em' }}
                            >
                              {visibility}: {count}
                            </Label>
                          ))}
                        </div>
                      )}
                  </Message>
                </div>
              )}

              {podPublishingStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>
                    Failed to load pod publishing stats:{' '}
                    {podPublishingStats.error}
                  </p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

  );
};

export default React.memo(PodDhtPublishingPanel);
