import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import {
  Card,
  Checkbox,
  Dropdown,
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

const ContentDescriptorRetrievalPanel = ({ setVerificationResult }) => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [retrievalResult, setRetrievalResult] = useMountedState(mountedRef, null);
  const [batchRetrievalResult, setBatchRetrievalResult] = useMountedState(mountedRef, null);
  const [queryResult, setQueryResult] = useMountedState(mountedRef, null);
  const [descriptorVerificationResult] = useMountedState(mountedRef, null);
  const [retrievalStats, setRetrievalStats] = useMountedState(mountedRef, null);
  const [retrieveContentId, setRetrieveContentId] = useMountedState(mountedRef, '');
  const [batchRetrieveContentIds, setBatchRetrieveContentIds] = useMountedState(mountedRef, '');
  const [queryDomain, setQueryDomain] = useMountedState(mountedRef, 'audio');
  const [queryType, setQueryType] = useMountedState(mountedRef, '');
  const [queryMaxResults, setQueryMaxResults] = useMountedState(mountedRef, 50);
  const [verifyDescriptor, setVerifyDescriptor] = useMountedState(mountedRef, '');
  const [bypassCache, setBypassCache] = useMountedState(mountedRef, false);
  const [retrievingDescriptor, setRetrievingDescriptor] = useMountedState(mountedRef, false);
  const [retrievingBatch, setRetrievingBatch] = useMountedState(mountedRef, false);
  const [queryingDescriptors, setQueryingDescriptors] = useMountedState(mountedRef, false);
  const [verifyingDescriptor, setVerifyingDescriptor] = useMountedState(mountedRef, false);
  const [loadingRetrievalStats, setLoadingRetrievalStats] = useMountedState(mountedRef, false);

  const handleRetrieveDescriptor = async () => {
    if (!retrieveContentId.trim()) return;

    try {
      setRetrievingDescriptor(true);
      setRetrievalResult(null);
      const result = await mediacore.retrieveContentDescriptor(
        retrieveContentId.trim(),
        bypassCache,
      );
      setRetrievalResult(result);
    } catch (error_) {
      setRetrievalResult({ error: toDisplayError(error_) });
    } finally {
      setRetrievingDescriptor(false);
    }
  };

  const handleRetrieveBatch = async () => {
    const contentIds = batchRetrieveContentIds
      .split('\n')
      .map((id) => id.trim())
      .filter(Boolean);
    if (!contentIds.length) return;

    try {
      setRetrievingBatch(true);
      setBatchRetrievalResult(null);
      const result =
        await mediacore.retrieveContentDescriptorsBatch(contentIds);
      setBatchRetrievalResult(result);
    } catch (error_) {
      setBatchRetrievalResult({ error: toDisplayError(error_) });
    } finally {
      setRetrievingBatch(false);
    }
  };

  const handleQueryDescriptors = async () => {
    if (!queryDomain.trim()) return;

    try {
      setQueryingDescriptors(true);
      setQueryResult(null);
      const result = await mediacore.queryDescriptorsByDomain(
        queryDomain.trim(),
        queryType.trim() || null,
        Number.parseInt(queryMaxResults),
      );
      setQueryResult(result);
    } catch (error_) {
      setQueryResult({ error: toDisplayError(error_) });
    } finally {
      setQueryingDescriptors(false);
    }
  };

  const handleVerifyDescriptor = async () => {
    if (!verifyDescriptor.trim()) return;

    try {
      setVerifyingDescriptor(true);
      setVerificationResult(null);

      let descriptor;
      try {
        descriptor = JSON.parse(verifyDescriptor.trim());
      } catch {
        throw new Error('Invalid JSON format for descriptor');
      }

      const result = await mediacore.verifyContentDescriptor(descriptor);
      setVerificationResult(result);
    } catch (error_) {
      setVerificationResult({ error: toDisplayError(error_) });
    } finally {
      setVerifyingDescriptor(false);
    }
  };

  const handleLoadRetrievalStats = async () => {
    try {
      setLoadingRetrievalStats(true);
      setRetrievalStats(null);
      const result = await mediacore.getRetrievalStats();
      setRetrievalStats(result);
    } catch (error_) {
      setRetrievalStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingRetrievalStats(false);
    }
  };

  const handleClearRetrievalCache = async () => {
    try {
      const result = await mediacore.clearRetrievalCache();
      // Reload stats to reflect changes
      await handleLoadRetrievalStats();
      toast.success(
        `Cache cleared: ${result.entriesCleared} entries, ${result.bytesFreed} bytes freed`,
      );
    } catch (error_) {
      toast.error(`Failed to clear cache: ${toDisplayError(error_)}`);
    }
  };

  return (
    <>
        {/* Descriptor Retrieval */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="search" />
                Retrieve Content Descriptor
              </Card.Header>
              <Card.Description>
                Retrieve content descriptors from the DHT by ContentID
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>ContentID</label>
                  <Input
                    onChange={(e) => setRetrieveContentId(e.target.value)}
                    placeholder="content:audio:track:mb-12345"
                    value={retrieveContentId}
                  />
                </Form.Field>
                <Form.Field>
                  <Checkbox
                    checked={bypassCache}
                    label="Bypass cache (force fresh retrieval)"
                    onChange={(e, { checked }) => setBypassCache(checked)}
                  />
                </Form.Field>
                <Button
                  disabled={!retrieveContentId.trim() || retrievingDescriptor}
                  loading={retrievingDescriptor}
                  onClick={handleRetrieveDescriptor}
                  primary
                >
                  Retrieve Descriptor
                </Button>
              </Form>

              {retrievalResult && (
                <div style={{ marginTop: '1em' }}>
                  {retrievalResult.error ? (
                    <Message error>
                      <p>{retrievalResult.error}</p>
                    </Message>
                  ) : !retrievalResult.found ? (
                    <Message warning>
                      <p>
                        Content descriptor not found for:{' '}
                        {retrievalResult.contentId || retrieveContentId}
                      </p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>Descriptor Retrieved</Message.Header>
                      <p>
                        <strong>ContentID:</strong>{' '}
                        {retrievalResult.descriptor?.contentId}
                        <br />
                        <strong>From Cache:</strong>{' '}
                        {retrievalResult.fromCache ? 'Yes' : 'No'}
                        <br />
                        <strong>Retrieved:</strong>{' '}
                        {new Date(retrievalResult.retrievedAt).toLocaleString()}
                        <br />
                        <strong>Duration:</strong>{' '}
                        {retrievalResult.retrievalDuration?.totalMilliseconds.toFixed(
                          0,
                        )}
                        ms
                        <br />
                        <strong>Verified:</strong>{' '}
                        {retrievalResult.verification?.isValid ? 'Yes' : 'No'}
                        {retrievalResult.verification?.warnings?.length > 0 && (
                          <span> (with warnings)</span>
                        )}
                      </p>
                      <details>
                        <summary>View Descriptor JSON</summary>
                        <pre
                          style={{
                            fontSize: '0.8em',
                            maxHeight: '200px',
                            overflow: 'auto',
                          }}
                        >
                          {JSON.stringify(retrievalResult.descriptor, null, 2)}
                        </pre>
                      </details>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Batch Retrieval */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="list alternate" />
                Batch Descriptor Retrieval
              </Card.Header>
              <Card.Description>
                Retrieve multiple content descriptors simultaneously
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>ContentIDs (one per line)</label>
                  <TextArea
                    onChange={(e) => setBatchRetrieveContentIds(e.target.value)}
                    placeholder="content:audio:track:mb-12345&#10;content:video:movie:imdb-tt0111161&#10;..."
                    rows={6}
                    value={batchRetrieveContentIds}
                  />
                </Form.Field>
                <Button
                  disabled={!batchRetrieveContentIds.trim() || retrievingBatch}
                  loading={retrievingBatch}
                  onClick={handleRetrieveBatch}
                  primary
                >
                  Retrieve Batch
                </Button>
              </Form>

              {batchRetrievalResult && (
                <div style={{ marginTop: '1em' }}>
                  {batchRetrievalResult.error ? (
                    <Message error>
                      <p>{batchRetrievalResult.error}</p>
                    </Message>
                  ) : (
                    <Message info>
                      <Message.Header>Batch Retrieval Results</Message.Header>
                      <p>
                        <strong>Requested:</strong>{' '}
                        {batchRetrievalResult.requested}
                        <br />
                        <strong>Found:</strong> {batchRetrievalResult.found}
                        <br />
                        <strong>Failed:</strong> {batchRetrievalResult.failed}
                        <br />
                        <strong>Duration:</strong>{' '}
                        {batchRetrievalResult.totalDuration?.totalSeconds.toFixed(
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

        {/* Domain Query */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="filter" />
                Query by Domain
              </Card.Header>
              <Card.Description>
                Query content descriptors by domain and optional type
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Domain</label>
                    <Dropdown
                      onChange={(e, { value }) => setQueryDomain(value)}
                      options={[
                        { key: 'audio', text: 'Audio', value: 'audio' },
                        { key: 'video', text: 'Video', value: 'video' },
                        { key: 'image', text: 'Image', value: 'image' },
                        { key: 'text', text: 'Text', value: 'text' },
                        {
                          key: 'application',
                          text: 'Application',
                          value: 'application',
                        },
                      ]}
                      selection
                      value={queryDomain}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Type (optional)</label>
                    <Input
                      onChange={(e) => setQueryType(e.target.value)}
                      placeholder="track, album, movie, etc."
                      value={queryType}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Max Results</label>
                    <Input
                      max="1000"
                      min="1"
                      onChange={(e) => setQueryMaxResults(e.target.value)}
                      type="number"
                      value={queryMaxResults}
                    />
                  </Form.Field>
                </Form.Group>
                <Button
                  disabled={!queryDomain.trim() || queryingDescriptors}
                  loading={queryingDescriptors}
                  onClick={handleQueryDescriptors}
                  primary
                >
                  Query Domain
                </Button>
              </Form>

              {queryResult && (
                <div style={{ marginTop: '1em' }}>
                  {queryResult.error ? (
                    <Message error>
                      <p>{queryResult.error}</p>
                    </Message>
                  ) : (
                    <Message>
                      <Message.Header>Query Results</Message.Header>
                      <p>
                        <strong>Domain:</strong> {queryResult.domain}
                        {queryResult.type && (
                          <span>
                            {' '}
                            | <strong>Type:</strong> {queryResult.type}
                          </span>
                        )}
                        <br />
                        <strong>Found:</strong> {queryResult.totalFound}
                        <br />
                        <strong>Query Time:</strong>{' '}
                        {queryResult.queryDuration?.totalMilliseconds.toFixed(
                          0,
                        )}
                        ms
                        <br />
                        <strong>Has More:</strong>{' '}
                        {queryResult.hasMoreResults ? 'Yes' : 'No'}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Descriptor Verification */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="shield" />
                Descriptor Verification
              </Card.Header>
              <Card.Description>
                Verify descriptor signature and freshness
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>Descriptor JSON</label>
                  <TextArea
                    onChange={(e) => setVerifyDescriptor(e.target.value)}
                    placeholder="Paste descriptor JSON to verify..."
                    rows={8}
                    value={verifyDescriptor}
                  />
                </Form.Field>
                <Button
                  disabled={!verifyDescriptor.trim() || verifyingDescriptor}
                  loading={verifyingDescriptor}
                  onClick={handleVerifyDescriptor}
                  primary
                >
                  Verify Descriptor
                </Button>
              </Form>

              {descriptorVerificationResult && (
                <div style={{ marginTop: '1em' }}>
                  {descriptorVerificationResult.error ? (
                    <Message error>
                      <p>{descriptorVerificationResult.error}</p>
                    </Message>
                  ) : (
                    <Message
                      success={descriptorVerificationResult.isValid}
                      warning={!descriptorVerificationResult.isValid}
                    >
                      <Message.Header>
                        Verification Result:{' '}
                        {descriptorVerificationResult.isValid
                          ? 'Valid'
                          : 'Invalid'}
                      </Message.Header>
                      <p>
                        <strong>Signature Valid:</strong>{' '}
                        {descriptorVerificationResult.signatureValid
                          ? 'Yes'
                          : 'No'}
                        <br />
                        <strong>Freshness Valid:</strong>{' '}
                        {descriptorVerificationResult.freshnessValid
                          ? 'Yes'
                          : 'No'}
                        <br />
                        <strong>Age:</strong>{' '}
                        {descriptorVerificationResult.age?.totalMinutes.toFixed(
                          1,
                        )}{' '}
                        minutes
                      </p>
                      {descriptorVerificationResult.warnings?.length > 0 && (
                        <div>
                          <strong>Warnings:</strong>
                          <List bulleted>
                            {descriptorVerificationResult.warnings.map(
                              (warning, index) => (
                                <List.Item key={index}>{warning}</List.Item>
                              ),
                            )}
                          </List>
                        </div>
                      )}
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Retrieval Management */}
        <Grid.Column width={16}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="chart line" />
                Retrieval Management
              </Card.Header>
              <Card.Description>
                Monitor retrieval performance and manage cache
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button.Group fluid>
                <Button
                  disabled={loadingRetrievalStats}
                  loading={loadingRetrievalStats}
                  onClick={handleLoadRetrievalStats}
                >
                  Load Stats
                </Button>
                <Button onClick={handleClearRetrievalCache}>Clear Cache</Button>
              </Button.Group>

              {/* Retrieval Stats */}
              {retrievalStats && (
                <div style={{ marginTop: '1em' }}>
                  {retrievalStats.error ? (
                    <Message error>
                      <p>{retrievalStats.error}</p>
                    </Message>
                  ) : (
                    <Message>
                      <Message.Header>Retrieval Statistics</Message.Header>
                      <p>
                        <strong>Total Retrievals:</strong>{' '}
                        {retrievalStats.totalRetrievals}
                        <br />
                        <strong>Cache Hits:</strong> {retrievalStats.cacheHits}
                        <br />
                        <strong>Cache Misses:</strong>{' '}
                        {retrievalStats.cacheMisses}
                        <br />
                        <strong>Hit Ratio:</strong>{' '}
                        {(retrievalStats.cacheHitRatio * 100).toFixed(1)}%<br />
                        <strong>Avg Retrieval Time:</strong>{' '}
                        {retrievalStats.averageRetrievalTime?.totalMilliseconds.toFixed(
                          0,
                        )}
                        ms
                        <br />
                        <strong>Active Cache Entries:</strong>{' '}
                        {retrievalStats.activeCacheEntries}
                        <br />
                        <strong>Cache Size:</strong>{' '}
                        {(retrievalStats.cacheSizeBytes / 1_024).toFixed(1)} KB
                      </p>
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

export default React.memo(ContentDescriptorRetrievalPanel);
