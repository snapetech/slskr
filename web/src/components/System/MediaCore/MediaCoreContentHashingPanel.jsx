import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import {
  Card,
  Dropdown,
  Form,
  Grid,
  Icon,
  Input,
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

const MediaCoreContentHashingPanel = React.memo(({ supportedAlgorithms }) => {
  const mountedRef = useRef(false);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [audioSamples, setAudioSamples] = useMountedState(mountedRef, '');
  const [sampleRate, setSampleRate] = useMountedState(mountedRef, 44_100);
  const [audioAlgorithm, setAudioAlgorithm] = useMountedState(mountedRef, 'ChromaPrint');
  const [imagePixels, setImagePixels] = useMountedState(mountedRef, '');
  const [imageWidth, setImageWidth] = useMountedState(mountedRef, 100);
  const [imageHeight, setImageHeight] = useMountedState(mountedRef, 100);
  const [imageAlgorithm, setImageAlgorithm] = useMountedState(mountedRef, 'PHash');
  const [audioHashResult, setAudioHashResult] = useMountedState(mountedRef, null);
  const [imageHashResult, setImageHashResult] = useMountedState(mountedRef, null);
  const [computingAudioHash, setComputingAudioHash] = useMountedState(mountedRef, false);
  const [computingImageHash, setComputingImageHash] = useMountedState(mountedRef, false);

  const handleComputeAudioHash = async () => {
    if (!audioSamples.trim()) return;

    try {
      setComputingAudioHash(true);
      setAudioHashResult(null);

      // Parse comma-separated float values
      const samples = audioSamples
        .split(',')
        .map((s) => Number.parseFloat(s.trim()))
        .filter((n) => !isNaN(n));

      if (samples.length === 0) {
        throw new Error('No valid audio samples provided');
      }

      const result = await mediacore.computeAudioHash(
        samples,
        Number.parseInt(sampleRate),
        audioAlgorithm,
      );
      setAudioHashResult(result);
    } catch (error_) {
      setAudioHashResult({ error: toDisplayError(error_) });
    } finally {
      setComputingAudioHash(false);
    }
  };

  const handleComputeImageHash = async () => {
    if (!imagePixels.trim()) return;

    try {
      setComputingImageHash(true);
      setImageHashResult(null);

      // Parse comma-separated byte values (0-255)
      const pixels = imagePixels
        .split(',')
        .map((s) => Number.parseInt(s.trim()))
        .filter((n) => !isNaN(n) && n >= 0 && n <= 255);

      if (pixels.length === 0) {
        throw new Error('No valid pixel data provided');
      }

      const result = await mediacore.computeImageHash(
        pixels,
        Number.parseInt(imageWidth),
        Number.parseInt(imageHeight),
        imageAlgorithm,
      );
      setImageHashResult(result);
    } catch (error_) {
      setImageHashResult({ error: toDisplayError(error_) });
    } finally {
      setComputingImageHash(false);
    }
  };

  return (
    <>
        {/* Perceptual Hash - Audio */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="sound" />
                Audio Perceptual Hash
              </Card.Header>
              <Card.Description>
                Compute perceptual hash for audio similarity detection
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Algorithm</label>
                    <Dropdown
                      onChange={(e, { value }) => setAudioAlgorithm(value)}
                      options={
                        supportedAlgorithms?.algorithms?.map((alg) => ({
                          key: alg,
                          text: alg,
                          value: alg,
                        })) || []
                      }
                      selection
                      value={audioAlgorithm}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Sample Rate (Hz)</label>
                    <Input
                      onChange={(e) => setSampleRate(e.target.value)}
                      type="number"
                      value={sampleRate}
                    />
                  </Form.Field>
                </Form.Group>
                <Form.Field>
                  <label>Audio Samples (comma-separated floats)</label>
                  <TextArea
                    onChange={(e) => setAudioSamples(e.target.value)}
                    placeholder="0.1, -0.2, 0.3, ... (normalized -1.0 to 1.0)"
                    rows={3}
                    value={audioSamples}
                  />
                </Form.Field>
                <Button
                  disabled={!audioSamples.trim() || computingAudioHash}
                  loading={computingAudioHash}
                  onClick={handleComputeAudioHash}
                  primary
                >
                  Compute Audio Hash
                </Button>
              </Form>

              {audioHashResult && (
                <div style={{ marginTop: '1em' }}>
                  {audioHashResult.error ? (
                    <Message error>
                      <p>{audioHashResult.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>Audio Hash Computed</Message.Header>
                      <p>
                        <strong>Algorithm:</strong> {audioHashResult.algorithm}
                        <br />
                        <strong>Hex Hash:</strong> {audioHashResult.hex}
                        <br />
                        <strong>Sample Count:</strong>{' '}
                        {audioSamples.split(',').filter((s) => s.trim()).length}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Perceptual Hash - Image */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="image" />
                Image Perceptual Hash
              </Card.Header>
              <Card.Description>
                Compute perceptual hash for image similarity detection
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Algorithm</label>
                    <Dropdown
                      onChange={(e, { value }) => setImageAlgorithm(value)}
                      options={
                        supportedAlgorithms?.algorithms
                          ?.filter((alg) => alg !== 'ChromaPrint')
                          .map((alg) => ({
                            key: alg,
                            text: alg,
                            value: alg,
                          })) || []
                      }
                      selection
                      value={imageAlgorithm}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Dimensions</label>
                    <Input
                      onChange={(e) => {
                        const [w, h] = e.target.value
                          .split('x')
                          .map((s) => Number.parseInt(s.trim()));
                        if (!isNaN(w)) setImageWidth(w);
                        if (!isNaN(h)) setImageHeight(h);
                      }}
                      placeholder="Width x Height"
                      value={`${imageWidth}x${imageHeight}`}
                    />
                  </Form.Field>
                </Form.Group>
                <Form.Field>
                  <label>Pixel Data (comma-separated bytes 0-255)</label>
                  <TextArea
                    onChange={(e) => setImagePixels(e.target.value)}
                    placeholder="255, 128, 64, ... (RGBA pixel data)"
                    rows={3}
                    value={imagePixels}
                  />
                </Form.Field>
                <Button
                  disabled={!imagePixels.trim() || computingImageHash}
                  loading={computingImageHash}
                  onClick={handleComputeImageHash}
                  primary
                >
                  Compute Image Hash
                </Button>
              </Form>

              {imageHashResult && (
                <div style={{ marginTop: '1em' }}>
                  {imageHashResult.error ? (
                    <Message error>
                      <p>{imageHashResult.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>Image Hash Computed</Message.Header>
                      <p>
                        <strong>Algorithm:</strong> {imageHashResult.algorithm}
                        <br />
                        <strong>Hex Hash:</strong> {imageHashResult.hex}
                        <br />
                        <strong>Dimensions:</strong> {imageWidth}x{imageHeight}
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

export default MediaCoreContentHashingPanel;
