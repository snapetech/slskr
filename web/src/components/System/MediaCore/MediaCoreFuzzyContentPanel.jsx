import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import { Card, Form, Grid, Icon, Input, List, Message } from 'semantic-ui-react';

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

const MediaCoreFuzzyContentPanel = React.memo(() => {
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [findSimilarContentId, setFindSimilarContentId] = useMountedState(mountedRef, '');
  const [findSimilarMinConfidence, setFindSimilarMinConfidence] = useMountedState(mountedRef, 0.7);
  const [findSimilarMaxResults, setFindSimilarMaxResults] = useMountedState(mountedRef, 10);
  const [findSimilarResult, setFindSimilarResult] = useMountedState(mountedRef, null);
  const [findingSimilarContent, setFindingSimilarContent] = useMountedState(mountedRef, false);

  const handleFindSimilarContent = async () => {
    if (!findSimilarContentId.trim()) return;

    try {
      setFindingSimilarContent(true);
      setFindSimilarResult(null);
      const result = await mediacore.findSimilarContent(
        findSimilarContentId.trim(),
        {
          maxResults: Number.parseInt(findSimilarMaxResults),
          minConfidence: Number.parseFloat(findSimilarMinConfidence),
        },
      );
      setFindSimilarResult(result);
    } catch (error_) {
      setFindSimilarResult({ error: toDisplayError(error_) });
    } finally {
      setFindingSimilarContent(false);
    }
  };

  return (
    <>
        {/* Fuzzy Content Matching */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="magic" />
                Fuzzy Content Matching
              </Card.Header>
              <Card.Description>
                Find similar content using perceptual hashes and text analysis
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>Target ContentID</label>
                  <Input
                    onChange={(e) => setFindSimilarContentId(e.target.value)}
                    placeholder="ContentID to find matches for"
                    value={findSimilarContentId}
                  />
                </Form.Field>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Min Confidence</label>
                    <Input
                      max="1"
                      min="0"
                      onChange={(e) =>
                        setFindSimilarMinConfidence(e.target.value)
                      }
                      step="0.1"
                      type="number"
                      value={findSimilarMinConfidence}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Max Results</label>
                    <Input
                      max="50"
                      min="1"
                      onChange={(e) => setFindSimilarMaxResults(e.target.value)}
                      type="number"
                      value={findSimilarMaxResults}
                    />
                  </Form.Field>
                </Form.Group>
                <Button
                  disabled={
                    !findSimilarContentId.trim() || findingSimilarContent
                  }
                  loading={findingSimilarContent}
                  onClick={handleFindSimilarContent}
                  primary
                >
                  Find Similar Content
                </Button>
              </Form>

              {findSimilarResult && (
                <div style={{ marginTop: '1em' }}>
                  {findSimilarResult.error ? (
                    <Message error>
                      <p>{findSimilarResult.error}</p>
                    </Message>
                  ) : (
                    <div>
                      <p>
                        <strong>Target:</strong>{' '}
                        {findSimilarResult.targetContentId}
                      </p>
                      <p>
                        <strong>
                          Searched {findSimilarResult.totalCandidates}{' '}
                          candidates
                        </strong>
                      </p>
                      <p>
                        <strong>
                          Found {findSimilarResult.matches?.length || 0} matches
                        </strong>
                      </p>
                      {findSimilarResult.matches?.length > 0 && (
                        <List
                          divided
                          relaxed
                          style={{ maxHeight: '200px', overflow: 'auto' }}
                        >
                          {findSimilarResult.matches.map((match, index) => (
                            <List.Item key={index}>
                              <List.Content>
                                <List.Header style={{ fontSize: '0.9em' }}>
                                  {match.candidateContentId}
                                </List.Header>
                                <List.Description>
                                  Confidence:{' '}
                                  {(match.confidence * 100).toFixed(1)}% |
                                  Reason: {match.reason}
                                </List.Description>
                              </List.Content>
                            </List.Item>
                          ))}
                        </List>
                      )}
                    </div>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

    </>
  );
});

export default MediaCoreFuzzyContentPanel;
