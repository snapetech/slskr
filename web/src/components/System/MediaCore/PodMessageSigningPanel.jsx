import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import PodWorkflowNotice from './PodWorkflowNotice';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import { Card, Form, Grid, Header, Icon, Message } from 'semantic-ui-react';

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

const PodMessageSigningPanel = ({
  messageToVerify,
  setMessageToVerify,
  setVerificationResult,
  verificationResult,
  visible = true,
}) => {
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [messageToSign, setMessageToSign] = useMountedState(mountedRef, '');
  const [privateKeyForSigning, setPrivateKeyForSigning] = useMountedState(mountedRef, '');
  const [signingMessage, setSigningMessage] = useMountedState(mountedRef, false);
  const [signedMessageResult, setSignedMessageResult] = useMountedState(mountedRef, null);
  const [verifyingSignature, setVerifyingSignature] = useMountedState(mountedRef, false);
  const [generatingKeyPair, setGeneratingKeyPair] = useMountedState(mountedRef, false);
  const [generatedKeyPair, setGeneratedKeyPair] = useMountedState(mountedRef, null);
  const [signingStats, setSigningStats] = useMountedState(mountedRef, null);
  const [loadingSigningStats, setLoadingSigningStats] = useMountedState(mountedRef, false);

  const handleSignMessage = async () => {
    if (!messageToSign.trim() || !privateKeyForSigning.trim()) {
      toast.warning('Please enter message JSON and private key');
      return;
    }

    try {
      setSigningMessage(true);
      setSignedMessageResult(null);
      const message = JSON.parse(messageToSign);
      const result = await mediacore.signPodMessage(
        message,
        privateKeyForSigning,
      );
      setSignedMessageResult(result);
      setMessageToSign('');
    } catch (error_) {
      setSignedMessageResult({ error: toDisplayError(error_) });
    } finally {
      setSigningMessage(false);
    }
  };

  const handleVerifySignature = async () => {
    if (!messageToVerify.trim()) {
      toast.warning('Please enter message JSON to verify');
      return;
    }

    try {
      setVerifyingSignature(true);
      setVerificationResult(null);
      const message = JSON.parse(messageToVerify);
      const result = await mediacore.verifyPodMessageSignature(message);
      setVerificationResult(result);
    } catch (error_) {
      setVerificationResult({ error: toDisplayError(error_) });
    } finally {
      setVerifyingSignature(false);
    }
  };

  const handleGenerateKeyPair = async () => {
    try {
      setGeneratingKeyPair(true);
      setGeneratedKeyPair(null);
      const result = await mediacore.generateMessageKeyPair();
      setGeneratedKeyPair(result);
    } catch (error_) {
      setGeneratedKeyPair({ error: toDisplayError(error_) });
    } finally {
      setGeneratingKeyPair(false);
    }
  };

  const handleLoadSigningStats = async () => {
    try {
      setLoadingSigningStats(true);
      setSigningStats(null);
      const result = await mediacore.getMessageSigningStats();
      setSigningStats(result);
    } catch (error_) {
      setSigningStats({ error: toDisplayError(error_) });
    } finally {
      setLoadingSigningStats(false);
    }
  };

  return (
        <Grid.Column style={{ display: visible ? undefined : 'none' }} width={16}>
          <Card id="pod-message-signing" fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="key" />
                Pod Message Signing
              </Card.Header>
              <Card.Description>
                Cryptographic signing and verification of pod messages for
                authenticity and integrity
              </Card.Description>
              <PodWorkflowNotice title="Handles key material">
                Signing and key generation workflows may expose private keys or
                signed payloads in the browser. Treat pasted keys and generated
                output as sensitive.
              </PodWorkflowNotice>
            </Card.Content>

            {/* Message Signing */}
            <Card.Content>
              <Header size="small">Sign Pod Message</Header>
              <Form>
                <Form.TextArea
                  label="Pod Message JSON"
                  onChange={(e) => setMessageToSign(e.target.value)}
                  placeholder='{"messageId": "msg123", "channelId": "pod:artist:mb:daft-punk-hash:general", "senderPeerId": "alice", "body": "Hello pod!", "timestampUnixMs": 1703123456789}'
                  rows={3}
                  value={messageToSign}
                />
                <Form.Input
                  label="Private Key"
                  onChange={(e) => setPrivateKeyForSigning(e.target.value)}
                  placeholder="base64-encoded private key"
                  type="password"
                  value={privateKeyForSigning}
                />
                <Button
                  disabled={
                    signingMessage ||
                    !messageToSign.trim() ||
                    !privateKeyForSigning.trim()
                  }
                  loading={signingMessage}
                  onClick={handleSignMessage}
                  primary
                >
                  Sign Message
                </Button>
              </Form>

              {signedMessageResult && (
                <div style={{ marginTop: '1em' }}>
                  {signedMessageResult.error ? (
                    <Message error>
                      <p>Failed to sign message: {signedMessageResult.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>
                        Message Signed Successfully
                      </Message.Header>
                      <p>
                        <strong>Message ID:</strong>{' '}
                        {signedMessageResult.messageId}
                        <br />
                        <strong>Channel:</strong>{' '}
                        {signedMessageResult.channelId}
                        <br />
                        <strong>Signature:</strong>{' '}
                        {signedMessageResult.signature?.slice(0, 50)}...
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>

            <Card.Content>
              <Grid>
                <Grid.Column width={8}>
                  {/* Signature Verification */}
                  <Header size="small">Verify Message Signature</Header>
                  <Form>
                    <Form.TextArea
                      label="Pod Message JSON (with signature)"
                      onChange={(e) => setMessageToVerify(e.target.value)}
                      placeholder='{"messageId": "msg123", "channelId": "pod:artist:mb:daft-punk-hash:general", "senderPeerId": "alice", "body": "Hello pod!", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature"}'
                      rows={4}
                      value={messageToVerify}
                    />
                    <Button
                      disabled={verifyingSignature || !messageToVerify.trim()}
                      fluid
                      loading={verifyingSignature}
                      onClick={handleVerifySignature}
                    >
                      Verify Signature
                    </Button>
                  </Form>

                  {verificationResult && (
                    <div style={{ marginTop: '0.5em' }}>
                      {verificationResult.error ? (
                        <Message
                          error
                          size="tiny"
                        >
                          <p>{verificationResult.error}</p>
                        </Message>
                      ) : (
                        <Message size="tiny">
                          <p>
                            Message {verificationResult.messageId}: Signature is{' '}
                            {verificationResult.isValid ? 'VALID' : 'INVALID'}
                          </p>
                        </Message>
                      )}
                    </div>
                  )}
                </Grid.Column>

                <Grid.Column width={8}>
                  {/* Key Pair Generation */}
                  <Header size="small">Generate Key Pair</Header>
                  <Form>
                    <Button
                      disabled={generatingKeyPair}
                      fluid
                      loading={generatingKeyPair}
                      onClick={handleGenerateKeyPair}
                    >
                      Generate New Key Pair
                    </Button>
                  </Form>

                  {generatedKeyPair && (
                    <div style={{ marginTop: '0.5em' }}>
                      {generatedKeyPair.error ? (
                        <Message
                          error
                          size="tiny"
                        >
                          <p>{generatedKeyPair.error}</p>
                        </Message>
                      ) : (
                        <Message
                          size="tiny"
                          success
                        >
                          <Message.Header>Key Pair Generated</Message.Header>
                          <p>
                            <strong>Public Key:</strong>{' '}
                            {generatedKeyPair.publicKey?.slice(0, 30)}...
                            <br />
                            <strong>Private Key:</strong>{' '}
                            {generatedKeyPair.privateKey?.slice(0, 30)}...
                            <br />
                            <em>⚠️ Keep private key secure!</em>
                          </p>
                        </Message>
                      )}
                    </div>
                  )}

                  {/* Signing Statistics */}
                  <Header
                    size="small"
                    style={{ marginTop: '1em' }}
                  >
                    Signing Statistics
                  </Header>
                  <Button.Group fluid>
                    <Button
                      disabled={loadingSigningStats}
                      loading={loadingSigningStats}
                      onClick={handleLoadSigningStats}
                    >
                      Load Stats
                    </Button>
                  </Button.Group>

                  {signingStats && !signingStats.error && (
                    <div style={{ marginTop: '0.5em' }}>
                      <Message size="tiny">
                        <p>
                          <strong>Signatures Created:</strong>{' '}
                          {signingStats.totalSignaturesCreated}
                          <br />
                          <strong>Signatures Verified:</strong>{' '}
                          {signingStats.totalSignaturesVerified}
                          <br />
                          <strong>Successful:</strong>{' '}
                          {signingStats.successfulVerifications}
                          <br />
                          <strong>Failed:</strong>{' '}
                          {signingStats.failedVerifications}
                          <br />
                          <strong>Avg Sign Time:</strong>{' '}
                          {signingStats.averageSigningTimeMs.toFixed(2)}ms
                          <br />
                          <strong>Avg Verify Time:</strong>{' '}
                          {signingStats.averageVerificationTimeMs.toFixed(2)}ms
                        </p>
                      </Message>
                    </div>
                  )}

                  {signingStats?.error && (
                    <Message
                      error
                      size="tiny"
                      style={{ marginTop: '0.5em' }}
                    >
                      <p>{signingStats.error}</p>
                    </Message>
                  )}
                </Grid.Column>
              </Grid>
            </Card.Content>
          </Card>
        </Grid.Column>

    );
};

export default React.memo(PodMessageSigningPanel);
