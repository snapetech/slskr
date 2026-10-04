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

const MediaCoreTextSimilarityPanel = React.memo(() => {
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [textSimilarityA, setTextSimilarityA] = useMountedState(mountedRef, '');
  const [textSimilarityB, setTextSimilarityB] = useMountedState(mountedRef, '');
  const [textSimilarityResult, setTextSimilarityResult] = useMountedState(mountedRef, null);
  const [computingTextSimilarity, setComputingTextSimilarity] = useMountedState(mountedRef, false);

  const handleComputeTextSimilarity = async () => {
    if (!textSimilarityA.trim() || !textSimilarityB.trim()) return;

    try {
      setComputingTextSimilarity(true);
      setTextSimilarityResult(null);
      const result = await mediacore.computeTextSimilarity(
        textSimilarityA.trim(),
        textSimilarityB.trim(),
      );
      setTextSimilarityResult(result);
    } catch (error_) {
      setTextSimilarityResult({ error: toDisplayError(error_) });
    } finally {
      setComputingTextSimilarity(false);
    }
  };

  return (
    <>
        {/* Text Similarity */}
        <Grid.Column width={16}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="font" />
                Text Similarity Analysis
              </Card.Header>
              <Card.Description>
                Compare text strings using Levenshtein distance and phonetic
                matching
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Text A</label>
                    <Input
                      onChange={(e) => setTextSimilarityA(e.target.value)}
                      placeholder="First text string"
                      value={textSimilarityA}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Text B</label>
                    <Input
                      onChange={(e) => setTextSimilarityB(e.target.value)}
                      placeholder="Second text string"
                      value={textSimilarityB}
                    />
                  </Form.Field>
                </Form.Group>
                <Button
                  disabled={
                    !textSimilarityA.trim() ||
                    !textSimilarityB.trim() ||
                    computingTextSimilarity
                  }
                  loading={computingTextSimilarity}
                  onClick={handleComputeTextSimilarity}
                  primary
                >
                  Analyze Text Similarity
                </Button>
              </Form>

              {textSimilarityResult && (
                <div style={{ marginTop: '1em' }}>
                  {textSimilarityResult.error ? (
                    <Message error>
                      <p>{textSimilarityResult.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>Text Similarity Results</Message.Header>
                      <p>
                        <strong>Text A:</strong> "{textSimilarityResult.textA}"
                        <br />
                        <strong>Text B:</strong> "{textSimilarityResult.textB}"
                        <br />
                        <strong>Levenshtein Similarity:</strong>{' '}
                        {(
                          textSimilarityResult.levenshteinSimilarity * 100
                        ).toFixed(1)}
                        %<br />
                        <strong>Phonetic Similarity:</strong>{' '}
                        {(
                          textSimilarityResult.phoneticSimilarity * 100
                        ).toFixed(1)}
                        %<br />
                        <strong>Combined Similarity:</strong>{' '}
                        {(
                          textSimilarityResult.combinedSimilarity * 100
                        ).toFixed(1)}
                        %
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

export default MediaCoreTextSimilarityPanel;
