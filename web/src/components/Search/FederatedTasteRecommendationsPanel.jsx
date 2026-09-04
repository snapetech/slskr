// <copyright file="FederatedTasteRecommendationsPanel.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import {
  fetchTasteRecommendations,
  previewTasteRecommendationGraph,
  promoteTasteRecommendationToWishlist,
  subscribeTasteRecommendationReleaseRadar,
} from '../../lib/tasteRecommendations';
import { toDisplayError } from '../../lib/errors';
import { useMountedRef } from '../../lib/useMountedRef';
import React, { useRef, useState } from 'react';
import { toast } from 'react-toastify';
import {
  Button,
  Checkbox,
  Form,
  Header,
  Icon,
  Label,
  List,
  Message,
  Popup,
  Segment,
} from 'semantic-ui-react';

const getCreator = (workRef = {}) => workRef.creator || workRef.Creator || '';

const getTitle = (workRef = {}) => workRef.title || workRef.Title || 'Untitled recommendation';

const getSearchText = (workRef = {}) =>
  [getCreator(workRef), getTitle(workRef)].filter(Boolean).join(' ') || getTitle(workRef);

const FederatedTasteRecommendationsPanel = ({ disabled }) => {
  const [error, setError] = useState('');
  const [graphPreview, setGraphPreview] = useState(null);
  const [includeSoulseekRecommendations, setIncludeSoulseekRecommendations] = useState(false);
  const [includeSourceActors, setIncludeSourceActors] = useState(false);
  const [limit, setLimit] = useState(20);
  const [loading, setLoading] = useState(false);
  const [minimumTrustedSources, setMinimumTrustedSources] = useState(2);
  const [recommendations, setRecommendations] = useState([]);
  const [status, setStatus] = useState('');
  const [summary, setSummary] = useState(null);
  const mountedRef = useMountedRef();
  const loadRequestIdRef = useRef(0);
  const actionRequestIdRef = useRef(0);

  const loadRecommendations = async () => {
    const requestId = ++loadRequestIdRef.current;
    if (!mountedRef.current || disabled) return;
    setError('');
    setLoading(true);
    try {
      const response = await fetchTasteRecommendations({
        includeSoulseekRecommendations,
        includeSourceActors,
        limit: Number(limit) || 20,
        minimumTrustedSources: Number(minimumTrustedSources) || 2,
      });
      if (
        !mountedRef.current ||
        requestId !== loadRequestIdRef.current
      ) {
        return;
      }
      setSummary(response.data);
      setRecommendations(response.data?.recommendations || []);
      setStatus(
        `Loaded ${response.data?.recommendations?.length || 0} privacy-filtered recommendation${
          response.data?.recommendations?.length === 1 ? '' : 's'
        }.`,
      );
    } catch (loadError) {
      if (
        mountedRef.current &&
        requestId === loadRequestIdRef.current
      ) {
        setError(
          toDisplayError(
            loadError,
            'Unable to load federated taste recommendations.',
          ),
        );
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === loadRequestIdRef.current
      ) {
        setLoading(false);
      }
    }
  };

  const promoteToWishlist = async (recommendation) => {
    const requestId = ++actionRequestIdRef.current;
    if (!mountedRef.current || disabled) return;
    try {
      const response = await promoteTasteRecommendationToWishlist({
        note: 'Promoted from federated taste recommendation review.',
        workRef: recommendation.workRef,
      });
      if (
        !mountedRef.current ||
        requestId !== actionRequestIdRef.current
      ) {
        return;
      }
      setStatus(response.data?.message || `Promoted ${getTitle(recommendation.workRef)} to Wishlist.`);
    } catch (promoteError) {
      if (
        mountedRef.current &&
        requestId === actionRequestIdRef.current
      ) {
        toast.error(toDisplayError(promoteError, 'Unable to promote recommendation.'));
      }
    }
  };

  const subscribeRadar = async (recommendation) => {
    const requestId = ++actionRequestIdRef.current;
    if (!mountedRef.current || disabled) return;
    try {
      const response = await subscribeTasteRecommendationReleaseRadar({
        scope: 'trusted',
        workRef: recommendation.workRef,
      });
      if (
        !mountedRef.current ||
        requestId !== actionRequestIdRef.current
      ) {
        return;
      }
      setStatus(response.data?.message || `Subscribed ${getCreator(recommendation.workRef)} to Release Radar.`);
    } catch (subscribeError) {
      if (
        mountedRef.current &&
        requestId === actionRequestIdRef.current
      ) {
        toast.error(toDisplayError(subscribeError, 'Unable to subscribe to Release Radar.'));
      }
    }
  };

  const previewGraph = async (recommendation) => {
    const requestId = ++actionRequestIdRef.current;
    if (!mountedRef.current || disabled) return;
    try {
      const response = await previewTasteRecommendationGraph({
        workRef: recommendation.workRef,
      });
      if (
        !mountedRef.current ||
        requestId !== actionRequestIdRef.current
      ) {
        return;
      }
      setGraphPreview(response.data);
      setStatus(`Previewed Discovery Graph for ${getTitle(recommendation.workRef)}.`);
    } catch (previewError) {
      if (
        mountedRef.current &&
        requestId === actionRequestIdRef.current
      ) {
        toast.error(toDisplayError(previewError, 'Unable to preview Discovery Graph.'));
      }
    }
  };

  if (disabled) {
    return (
      <Segment raised>
        <Header as="h4">Federated Taste Recommendations</Header>
        <p>Connect to the server to load privacy-filtered recommendations.</p>
      </Segment>
    );
  }

  return (
    <Segment
      loading={loading}
      raised
    >
      <Header as="h4">Federated Taste Recommendations</Header>
      <Form>
        <Form.Group widths="equal">
          <Form.Input
            aria-label="Taste recommendation limit"
            label="Limit"
            min={1}
            onChange={(event) => setLimit(event.target.value)}
            type="number"
            value={limit}
          />
          <Form.Input
            aria-label="Minimum trusted taste sources"
            label="Minimum trusted sources"
            min={1}
            onChange={(event) => setMinimumTrustedSources(event.target.value)}
            type="number"
            value={minimumTrustedSources}
          />
          <Form.Field>
            <label>Reveal source actors</label>
            <Checkbox
              aria-label="Reveal federated recommendation source actors"
              checked={includeSourceActors}
              onChange={(_event, data) => setIncludeSourceActors(data.checked)}
              toggle
            />
          </Form.Field>
          <Form.Field>
            <label>Include Soulseek native</label>
            <Checkbox
              aria-label="Include Soulseek native recommendations"
              checked={includeSoulseekRecommendations}
              onChange={(_event, data) => setIncludeSoulseekRecommendations(data.checked)}
              toggle
            />
          </Form.Field>
        </Form.Group>
        <Popup
          content="Load privacy-filtered recommendations from followed federated actors. Optionally include raw native Soulseek recommendation seeds."
          position="top center"
          trigger={
            <Button
              onClick={loadRecommendations}
              primary
              type="button"
            >
              <Icon name="users" />
              Load Recommendations
            </Button>
          }
        />
      </Form>
      {status && <Message compact size="mini">{status}</Message>}
      {error && <Message compact error size="mini">{String(error)}</Message>}
      {summary && (
        <div className="search-acquisition-profile-strip">
          <Label basic>
            Trusted actors
            <Label.Detail>{summary.trustedActorCount}</Label.Detail>
          </Label>
          <Label basic>
            Candidates
            <Label.Detail>{summary.candidateCount}</Label.Detail>
          </Label>
          <Label basic>
            Minimum sources
            <Label.Detail>{summary.minimumTrustedSources}</Label.Detail>
          </Label>
        </div>
      )}
      {graphPreview && (
        <Message compact size="mini">
          Graph preview: {graphPreview.nodeCount} nodes, {graphPreview.edgeCount} edges.
        </Message>
      )}
      <List divided relaxed>
        {recommendations.map((recommendation) => {
          const title = getTitle(recommendation.workRef);
          const creator = getCreator(recommendation.workRef);

          return (
            <List.Item key={recommendation.workRef?.id || `${creator}-${title}`}>
              <List.Icon name="user friends" />
              <List.Content>
                <List.Header>{[creator, title].filter(Boolean).join(' - ') || title}</List.Header>
                <List.Description>
                  {recommendation.trustedSourceCount} trusted sources · score {Math.round((recommendation.score || 0) * 100)}%
                </List.Description>
                {(recommendation.reasons || []).map((reason) => (
                  <Label basic key={reason}>{reason}</Label>
                ))}
                {includeSourceActors && recommendation.sourceActors?.length > 0 && (
                  <div className="search-acquisition-profile-summary">
                    {recommendation.sourceActors.join(', ')}
                  </div>
                )}
                <Popup
                  content="Promote this WorkRef to Wishlist through the backend review handoff without starting a download."
                  position="top center"
                  trigger={
                    <Button
                      aria-label={`Promote ${title} taste recommendation to Wishlist`}
                      onClick={() => promoteToWishlist(recommendation)}
                      size="mini"
                      type="button"
                    >
                      <Icon name="heart" />
                      Wishlist
                    </Button>
                  }
                />
                <Popup
                  content="Subscribe this artist to Release Radar using trusted scope."
                  position="top center"
                  trigger={
                    <Button
                      aria-label={`Subscribe ${title} taste recommendation to Release Radar`}
                      onClick={() => subscribeRadar(recommendation)}
                      size="mini"
                      type="button"
                    >
                      <Icon name="rss" />
                      Radar
                    </Button>
                  }
                />
                <Popup
                  content="Preview nearby Discovery Graph evidence for this WorkRef."
                  position="top center"
                  trigger={
                    <Button
                      aria-label={`Preview ${title} taste recommendation graph`}
                      onClick={() => previewGraph(recommendation)}
                      size="mini"
                      type="button"
                    >
                      <Icon name="share alternate" />
                      Graph
                    </Button>
                  }
                />
              </List.Content>
            </List.Item>
          );
        })}
      </List>
    </Segment>
  );
};

export default FederatedTasteRecommendationsPanel;
