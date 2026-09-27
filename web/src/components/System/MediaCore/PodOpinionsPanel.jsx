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

const opinionQueryKey = (podId, contentId) =>
  JSON.stringify([podId.trim(), contentId.trim()]);

const PodOpinionsPanel = ({ visible = true }) => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Pod Opinion Management states
  const [opinionPodId, setOpinionPodId] = useMountedState(mountedRef, '');
  const [opinionContentId, setOpinionContentId] = useMountedState(mountedRef, '');
  const [opinionVariantHash, setOpinionVariantHash] = useMountedState(mountedRef, '');
  const [opinionScore, setOpinionScore] = useMountedState(mountedRef, 5);
  const [opinionNote, setOpinionNote] = useMountedState(mountedRef, '');
  const [opinions, setOpinions] = useMountedState(mountedRef, []);
  const [opinionsLoadedFor, setOpinionsLoadedFor] = useMountedState(
    mountedRef,
    null,
  );
  const [opinionsError, setOpinionsError] = useMountedState(mountedRef, null);
  const [opinionStatistics, setOpinionStatistics] = useMountedState(mountedRef, null);
  const [opinionStatisticsLoadedFor, setOpinionStatisticsLoadedFor] =
    useMountedState(mountedRef, null);
  const [opinionStatisticsError, setOpinionStatisticsError] = useMountedState(
    mountedRef,
    null,
  );
  const [publishOpinionLoading, setPublishOpinionLoading] = useMountedState(mountedRef, false);
  const [getOpinionsLoading, setGetOpinionsLoading] = useMountedState(mountedRef, false);
  const [getStatsLoading, setGetStatsLoading] = useMountedState(mountedRef, false);
  const [refreshOpinionsLoading, setRefreshOpinionsLoading] = useMountedState(mountedRef, false);

  // Pod Opinion Aggregation states
  const [aggregatedOpinions, setAggregatedOpinions] = useMountedState(mountedRef, null);
  const [aggregatedOpinionsLoadedFor, setAggregatedOpinionsLoadedFor] =
    useMountedState(mountedRef, null);
  const [aggregatedOpinionsError, setAggregatedOpinionsError] = useMountedState(
    mountedRef,
    null,
  );
  const [memberAffinities, setMemberAffinities] = useMountedState(mountedRef, {});
  const [memberAffinitiesLoadedFor, setMemberAffinitiesLoadedFor] = useMountedState(
    mountedRef,
    null,
  );
  const [memberAffinitiesError, setMemberAffinitiesError] = useMountedState(
    mountedRef,
    null,
  );
  const [consensusRecommendations, setConsensusRecommendations] = useMountedState(mountedRef, []);
  const [consensusRecommendationsLoadedFor, setConsensusRecommendationsLoadedFor] =
    useMountedState(mountedRef, null);
  const [consensusRecommendationsError, setConsensusRecommendationsError] =
    useMountedState(mountedRef, null);
  const [getAggregatedLoading, setGetAggregatedLoading] = useMountedState(mountedRef, false);
  const [getAffinitiesLoading, setGetAffinitiesLoading] = useMountedState(mountedRef, false);
  const [getRecommendationsLoading, setGetRecommendationsLoading] =
    useMountedState(mountedRef, false);
  const [updateAffinitiesLoading, setUpdateAffinitiesLoading] = useMountedState(mountedRef, false);


    const resetOpinionReadState = () => {
      setOpinionsLoadedFor(null);
      setOpinionsError(null);
      setOpinionStatisticsLoadedFor(null);
      setOpinionStatisticsError(null);
      setAggregatedOpinionsLoadedFor(null);
      setAggregatedOpinionsError(null);
      setMemberAffinitiesLoadedFor(null);
      setMemberAffinitiesError(null);
      setConsensusRecommendationsLoadedFor(null);
      setConsensusRecommendationsError(null);
    };


  const handlePublishOpinion = async () => {
    if (
      !opinionPodId.trim() ||
      !opinionContentId.trim() ||
      !opinionVariantHash.trim()
    ) {
      toast.error('Pod ID, Content ID, and Variant Hash are required');
      return;
    }

    if (opinionScore < 0 || opinionScore > 10) {
      toast.error('Score must be between 0 and 10');
      return;
    }

    try {
      setPublishOpinionLoading(true);
      const opinion = {
        contentId: opinionContentId.trim(),
        note: opinionNote.trim(),
        score: opinionScore,
        senderPeerId: 'current-user',
        variantHash: opinionVariantHash.trim(), // Get from session when available
      };

      await mediacore.publishOpinion(opinionPodId.trim(), opinion);
      toast.success('Opinion published successfully');

      // Reset form
      setOpinionContentId('');
      setOpinionVariantHash('');
      setOpinionScore(5);
      setOpinionNote('');

      // Refresh opinions if we're viewing them
      if (opinionContentId) {
        await handleGetOpinions();
      }
    } catch (error_) {
      toast.error(`Failed to publish opinion: ${toDisplayError(error_)}`);
    } finally {
      setPublishOpinionLoading(false);
    }
  };

  const handleGetOpinions = async () => {
    const requestedPodId = opinionPodId.trim();
    const requestedContentId = opinionContentId.trim();
    if (!requestedPodId || !requestedContentId) {
      toast.error('Pod ID and Content ID are required');
      return;
    }

    try {
      setGetOpinionsLoading(true);
      setOpinionsError(null);
      const result = await mediacore.getContentOpinions(
        requestedPodId,
        requestedContentId,
      );
      if (!Array.isArray(result)) {
        throw new Error('Invalid opinions response');
      }
      setOpinions(result);
      setOpinionsLoadedFor(opinionQueryKey(requestedPodId, requestedContentId));
    } catch (error_) {
      const message = toDisplayError(error_, 'Failed to get opinions');
      setOpinionsError(message);
      toast.error(`Failed to get opinions: ${message}`);
    } finally {
      setGetOpinionsLoading(false);
    }
  };

  const handleGetOpinionStatistics = async () => {
    const requestedPodId = opinionPodId.trim();
    const requestedContentId = opinionContentId.trim();
    if (!requestedPodId || !requestedContentId) {
      toast.error('Pod ID and Content ID are required');
      return;
    }

    try {
      setGetStatsLoading(true);
      setOpinionStatisticsError(null);
      const stats = await mediacore.getOpinionStatistics(
        requestedPodId,
        requestedContentId,
      );
      if (
        stats === null ||
        typeof stats !== 'object' ||
        Array.isArray(stats)
      ) {
        throw new Error('Invalid opinion statistics response');
      }
      setOpinionStatistics(stats);
      setOpinionStatisticsLoadedFor(
        opinionQueryKey(requestedPodId, requestedContentId),
      );
    } catch (error_) {
      const message = toDisplayError(
        error_,
        'Failed to get opinion statistics',
      );
      setOpinionStatisticsError(message);
      toast.error(`Failed to get opinion statistics: ${message}`);
    } finally {
      setGetStatsLoading(false);
    }
  };

  const handleRefreshOpinions = async () => {
    if (!opinionPodId.trim()) {
      toast.error('Pod ID is required');
      return;
    }

    try {
      setRefreshOpinionsLoading(true);
      const result = await mediacore.refreshPodOpinions(opinionPodId.trim());
      toast.success(`Refreshed ${result.opinionsRefreshed} opinions`);

      // Refresh current view
      if (opinionContentId) {
        await Promise.all([handleGetOpinions(), handleGetOpinionStatistics()]);
      }
    } catch (error_) {
      toast.error(`Failed to refresh opinions: ${toDisplayError(error_)}`);
    } finally {
      setRefreshOpinionsLoading(false);
    }
  };

  // Pod Opinion Aggregation handlers
  const handleGetAggregatedOpinions = async () => {
    const requestedPodId = opinionPodId.trim();
    const requestedContentId = opinionContentId.trim();
    if (!requestedPodId || !requestedContentId) {
      toast.error('Pod ID and Content ID are required');
      return;
    }

    try {
      setGetAggregatedLoading(true);
      setAggregatedOpinionsError(null);
      const aggregated = await mediacore.getAggregatedOpinions(
        requestedPodId,
        requestedContentId,
      );
      if (
        aggregated === null ||
        typeof aggregated !== 'object' ||
        Array.isArray(aggregated)
      ) {
        throw new Error('Invalid aggregated opinions response');
      }
      setAggregatedOpinions(aggregated);
      setAggregatedOpinionsLoadedFor(
        opinionQueryKey(requestedPodId, requestedContentId),
      );
    } catch (error_) {
      const message = toDisplayError(
        error_,
        'Failed to get aggregated opinions',
      );
      setAggregatedOpinionsError(message);
      toast.error(`Failed to get aggregated opinions: ${message}`);
    } finally {
      setGetAggregatedLoading(false);
    }
  };

  const handleGetMemberAffinities = async () => {
    const requestedPodId = opinionPodId.trim();
    if (!requestedPodId) {
      toast.error('Pod ID is required');
      return;
    }

    try {
      setGetAffinitiesLoading(true);
      setMemberAffinitiesError(null);
      const affinities = await mediacore.getMemberAffinities(
        requestedPodId,
      );
      if (
        affinities === null ||
        typeof affinities !== 'object' ||
        Array.isArray(affinities)
      ) {
        throw new Error('Invalid member affinities response');
      }
      setMemberAffinities(affinities);
      setMemberAffinitiesLoadedFor(requestedPodId);
    } catch (error_) {
      const message = toDisplayError(
        error_,
        'Failed to get member affinities',
      );
      setMemberAffinitiesError(message);
      toast.error(`Failed to get member affinities: ${message}`);
    } finally {
      setGetAffinitiesLoading(false);
    }
  };

  const handleGetConsensusRecommendations = async () => {
    const requestedPodId = opinionPodId.trim();
    const requestedContentId = opinionContentId.trim();
    if (!requestedPodId || !requestedContentId) {
      toast.error('Pod ID and Content ID are required');
      return;
    }

    try {
      setGetRecommendationsLoading(true);
      setConsensusRecommendationsError(null);
      const recommendations = await mediacore.getConsensusRecommendations(
        requestedPodId,
        requestedContentId,
      );
      if (!Array.isArray(recommendations)) {
        throw new Error('Invalid consensus recommendations response');
      }
      setConsensusRecommendations(recommendations);
      setConsensusRecommendationsLoadedFor(
        opinionQueryKey(requestedPodId, requestedContentId),
      );
    } catch (error_) {
      const message = toDisplayError(
        error_,
        'Failed to get consensus recommendations',
      );
      setConsensusRecommendationsError(message);
      toast.error(`Failed to get consensus recommendations: ${message}`);
    } finally {
      setGetRecommendationsLoading(false);
    }
  };

  const handleUpdateMemberAffinities = async () => {
    if (!opinionPodId.trim()) {
      toast.error('Pod ID is required');
      return;
    }

    try {
      setUpdateAffinitiesLoading(true);
      const result = await mediacore.updateMemberAffinities(
        opinionPodId.trim(),
      );
      toast.success(`Updated affinities for ${result.membersUpdated} members`);

      // Refresh affinities display
      await handleGetMemberAffinities();
    } catch (error_) {
      toast.error(`Failed to update member affinities: ${toDisplayError(error_)}`);
    } finally {
      setUpdateAffinitiesLoading(false);
    }
  };


    const currentOpinionQueryKey = opinionQueryKey(
      opinionPodId,
      opinionContentId,
    );
    const hasCurrentOpinions =
      opinionsLoadedFor === currentOpinionQueryKey;
    const hasCurrentOpinionStatistics =
      opinionStatisticsLoadedFor === currentOpinionQueryKey;
    const hasCurrentAggregatedOpinions =
      aggregatedOpinionsLoadedFor === currentOpinionQueryKey;
    const hasCurrentMemberAffinities =
      memberAffinitiesLoadedFor === opinionPodId.trim();
    const hasCurrentConsensusRecommendations =
      consensusRecommendationsLoadedFor === currentOpinionQueryKey;


  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-opinion-management" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="star" />
                Pod Opinion Management
              </Card.Header>
              <Card.Description>
                Publish and view opinions on content variants within pods for
                quality assessment and community feedback
              </Card.Description>
              <PodWorkflowNotice title="Publishes opinion data">
                Opinion publishing can expose peer preferences, ratings,
                confidence values, and content identifiers to other pod
                participants.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Opinion Management */}
            <Card.Content>
              <Header size="small">Opinion Management</Header>

              {/* Pod Selection */}
              <Input
                onChange={(e) => {
                  const nextPodId = e.target.value;
                  setOpinionPodId(nextPodId);
                  if (
                    opinionQueryKey(nextPodId, opinionContentId) !==
                    opinionQueryKey(opinionPodId, opinionContentId)
                  ) {
                    resetOpinionReadState();
                  }
                }}
                placeholder="Pod ID"
                style={{ marginBottom: '1em', width: '100%' }}
                value={opinionPodId}
              />

              <Button
                color="blue"
                disabled={!opinionPodId.trim()}
                loading={refreshOpinionsLoading}
                onClick={() => handleRefreshOpinions()}
                style={{ marginBottom: '1em' }}
              >
                Refresh Pod Opinions
              </Button>

              {/* Content Opinions */}
              <Header size="tiny">Content Opinions</Header>
              <Input
                onChange={(e) => {
                  const nextContentId = e.target.value;
                  setOpinionContentId(nextContentId);
                  if (
                    opinionQueryKey(opinionPodId, nextContentId) !==
                    opinionQueryKey(opinionPodId, opinionContentId)
                  ) {
                    resetOpinionReadState();
                  }
                }}
                placeholder="Content ID (e.g., content:audio:album:mb-id)"
                style={{ marginBottom: '1em', width: '100%' }}
                value={opinionContentId}
              />

              {opinionsError && (
                <Message
                  data-testid="media-core-opinions-error"
                  negative
                  size="small"
                >
                  {opinionsError}
                  {hasCurrentOpinions && opinions.length > 0 && (
                    <div>Showing last successfully loaded opinions.</div>
                  )}
                </Message>
              )}

              <div style={{ marginBottom: '1em' }}>
                <Button
                  color="teal"
                  disabled={!opinionPodId.trim() || !opinionContentId.trim()}
                  loading={getOpinionsLoading}
                  onClick={() => handleGetOpinions()}
                  style={{ marginRight: '0.5em' }}
                >
                  Get Opinions
                </Button>

                <Button
                  color="purple"
                  disabled={!opinionPodId.trim() || !opinionContentId.trim()}
                  loading={getStatsLoading}
                  onClick={() => handleGetOpinionStatistics()}
                >
                  Get Statistics
                </Button>
              </div>

              {/* Opinion Statistics */}
              {opinionStatisticsError && (
                <Message
                  data-testid="media-core-opinion-statistics-error"
                  negative
                  size="small"
                >
                  {opinionStatisticsError}
                  {hasCurrentOpinionStatistics && opinionStatistics && (
                    <div>Showing last successfully loaded statistics.</div>
                  )}
                </Message>
              )}

              {hasCurrentOpinionStatistics && opinionStatistics && (
                <Message
                  info
                  style={{ marginBottom: '1em' }}
                >
                  <Message.Header>Opinion Statistics</Message.Header>
                  <p>
                    <strong>Total Opinions:</strong>{' '}
                    {opinionStatistics.totalOpinions}
                    <br />
                    <strong>Unique Variants:</strong>{' '}
                    {opinionStatistics.uniqueVariants}
                    <br />
                    <strong>Average Score:</strong>{' '}
                    {opinionStatistics.averageScore.toFixed(1)}
                    <br />
                    <strong>Score Range:</strong> {opinionStatistics.minScore} -{' '}
                    {opinionStatistics.maxScore}
                    <br />
                    <strong>Last Updated:</strong>{' '}
                    {new Date(opinionStatistics.lastUpdated).toLocaleString()}
                  </p>
                </Message>
              )}

              {/* Opinions List */}
              {hasCurrentOpinions && opinions.length > 0 && (
                <div style={{ marginBottom: '1em' }}>
                  <Header size="tiny">Opinions ({opinions.length})</Header>
                  {opinions.map((opinion, index) => (
                    <Card
                      key={index}
                      style={{ marginBottom: '0.5em' }}
                    >
                      <Card.Content style={{ padding: '0.5em' }}>
                        <div
                          style={{
                            display: 'flex',
                            justifyContent: 'space-between',
                          }}
                        >
                          <div>
                            <strong>Variant:</strong>{' '}
                            {opinion.variantHash.slice(0, 8)}...
                            <br />
                            <strong>Score:</strong> {opinion.score}/10
                            {opinion.note && (
                              <>
                                <br />
                                <strong>Note:</strong> {opinion.note}
                              </>
                            )}
                          </div>
                          <small>{opinion.senderPeerId}</small>
                        </div>
                      </Card.Content>
                    </Card>
                  ))}
                </div>
              )}

              {hasCurrentOpinions &&
                opinions.length === 0 &&
                opinionContentId &&
                !getOpinionsLoading &&
                !opinionsError && (
                <Message info size="small">
                  No opinions found for {opinionContentId}.
                </Message>
              )}

              {/* Publish Opinion */}
              <Header size="small">Publish New Opinion</Header>

              <Input
                onChange={(e) => setOpinionVariantHash(e.target.value)}
                placeholder="Variant Hash"
                style={{ marginBottom: '1em', width: '100%' }}
                value={opinionVariantHash}
              />

              <div style={{ marginBottom: '1em' }}>
                <label style={{ marginRight: '1em' }}>Score (0-10):</label>
                <input
                  max="10"
                  min="0"
                  onChange={(e) =>
                    setOpinionScore(Number.parseFloat(e.target.value))
                  }
                  step="0.5"
                  style={{ width: '200px' }}
                  type="range"
                  value={opinionScore}
                />
                <span style={{ marginLeft: '1em' }}>{opinionScore}/10</span>
              </div>

              <Input
                onChange={(e) => setOpinionNote(e.target.value)}
                placeholder="Optional note about this variant"
                style={{ marginBottom: '1em', width: '100%' }}
                value={opinionNote}
              />

              <Button
                color="green"
                disabled={
                  !opinionPodId.trim() ||
                  !opinionContentId.trim() ||
                  !opinionVariantHash.trim()
                }
                loading={publishOpinionLoading}
                onClick={() => handlePublishOpinion()}
              >
                Publish Opinion
              </Button>
            </Card.Content>

            {/* Opinion Aggregation */}
            <Card.Content>
              <Header size="small">Opinion Aggregation & Consensus</Header>

              <div style={{ marginBottom: '1em' }}>
                <Button
                  color="purple"
                  disabled={!opinionPodId.trim() || !opinionContentId.trim()}
                  loading={getAggregatedLoading}
                  onClick={() => handleGetAggregatedOpinions()}
                  style={{ marginRight: '0.5em' }}
                >
                  Get Aggregated Opinions
                </Button>

                <Button
                  color="blue"
                  disabled={!opinionPodId.trim()}
                  loading={getAffinitiesLoading}
                  onClick={() => handleGetMemberAffinities()}
                  style={{ marginRight: '0.5em' }}
                >
                  Get Member Affinities
                </Button>

                <Button
                  color="teal"
                  disabled={!opinionPodId.trim() || !opinionContentId.trim()}
                  loading={getRecommendationsLoading}
                  onClick={() => handleGetConsensusRecommendations()}
                  style={{ marginRight: '0.5em' }}
                >
                  Get Recommendations
                </Button>

                <Button
                  color="orange"
                  disabled={!opinionPodId.trim()}
                  loading={updateAffinitiesLoading}
                  onClick={() => handleUpdateMemberAffinities()}
                >
                  Update Affinities
                </Button>
              </div>

              {/* Aggregated Opinions */}
              {aggregatedOpinionsError && (
                <Message
                  data-testid="media-core-aggregated-opinions-error"
                  negative
                  size="small"
                >
                  {aggregatedOpinionsError}
                  {hasCurrentAggregatedOpinions && aggregatedOpinions && (
                    <div>Showing last successfully loaded aggregate.</div>
                  )}
                </Message>
              )}

              {hasCurrentAggregatedOpinions && aggregatedOpinions && (
                <div style={{ marginBottom: '1em' }}>
                  <Header size="tiny">Aggregated Opinion Results</Header>
                  <Message info>
                    <strong>Weighted Average:</strong>{' '}
                    {aggregatedOpinions.weightedAverageScore.toFixed(2)}/10
                    <br />
                    <strong>Unweighted Average:</strong>{' '}
                    {aggregatedOpinions.unweightedAverageScore.toFixed(2)}/10
                    <br />
                    <strong>Consensus Strength:</strong>{' '}
                    {(aggregatedOpinions.consensusStrength * 100).toFixed(1)}%
                    <br />
                    <strong>Total Opinions:</strong>{' '}
                    {aggregatedOpinions.totalOpinions}
                    <br />
                    <strong>Unique Variants:</strong>{' '}
                    {aggregatedOpinions.uniqueVariants}
                    <br />
                    <strong>Contributing Members:</strong>{' '}
                    {aggregatedOpinions.contributingMembers}
                  </Message>

                  {/* Variant Breakdown */}
                  {aggregatedOpinions.variantAggregates.length > 0 && (
                    <div style={{ marginTop: '1em' }}>
                      <Header size="tiny">Variant Analysis</Header>
                      {aggregatedOpinions.variantAggregates.map(
                        (variant, index) => (
                          <Card
                            key={index}
                            style={{ marginBottom: '0.5em' }}
                          >
                            <Card.Content style={{ padding: '0.5em' }}>
                              <div>
                                <strong>Variant:</strong>{' '}
                                {variant.variantHash.slice(0, 8)}...
                                <br />
                                <strong>Weighted Score:</strong>{' '}
                                {variant.weightedAverageScore.toFixed(2)}/10
                                <br />
                                <strong>Unweighted Score:</strong>{' '}
                                {variant.unweightedAverageScore.toFixed(2)}/10
                                <br />
                                <strong>Opinions:</strong>{' '}
                                {variant.opinionCount}
                                <br />
                                <strong>Agreement:</strong>{' '}
                                {(
                                  1 -
                                  variant.scoreStandardDeviation / 5
                                ).toFixed(2)}{' '}
                                (lower std dev = higher agreement)
                              </div>
                            </Card.Content>
                          </Card>
                        ),
                      )}
                    </div>
                  )}
                </div>
              )}

              {/* Consensus Recommendations */}
              {consensusRecommendationsError && (
                <Message
                  data-testid="media-core-consensus-recommendations-error"
                  negative
                  size="small"
                >
                  {consensusRecommendationsError}
                  {hasCurrentConsensusRecommendations &&
                    consensusRecommendations.length > 0 && (
                    <div>
                      Showing last successfully loaded recommendations.
                    </div>
                  )}
                </Message>
              )}

              {hasCurrentConsensusRecommendations &&
                consensusRecommendations.length > 0 && (
                <div style={{ marginBottom: '1em' }}>
                  <Header size="tiny">Consensus Recommendations</Header>
                  {consensusRecommendations.map((rec, index) => (
                    <Card
                      key={index}
                      style={{
                        borderLeft:
                          rec.recommendation === 'StronglyRecommended'
                            ? '5px solid #21ba45'
                            : rec.recommendation === 'Recommended'
                              ? '5px solid #2185d0'
                              : rec.recommendation === 'Neutral'
                                ? '5px solid #fbbd08'
                                : rec.recommendation === 'NotRecommended'
                                  ? '5px solid #f2711c'
                                  : '5px solid #db2828',
                        marginBottom: '0.5em',
                      }}
                    >
                      <Card.Content style={{ padding: '0.5em' }}>
                        <div>
                          <strong>Variant:</strong>{' '}
                          {rec.variantHash.slice(0, 8)}...
                          <br />
                          <strong>Recommendation:</strong>{' '}
                          {rec.recommendation
                            .replaceAll(/([A-Z])/g, ' $1')
                            .trim()}
                          <br />
                          <strong>Consensus Score:</strong>{' '}
                          {(rec.consensusScore * 100).toFixed(1)}%<br />
                          <strong>Reasoning:</strong> {rec.reasoning}
                          <br />
                          <small>
                            <strong>Factors:</strong>{' '}
                            {rec.supportingFactors.join(', ')}
                          </small>
                        </div>
                      </Card.Content>
                    </Card>
                  ))}
                </div>
              )}

              {/* Member Affinities */}
              {memberAffinitiesError && (
                <Message
                  data-testid="media-core-member-affinities-error"
                  negative
                  size="small"
                >
                  {memberAffinitiesError}
                  {hasCurrentMemberAffinities &&
                    Object.keys(memberAffinities).length > 0 && (
                    <div>Showing last successfully loaded affinities.</div>
                  )}
                </Message>
              )}

              {hasCurrentMemberAffinities &&
                Object.keys(memberAffinities).length > 0 && (
                <div style={{ marginBottom: '1em' }}>
                  <Header size="tiny">
                    Member Affinities ({Object.keys(memberAffinities).length})
                  </Header>
                  {Object.entries(memberAffinities).map(
                    ([peerId, affinity], index) => (
                      <Card
                        key={index}
                        style={{ marginBottom: '0.5em' }}
                      >
                        <Card.Content style={{ padding: '0.5em' }}>
                          <div>
                            <strong>Peer:</strong> {peerId.slice(0, 8)}...
                            <br />
                            <strong>Affinity Score:</strong>{' '}
                            {(affinity.affinityScore * 100).toFixed(1)}%<br />
                            <strong>Trust Score:</strong>{' '}
                            {(affinity.trustScore * 100).toFixed(1)}%<br />
                            <strong>Messages:</strong> {affinity.messageCount}
                            <br />
                            <strong>Opinions:</strong> {affinity.opinionCount}
                            <br />
                            <small>
                              Last Activity:{' '}
                              {new Date(
                                affinity.lastActivity,
                              ).toLocaleDateString()}
                            </small>
                          </div>
                        </Card.Content>
                      </Card>
                    ),
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

  );
};

export default React.memo(PodOpinionsPanel);
