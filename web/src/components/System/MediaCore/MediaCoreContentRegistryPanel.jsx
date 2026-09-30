import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import ResolveExternalIdPanel from './ResolveExternalIdPanel';
import {
  contentExamples,
} from './mediaCoreWorkflows';
import {
  Card,
  Form,
  Grid,
  Header,
  Icon,
  Input,
  List,
  Message,
  Segment,
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

const MediaCoreContentRegistryPanel = ({ setContentId, setError, setStats }) => {
  const mountedRef = useRef(false);
  const resolveExternalIdPanelRef = useRef(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [externalId, setExternalId] = useMountedState(mountedRef, '');
  const [descriptorContentId, setDescriptorContentId] = useMountedState(mountedRef, '');
  const [validateContentIdInput, setValidateContentIdInput] = useMountedState(mountedRef, '');
  const [domain, setDomain] = useMountedState(mountedRef, '');
  const [type, setType] = useMountedState(mountedRef, '');
  const [validatedContent, setValidatedContent] = useMountedState(mountedRef, null);
  const [domainResults, setDomainResults] = useMountedState(mountedRef, null);
  const [registering, setRegistering] = useMountedState(mountedRef, false);
  const [validating, setValidating] = useMountedState(mountedRef, false);
  const [searchingDomain, setSearchingDomain] = useMountedState(mountedRef, false);

  const handleRegister = async () => {
    if (!externalId.trim() || !descriptorContentId.trim()) return;

    try {
      setRegistering(true);
      await mediacore.registerContentId(
        externalId.trim(),
        descriptorContentId.trim(),
      );
      setExternalId('');
      setDescriptorContentId('');
      setContentId('');

      // Refresh stats
      const data = await mediacore.getContentIdStats();
      setStats(data);
    } catch (error_) {
      setError(`Failed to register: ${toDisplayError(error_)}`);
    } finally {
      setRegistering(false);
    }
  };

  const handleValidate = async () => {
    if (!validateContentIdInput.trim()) return;

    try {
      setValidating(true);
      setValidatedContent(null);
      const result = await mediacore.validateContentId(
        validateContentIdInput.trim(),
      );
      setValidatedContent(result);
    } catch (error_) {
      setValidatedContent({ error: toDisplayError(error_) });
    } finally {
      setValidating(false);
    }
  };

  const handleDomainSearch = async () => {
    if (!domain.trim()) return;

    try {
      setSearchingDomain(true);
      setDomainResults(null);
      const result = type.trim()
        ? await mediacore.findContentIdsByDomainAndType(
            domain.trim(),
            type.trim(),
          )
        : await mediacore.findContentIdsByDomain(domain.trim());
      setDomainResults(result);
    } catch (error_) {
      setDomainResults({ error: toDisplayError(error_) });
    } finally {
      setSearchingDomain(false);
    }
  };

  const fillExample = (domain, type) => {
    const example = contentExamples[domain]?.[type];
    if (example) {
      setExternalId(example.external);
      resolveExternalIdPanelRef.current?.setQuery(example.external);
      setDescriptorContentId(example.content);
      setValidateContentIdInput(example.content);
    }
  };

  return (
    <>
        {/* Register New Mapping */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="plus" />
                Register ContentID Mapping
              </Card.Header>
              <Card.Description>
                Map an external identifier to an internal ContentID
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>External ID</label>
                  <Input
                    onChange={(e) => setExternalId(e.target.value)}
                    placeholder="e.g., mb:recording:12345-6789-..."
                    value={externalId}
                  />
                </Form.Field>
                <Form.Field>
                  <label>Content ID</label>
                  <Input
                    onChange={(e) => setDescriptorContentId(e.target.value)}
                    placeholder="e.g., content:mb:recording:12345-6789-..."
                    value={descriptorContentId}
                  />
                </Form.Field>
                <Button
                  disabled={
                    !externalId.trim() ||
                    !descriptorContentId.trim() ||
                    registering
                  }
                  loading={registering}
                  onClick={handleRegister}
                  primary
                >
                  Register Mapping
                </Button>
              </Form>
            </Card.Content>
          </Card>
        </Grid.Column>

        <ResolveExternalIdPanel ref={resolveExternalIdPanelRef} />

        {/* ContentID Validation */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="check circle" />
                ContentID Validation
              </Card.Header>
              <Card.Description>
                Validate ContentID format and extract components
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>ContentID to Validate</label>
                  <Input
                    action={
                      <Button
                        disabled={!validateContentIdInput.trim() || validating}
                        loading={validating}
                        onClick={handleValidate}
                        primary
                      >
                        Validate
                      </Button>
                    }
                    onChange={(e) => setValidateContentIdInput(e.target.value)}
                    placeholder="e.g., content:audio:track:mb-12345"
                    value={validateContentIdInput}
                  />
                </Form.Field>
              </Form>

              {validatedContent && (
                <div style={{ marginTop: '1em' }}>
                  {validatedContent.error ? (
                    <Message error>
                      <p>{validatedContent.error}</p>
                    </Message>
                  ) : (
                    <Message success>
                      <Message.Header>Valid ContentID</Message.Header>
                      <p>
                        <strong>Domain:</strong> {validatedContent.domain}
                        <br />
                        <strong>Type:</strong> {validatedContent.type}
                        <br />
                        <strong>ID:</strong> {validatedContent.id}
                        <br />
                        <strong>Audio:</strong>{' '}
                        {validatedContent.isAudio ? 'Yes' : 'No'} |
                        <strong>Video:</strong>{' '}
                        {validatedContent.isVideo ? 'Yes' : 'No'} |
                        <strong>Image:</strong>{' '}
                        {validatedContent.isImage ? 'Yes' : 'No'}
                      </p>
                    </Message>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Domain Search */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="search plus" />
                Domain Search
              </Card.Header>
              <Card.Description>
                Find ContentIDs by domain and optional type
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Domain</label>
                    <Input
                      onChange={(e) => setDomain(e.target.value)}
                      placeholder="e.g., audio, video, image"
                      value={domain}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Type (optional)</label>
                    <Input
                      onChange={(e) => setType(e.target.value)}
                      placeholder="e.g., track, movie, photo"
                      value={type}
                    />
                  </Form.Field>
                </Form.Group>
                <Button
                  disabled={!domain.trim() || searchingDomain}
                  loading={searchingDomain}
                  onClick={handleDomainSearch}
                  primary
                >
                  Search Domain
                </Button>
              </Form>

              {domainResults && (
                <div style={{ marginTop: '1em' }}>
                  {domainResults.error ? (
                    <Message error>
                      <p>{domainResults.error}</p>
                    </Message>
                  ) : (
                    <div>
                      <p>
                        <strong>
                          Found {domainResults.contentIds?.length || 0}{' '}
                          ContentIDs
                        </strong>
                      </p>
                      {domainResults.contentIds?.length > 0 && (
                        <List
                          divided
                          relaxed
                          style={{ maxHeight: '200px', overflow: 'auto' }}
                        >
                          {domainResults.contentIds.map((id, index) => (
                            <List.Item key={index}>
                              <List.Content>
                                <code>{id}</code>
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

        {/* Examples */}
        <Grid.Column width={16}>
          <Segment>
            <Header as="h3">
              <Icon name="lightbulb" />
              ContentID Examples
            </Header>
            <p>
              Click any example to fill the read-only resolve and validation
              fields, plus the advanced registration fields.
            </p>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5em' }}>
              {Object.entries(contentExamples).map(([domainName, types]) =>
                Object.entries(types).map(([typeName, example]) => (
                  <Button
                    key={`${domainName}-${typeName}`}
                    onClick={() => fillExample(domainName, typeName)}
                    size="small"
                  >
                    {domainName}:{typeName}
                  </Button>
                )),
              )}
            </div>
          </Segment>
        </Grid.Column>

    </>
  );
};

export default React.memo(MediaCoreContentRegistryPanel);
