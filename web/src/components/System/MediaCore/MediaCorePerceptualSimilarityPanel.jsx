import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import { Card, Form, Grid, Icon, Input, Message } from 'semantic-ui-react';

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

const MediaCorePerceptualSimilarityPanel = React.memo(() => {
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [perceptualContentIdA, setPerceptualContentIdA] = useMountedState(mountedRef, '');
  const [perceptualContentIdB, setPerceptualContentIdB] = useMountedState(mountedRef, '');
  const [perceptualThreshold, setPerceptualThreshold] = useMountedState(mountedRef, 0.7);
  const [perceptualSimilarityResult, setPerceptualSimilarityResult] =
    useMountedState(mountedRef, null);
  const [computingPerceptualSimilarity, setComputingPerceptualSimilarity] =
    useMountedState(mountedRef, false);

  const handleComputePerceptualSimilarity = async () => {
    if (!perceptualContentIdA.trim() || !perceptualContentIdB.trim()) return;

    try {
      setComputingPerceptualSimilarity(true);
      setPerceptualSimilarityResult(null);
      const result = await mediacore.computePerceptualSimilarity(
        perceptualContentIdA.trim(),
        perceptualContentIdB.trim(),
        Number.parseFloat(perceptualThreshold),
      );
      setPerceptualSimilarityResult(result);
    } catch (error_) {
      setPerceptualSimilarityResult({ error: toDisplayError(error_) });
    } finally {
      setComputingPerceptualSimilarity(false);
    }
  };

  return (
    <>
        {/* Perceptual Similarity */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="chart bar" />
                Perceptual Similarity
              </Card.Header>
              <Card.Description>
                Compare perceptual similarity between two ContentIDs
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>ContentID A</label>
                    <Input
                      onChange={(e) => setPerceptualContentIdA(e.target.value)}
                      placeholder="First ContentID"
                      value={perceptualContentIdA}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>ContentID B</label>
                    <Input
                      onChange={(e) => setPerceptualContentIdB(e.target.value)}
                      placeholder="Second ContentID"
                      value={perceptualContentIdB}
                    />
                  </Form.Field>
                </Form.Group>
                <Form.Field>
                  <label>Similarity Threshold</label>
                  <Input
                    max="1"
                    min="0"
                    onChange={(e) => setPerceptualThreshold(e.target.value)}
                    step="0.1"
                    type="number"
                    value={perceptualThreshold}
                  />
                </Form.Field>
                <Button
                  disabled={
                    !perceptualContentIdA.trim() ||
                    !perceptualContentIdB.trim() ||
                    computingPerceptualSimilarity
                  }
                  loading={computingPerceptualSimilarity}
                  onClick={handleComputePerceptualSimilarity}
                  primary
                >
                  Compute Similarity
                </Button>
              </Form>

              {perceptualSimilarityResult && (
                <div style={{ marginTop: '1em' }}>
                  {perceptualSimilarityResult.error ? (
                    <Message error>
                      <p>{perceptualSimilarityResult.error}</p>
                    </Message>
                  ) : (
                    <Message info>
                      <Message.Header>Similarity Analysis</Message.Header>
                      <p>
                        <strong>Content A:</strong>{' '}
                        {perceptualSimilarityResult.contentIdA}
                        <br />
                        <strong>Content B:</strong>{' '}
                        {perceptualSimilarityResult.contentIdB}
                        <br />
                        <strong>Similarity:</strong>{' '}
                        {(perceptualSimilarityResult.similarity * 100).toFixed(
                          1,
                        )}
                        %<br />
                        <strong>Are Similar:</strong>{' '}
                        {perceptualSimilarityResult.isSimilar ? 'Yes' : 'No'}{' '}
                        (threshold:{' '}
                        {(perceptualSimilarityResult.threshold * 100).toFixed(
                          1,
                        )}
                        %)
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
});

export default MediaCorePerceptualSimilarityPanel;
