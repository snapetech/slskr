import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import {
  Card,
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

const MediaCoreStatisticsDashboardPanel = () => {
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [mediaCoreDashboard, setMediaCoreDashboard] = useMountedState(mountedRef, null);
  const [contentRegistryStats, setContentRegistryStats] = useMountedState(mountedRef, null);
  const [descriptorStats, setDescriptorStats] = useMountedState(mountedRef, null);
  const [fuzzyMatchingStats, setFuzzyMatchingStats] = useMountedState(mountedRef, null);
  const [ipldMappingStats, setIpldMappingStats] = useMountedState(mountedRef, null);
  const [perceptualHashingStats, setPerceptualHashingStats] = useMountedState(mountedRef, null);
  const [metadataPortabilityStats, setMetadataPortabilityStats] =
    useMountedState(mountedRef, null);
  const [contentPublishingStats, setContentPublishingStats] = useMountedState(mountedRef, null);
  const [loadingDashboard, setLoadingDashboard] = useMountedState(mountedRef, false);
  const [loadingRegistryStats, setLoadingRegistryStats] = useMountedState(mountedRef, false);
  const [loadingDescriptorStats, setLoadingDescriptorStats] = useMountedState(mountedRef, false);
  const [loadingFuzzyStats, setLoadingFuzzyStats] = useMountedState(mountedRef, false);
  const [loadingIpldStats, setLoadingIpldStats] = useMountedState(mountedRef, false);
  const [loadingPerceptualStats, setLoadingPerceptualStats] = useMountedState(mountedRef, false);
  const [loadingPortabilityStats, setLoadingPortabilityStats] = useMountedState(mountedRef, false);
  const [loadingPublishingStats, setLoadingPublishingStats] = useMountedState(mountedRef, false);


  const handleLoadMediaCoreDashboard = async () => {
    try {
      setLoadingDashboard(true);
      setMediaCoreDashboard(null);
      const result = await mediacore.getMediaCoreDashboard();
      setMediaCoreDashboard(result);
    } catch (error_) {
      setMediaCoreDashboard({ error: toDisplayError(error_) });
    } finally {
      setLoadingDashboard(false);
    }
  };

  const handleLoadContentRegistryStats = async () => {
    try {
      setLoadingRegistryStats(true);
      setContentRegistryStats(null);
      const result = await mediacore.getContentRegistryStats();
      setContentRegistryStats(result);
    } catch (error_) {
      setContentRegistryStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingRegistryStats(false);
    }
  };

  const handleLoadDescriptorStats = async () => {
    try {
      setLoadingDescriptorStats(true);
      setDescriptorStats(null);
      const result = await mediacore.getDescriptorStats();
      setDescriptorStats(result);
    } catch (error_) {
      setDescriptorStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingDescriptorStats(false);
    }
  };

  const handleLoadFuzzyMatchingStats = async () => {
    try {
      setLoadingFuzzyStats(true);
      setFuzzyMatchingStats(null);
      const result = await mediacore.getFuzzyMatchingStats();
      setFuzzyMatchingStats(result);
    } catch (error_) {
      setFuzzyMatchingStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingFuzzyStats(false);
    }
  };

  const handleLoadIpldMappingStats = async () => {
    try {
      setLoadingIpldStats(true);
      setIpldMappingStats(null);
      const result = await mediacore.getIpldMappingStats();
      setIpldMappingStats(result);
    } catch (error_) {
      setIpldMappingStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingIpldStats(false);
    }
  };

  const handleLoadPerceptualHashingStats = async () => {
    try {
      setLoadingPerceptualStats(true);
      setPerceptualHashingStats(null);
      const result = await mediacore.getPerceptualHashingStats();
      setPerceptualHashingStats(result);
    } catch (error_) {
      setPerceptualHashingStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingPerceptualStats(false);
    }
  };

  const handleLoadMetadataPortabilityStats = async () => {
    try {
      setLoadingPortabilityStats(true);
      setMetadataPortabilityStats(null);
      const result = await mediacore.getMetadataPortabilityStats();
      setMetadataPortabilityStats(result);
    } catch (error_) {
      setMetadataPortabilityStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingPortabilityStats(false);
    }
  };

  const handleLoadContentPublishingStats = async () => {
    try {
      setLoadingPublishingStats(true);
      setContentPublishingStats(null);
      const result = await mediacore.getContentPublishingStats();
      setContentPublishingStats(result);
    } catch (error_) {
      setContentPublishingStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingPublishingStats(false);
    }
  };

  const handleResetMediaCoreStats = async () => {
    if (
      !confirm(
        'Are you sure you want to reset all MediaCore statistics? This cannot be undone.',
      )
    ) {
      return;
    }

    try {
      await mediacore.resetMediaCoreStats();
      // Clear all displayed stats
      setMediaCoreDashboard(null);
      setContentRegistryStats(null);
      setDescriptorStats(null);
      setFuzzyMatchingStats(null);
      setIpldMappingStats(null);
      setPerceptualHashingStats(null);
      setMetadataPortabilityStats(null);
      setContentPublishingStats(null);
      toast.success('MediaCore statistics have been reset');
    } catch (error_) {
      toast.error(`Failed to reset stats: ${toDisplayError(error_)}`);
    }
  };


  return (
    <>
        {/* MediaCore Statistics Dashboard */}
        <Grid.Column width={16}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="chart bar" />
                MediaCore Statistics Dashboard
              </Card.Header>
              <Card.Description>
                Comprehensive overview of all MediaCore system performance and
                usage metrics
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button.Group fluid>
                <Button
                  disabled={loadingDashboard}
                  loading={loadingDashboard}
                  onClick={handleLoadMediaCoreDashboard}
                  primary
                >
                  Load Full Dashboard
                </Button>
                <Button
                  color="red"
                  onClick={handleResetMediaCoreStats}
                >
                  Reset All Stats
                </Button>
              </Button.Group>

              {/* Dashboard Overview */}
              {mediaCoreDashboard && !mediaCoreDashboard.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message info>
                    <Message.Header>System Overview</Message.Header>
                    <p>
                      <strong>Uptime:</strong>{' '}
                      {mediaCoreDashboard.uptime
                        ? `${Math.floor(mediaCoreDashboard.uptime.totalHours)}h ${mediaCoreDashboard.uptime.minutes}m`
                        : 'N/A'}
                      <br />
                      <strong>Last Updated:</strong>{' '}
                      {mediaCoreDashboard.timestamp
                        ? new Date(
                            mediaCoreDashboard.timestamp,
                          ).toLocaleString()
                        : 'N/A'}
                    </p>
                  </Message>

                  {/* System Resources */}
                  {mediaCoreDashboard.systemResources && (
                    <Message>
                      <Message.Header>System Resources</Message.Header>
                      <p>
                        <strong>Working Set:</strong>{' '}
                        {(
                          mediaCoreDashboard.systemResources.workingSetBytes /
                          1_024 /
                          1_024
                        ).toFixed(1)}{' '}
                        MB
                        <br />
                        <strong>Private Memory:</strong>{' '}
                        {(
                          mediaCoreDashboard.systemResources
                            .privateMemoryBytes /
                          1_024 /
                          1_024
                        ).toFixed(1)}{' '}
                        MB
                        <br />
                        <strong>GC Memory:</strong>{' '}
                        {(
                          mediaCoreDashboard.systemResources
                            .gcTotalMemoryBytes /
                          1_024 /
                          1_024
                        ).toFixed(1)}{' '}
                        MB
                        <br />
                        <strong>Thread Count:</strong>{' '}
                        {mediaCoreDashboard.systemResources.threadCount}
                      </p>
                    </Message>
                  )}
                </div>
              )}

              {/* Error Display */}
              {mediaCoreDashboard?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>Failed to load dashboard: {mediaCoreDashboard.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Content Registry Stats */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="database" />
                Content Registry
              </Card.Header>
              <Card.Description>
                Content ID mappings and domain statistics
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button
                disabled={loadingRegistryStats}
                fluid
                loading={loadingRegistryStats}
                onClick={handleLoadContentRegistryStats}
              >
                Load Registry Stats
              </Button>

              {contentRegistryStats && !contentRegistryStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message success>
                    <Message.Header>Registry Overview</Message.Header>
                    <p>
                      <strong>Total Mappings:</strong>{' '}
                      {contentRegistryStats.totalMappings}
                      <br />
                      <strong>Domains:</strong>{' '}
                      {contentRegistryStats.totalDomains}
                      <br />
                      <strong>Avg Mappings/Domain:</strong>{' '}
                      {contentRegistryStats.averageMappingsPerDomain.toFixed(1)}
                    </p>
                    {contentRegistryStats.mappingsByDomain &&
                      Object.keys(contentRegistryStats.mappingsByDomain)
                        .length > 0 && (
                        <div style={{ marginTop: '0.5em' }}>
                          <strong>Mappings by Domain:</strong>
                          {Object.entries(
                            contentRegistryStats.mappingsByDomain,
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
                </div>
              )}

              {contentRegistryStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>{contentRegistryStats.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Descriptor Stats */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="search" />
                Descriptor Retrieval
              </Card.Header>
              <Card.Description>
                Cache performance and retrieval statistics
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button
                disabled={loadingDescriptorStats}
                fluid
                loading={loadingDescriptorStats}
                onClick={handleLoadDescriptorStats}
              >
                Load Descriptor Stats
              </Button>

              {descriptorStats && !descriptorStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Cache Performance</Message.Header>
                    <p>
                      <strong>Total Retrievals:</strong>{' '}
                      {descriptorStats.totalRetrievals}
                      <br />
                      <strong>Cache Hits:</strong> {descriptorStats.cacheHits}
                      <br />
                      <strong>Cache Misses:</strong>{' '}
                      {descriptorStats.cacheMisses}
                      <br />
                      <strong>Hit Ratio:</strong>{' '}
                      {(descriptorStats.cacheHitRatio * 100).toFixed(1)}%<br />
                      <strong>Avg Retrieval Time:</strong>{' '}
                      {descriptorStats.averageRetrievalTime?.totalMilliseconds.toFixed(
                        0,
                      )}
                      ms
                      <br />
                      <strong>Active Cache Entries:</strong>{' '}
                      {descriptorStats.activeCacheEntries}
                      <br />
                      <strong>Cache Size:</strong>{' '}
                      {(descriptorStats.cacheSizeBytes / 1_024).toFixed(1)} KB
                    </p>
                  </Message>
                </div>
              )}

              {descriptorStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>{descriptorStats.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Fuzzy Matching Stats */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="magic" />
                Fuzzy Matching
              </Card.Header>
              <Card.Description>
                Similarity detection and accuracy metrics
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button
                disabled={loadingFuzzyStats}
                fluid
                loading={loadingFuzzyStats}
                onClick={handleLoadFuzzyMatchingStats}
              >
                Load Fuzzy Stats
              </Button>

              {fuzzyMatchingStats && !fuzzyMatchingStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Matching Performance</Message.Header>
                    <p>
                      <strong>Total Matches:</strong>{' '}
                      {fuzzyMatchingStats.totalMatches}
                      <br />
                      <strong>Success Rate:</strong>{' '}
                      {(fuzzyMatchingStats.successRate * 100).toFixed(1)}%<br />
                      <strong>Avg Confidence:</strong>{' '}
                      {(
                        fuzzyMatchingStats.averageConfidenceScore * 100
                      ).toFixed(1)}
                      %<br />
                      <strong>Avg Match Time:</strong>{' '}
                      {fuzzyMatchingStats.averageMatchingTime?.totalMilliseconds.toFixed(
                        0,
                      )}
                      ms
                    </p>
                    {fuzzyMatchingStats.accuracyByAlgorithm &&
                      Object.keys(fuzzyMatchingStats.accuracyByAlgorithm)
                        .length > 0 && (
                        <div style={{ marginTop: '0.5em' }}>
                          <strong>Algorithm Accuracy:</strong>
                          {Object.entries(
                            fuzzyMatchingStats.accuracyByAlgorithm,
                          ).map(([algorithm, stats]) => (
                            <div
                              key={algorithm}
                              style={{ margin: '0.2em 0' }}
                            >
                              <small>
                                {algorithm}: F1={stats.f1Score.toFixed(2)},
                                Precision={stats.precision.toFixed(2)}
                              </small>
                            </div>
                          ))}
                        </div>
                      )}
                  </Message>
                </div>
              )}

              {fuzzyMatchingStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>{fuzzyMatchingStats.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Perceptual Hashing Stats */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="hashtag" />
                Perceptual Hashing
              </Card.Header>
              <Card.Description>
                Hash computation performance and accuracy
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button
                disabled={loadingPerceptualStats}
                fluid
                loading={loadingPerceptualStats}
                onClick={handleLoadPerceptualHashingStats}
              >
                Load Hashing Stats
              </Button>

              {perceptualHashingStats && !perceptualHashingStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Hashing Performance</Message.Header>
                    <p>
                      <strong>Total Hashes:</strong>{' '}
                      {perceptualHashingStats.totalHashesComputed}
                      <br />
                      <strong>Avg Computation Time:</strong>{' '}
                      {perceptualHashingStats.averageComputationTime?.totalMilliseconds.toFixed(
                        0,
                      )}
                      ms
                      <br />
                      <strong>Overall Accuracy:</strong>{' '}
                      {(perceptualHashingStats.overallAccuracy * 100).toFixed(
                        1,
                      )}
                      %<br />
                      <strong>Duplicates Detected:</strong>{' '}
                      {perceptualHashingStats.duplicateHashesDetected}
                    </p>
                    {perceptualHashingStats.statsByAlgorithm &&
                      Object.keys(perceptualHashingStats.statsByAlgorithm)
                        .length > 0 && (
                        <div style={{ marginTop: '0.5em' }}>
                          <strong>Algorithm Breakdown:</strong>
                          {Object.entries(
                            perceptualHashingStats.statsByAlgorithm,
                          ).map(([algorithm, stats]) => (
                            <div
                              key={algorithm}
                              style={{ margin: '0.2em 0' }}
                            >
                              <small>
                                {algorithm}: {stats.hashesComputed} hashes,{' '}
                                {stats.averageTime.totalMilliseconds.toFixed(0)}
                                ms avg, {stats.accuracy.toFixed(2)} accuracy
                              </small>
                            </div>
                          ))}
                        </div>
                      )}
                  </Message>
                </div>
              )}

              {perceptualHashingStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>{perceptualHashingStats.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* IPLD Mapping Stats */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="sitemap" />
                IPLD Mapping
              </Card.Header>
              <Card.Description>
                Graph structure and link statistics
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button
                disabled={loadingIpldStats}
                fluid
                loading={loadingIpldStats}
                onClick={handleLoadIpldMappingStats}
              >
                Load IPLD Stats
              </Button>

              {ipldMappingStats && !ipldMappingStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Graph Statistics</Message.Header>
                    <p>
                      <strong>Total Links:</strong>{' '}
                      {ipldMappingStats.totalLinks}
                      <br />
                      <strong>Total Nodes:</strong>{' '}
                      {ipldMappingStats.totalNodes}
                      <br />
                      <strong>Total Graphs:</strong>{' '}
                      {ipldMappingStats.totalGraphs}
                      <br />
                      <strong>Connectivity Ratio:</strong>{' '}
                      {(ipldMappingStats.graphConnectivityRatio * 100).toFixed(
                        1,
                      )}
                      %<br />
                      <strong>Broken Links:</strong>{' '}
                      {ipldMappingStats.brokenLinksDetected}
                      <br />
                      <strong>Avg Traversal Time:</strong>{' '}
                      {ipldMappingStats.averageTraversalTime?.totalMilliseconds.toFixed(
                        0,
                      )}
                      ms
                    </p>
                  </Message>
                </div>
              )}

              {ipldMappingStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>{ipldMappingStats.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Metadata Portability Stats */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="exchange" />
                Metadata Portability
              </Card.Header>
              <Card.Description>
                Export/import operations and conflict resolution
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button
                disabled={loadingPortabilityStats}
                fluid
                loading={loadingPortabilityStats}
                onClick={handleLoadMetadataPortabilityStats}
              >
                Load Portability Stats
              </Button>

              {metadataPortabilityStats && !metadataPortabilityStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Portability Metrics</Message.Header>
                    <p>
                      <strong>Total Exports:</strong>{' '}
                      {metadataPortabilityStats.totalExports}
                      <br />
                      <strong>Total Imports:</strong>{' '}
                      {metadataPortabilityStats.totalImports}
                      <br />
                      <strong>Import Success Rate:</strong>{' '}
                      {(
                        metadataPortabilityStats.importSuccessRate * 100
                      ).toFixed(1)}
                      %<br />
                      <strong>Data Transferred:</strong>{' '}
                      {(
                        metadataPortabilityStats.totalDataTransferred / 1_024
                      ).toFixed(1)}{' '}
                      KB
                      <br />
                      <strong>Avg Export Time:</strong>{' '}
                      {metadataPortabilityStats.averageExportTime?.totalMilliseconds.toFixed(
                        0,
                      )}
                      ms
                      <br />
                      <strong>Avg Import Time:</strong>{' '}
                      {metadataPortabilityStats.averageImportTime?.totalMilliseconds.toFixed(
                        0,
                      )}
                      ms
                    </p>
                  </Message>
                </div>
              )}

              {metadataPortabilityStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>{metadataPortabilityStats.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Content Publishing Stats */}
        <Grid.Column width={16}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="cloud upload" />
                Content Publishing
              </Card.Header>
              <Card.Description>
                DHT publishing performance and publication management
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Button
                disabled={loadingPublishingStats}
                fluid
                loading={loadingPublishingStats}
                onClick={handleLoadContentPublishingStats}
              >
                Load Publishing Stats
              </Button>

              {contentPublishingStats && !contentPublishingStats.error && (
                <div style={{ marginTop: '1em' }}>
                  <Message>
                    <Message.Header>Publishing Overview</Message.Header>
                    <p>
                      <strong>Total Published:</strong>{' '}
                      {contentPublishingStats.totalPublished}
                      <br />
                      <strong>Active Publications:</strong>{' '}
                      {contentPublishingStats.activePublications}
                      <br />
                      <strong>Expired Publications:</strong>{' '}
                      {contentPublishingStats.expiredPublications}
                      <br />
                      <strong>Success Rate:</strong>{' '}
                      {(
                        contentPublishingStats.publicationSuccessRate * 100
                      ).toFixed(1)}
                      %<br />
                      <strong>Republished:</strong>{' '}
                      {contentPublishingStats.republishedDescriptors}
                      <br />
                      <strong>Failed:</strong>{' '}
                      {contentPublishingStats.failedPublications}
                      <br />
                      <strong>Avg Publish Time:</strong>{' '}
                      {contentPublishingStats.averagePublishTime?.totalMilliseconds.toFixed(
                        0,
                      )}
                      ms
                    </p>
                    {contentPublishingStats.publicationsByDomain &&
                      Object.keys(contentPublishingStats.publicationsByDomain)
                        .length > 0 && (
                        <div style={{ marginTop: '0.5em' }}>
                          <strong>Publications by Domain:</strong>
                          {Object.entries(
                            contentPublishingStats.publicationsByDomain,
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
                </div>
              )}

              {contentPublishingStats?.error && (
                <Message
                  error
                  style={{ marginTop: '1em' }}
                >
                  <p>{contentPublishingStats.error}</p>
                </Message>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

    </>
  );
};

export default React.memo(MediaCoreStatisticsDashboardPanel);
