import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import { Card, Form, Grid, Header, Icon, Input, List, Message } from 'semantic-ui-react';

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

const MediaCoreContentGraphPanel = () => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [traversalResults, setTraversalResults] = useMountedState(mountedRef, null);
  const [graphResults, setGraphResults] = useMountedState(mountedRef, null);
  const [inboundResults, setInboundResults] = useMountedState(mountedRef, null);
  const [traverseContentId, setTraverseContentId] = useMountedState(mountedRef, '');
  const [traverseLinkName, setTraverseLinkName] = useMountedState(mountedRef, '');
  const [graphContentId, setGraphContentId] = useMountedState(mountedRef, '');
  const [inboundTargetId, setInboundTargetId] = useMountedState(mountedRef, '');
  const [registering, setRegistering] = useMountedState(mountedRef, false);
  const [validating, setValidating] = useMountedState(mountedRef, false);
  const [searchingDomain, setSearchingDomain] = useMountedState(mountedRef, false);
  const [traversing, setTraversing] = useMountedState(mountedRef, false);
  const [gettingGraph, setGettingGraph] = useMountedState(mountedRef, false);
  const [findingInbound, setFindingInbound] = useMountedState(mountedRef, false);

  const handleTraverse = async () => {
    if (!traverseContentId.trim() || !traverseLinkName.trim()) return;

    try {
      setTraversing(true);
      setTraversalResults(null);
      const result = await mediacore.traverseContentGraph(
        traverseContentId.trim(),
        traverseLinkName.trim(),
      );
      setTraversalResults(result);
    } catch (error_) {
      setTraversalResults({ error: toDisplayError(error_) });
    } finally {
      setTraversing(false);
    }
  };

  const handleGetGraph = async () => {
    if (!graphContentId.trim()) return;

    try {
      setGettingGraph(true);
      setGraphResults(null);
      const result = await mediacore.getContentGraph(graphContentId.trim());
      setGraphResults(result);
    } catch (error_) {
      setGraphResults({ error: toDisplayError(error_) });
    } finally {
      setGettingGraph(false);
    }
  };

  const handleFindInbound = async () => {
    if (!inboundTargetId.trim()) return;

    try {
      setFindingInbound(true);
      setInboundResults(null);
      const result = await mediacore.findInboundLinks(inboundTargetId.trim());
      setInboundResults(result);
    } catch (error_) {
      setInboundResults({ error: toDisplayError(error_) });
    } finally {
      setFindingInbound(false);
    }
  };

  return (
    <>
        {/* IPLD Graph Traversal */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="sitemap" />
                IPLD Graph Traversal
              </Card.Header>
              <Card.Description>
                Traverse content relationships following specific link types
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Group widths="equal">
                  <Form.Field>
                    <label>Start ContentID</label>
                    <Input
                      onChange={(e) => setTraverseContentId(e.target.value)}
                      placeholder="e.g., content:audio:track:mb-12345"
                      value={traverseContentId}
                    />
                  </Form.Field>
                  <Form.Field>
                    <label>Link Type</label>
                    <Input
                      onChange={(e) => setTraverseLinkName(e.target.value)}
                      placeholder="e.g., album, artist, artwork"
                      value={traverseLinkName}
                    />
                  </Form.Field>
                </Form.Group>
                <Button
                  disabled={
                    !traverseContentId.trim() ||
                    !traverseLinkName.trim() ||
                    traversing
                  }
                  loading={traversing}
                  onClick={handleTraverse}
                  primary
                >
                  Traverse Graph
                </Button>
              </Form>

              {traversalResults && (
                <div style={{ marginTop: '1em' }}>
                  {traversalResults.error ? (
                    <Message error>
                      <p>{traversalResults.error}</p>
                    </Message>
                  ) : (
                    <div>
                      <p>
                        <strong>Traversal completed:</strong>{' '}
                        {traversalResults.completedTraversal ? 'Yes' : 'No'}
                      </p>
                      <p>
                        <strong>
                          Visited {traversalResults.visitedNodes?.length || 0}{' '}
                          nodes
                        </strong>
                      </p>
                      {traversalResults.visitedNodes?.length > 0 && (
                        <List
                          divided
                          relaxed
                          style={{ maxHeight: '150px', overflow: 'auto' }}
                        >
                          {traversalResults.visitedNodes.map((node, index) => (
                            <List.Item key={index}>
                              <List.Content>
                                <List.Header>{node.contentId}</List.Header>
                                <List.Description>
                                  {node.outgoingLinks?.length || 0} outgoing
                                  links
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

        {/* Content Graph */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="share alternate" />
                Content Graph
              </Card.Header>
              <Card.Description>
                Get the complete relationship graph for a ContentID
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>ContentID</label>
                  <Input
                    action={
                      <Button
                        disabled={!graphContentId.trim() || gettingGraph}
                        loading={gettingGraph}
                        onClick={handleGetGraph}
                        primary
                      >
                        Get Graph
                      </Button>
                    }
                    onChange={(e) => setGraphContentId(e.target.value)}
                    placeholder="Enter ContentID to get its graph"
                    value={graphContentId}
                  />
                </Form.Field>
              </Form>

              {graphResults && (
                <div style={{ marginTop: '1em' }}>
                  {graphResults.error ? (
                    <Message error>
                      <p>{graphResults.error}</p>
                    </Message>
                  ) : (
                    <div>
                      <p>
                        <strong>Root:</strong> {graphResults.rootContentId}
                      </p>
                      <p>
                        <strong>Nodes:</strong>{' '}
                        {graphResults.nodes?.length || 0}
                      </p>
                      <p>
                        <strong>Paths:</strong>{' '}
                        {graphResults.paths?.length || 0}
                      </p>
                      {graphResults.nodes?.length > 0 && (
                        <List
                          divided
                          relaxed
                          style={{ maxHeight: '150px', overflow: 'auto' }}
                        >
                          {graphResults.nodes.slice(0, 5).map((node, index) => (
                            <List.Item key={index}>
                              <List.Content>
                                <List.Header style={{ fontSize: '0.9em' }}>
                                  {node.contentId}
                                </List.Header>
                                <List.Description style={{ fontSize: '0.8em' }}>
                                  {node.outgoingLinks?.length || 0} outgoing,{' '}
                                  {node.incomingLinks?.length || 0} incoming
                                </List.Description>
                              </List.Content>
                            </List.Item>
                          ))}
                          {graphResults.nodes.length > 5 && (
                            <List.Item>
                              <List.Content>
                                <em>
                                  ... and {graphResults.nodes.length - 5} more
                                  nodes
                                </em>
                              </List.Content>
                            </List.Item>
                          )}
                        </List>
                      )}
                    </div>
                  )}
                </div>
              )}
            </Card.Content>
          </Card>
        </Grid.Column>

        {/* Inbound Links */}
        <Grid.Column width={8}>
          <Card fluid>
            <Card.Content>
              <Card.Header>
                <Icon name="arrow left" />
                Inbound Links
              </Card.Header>
              <Card.Description>
                Find all content that links to a specific ContentID
              </Card.Description>
            </Card.Content>
            <Card.Content>
              <Form>
                <Form.Field>
                  <label>Target ContentID</label>
                  <Input
                    action={
                      <Button
                        disabled={!inboundTargetId.trim() || findingInbound}
                        loading={findingInbound}
                        onClick={handleFindInbound}
                        primary
                      >
                        Find Links
                      </Button>
                    }
                    onChange={(e) => setInboundTargetId(e.target.value)}
                    placeholder="Find content that links to this ID"
                    value={inboundTargetId}
                  />
                </Form.Field>
              </Form>

              {inboundResults && (
                <div style={{ marginTop: '1em' }}>
                  {inboundResults.error ? (
                    <Message error>
                      <p>{inboundResults.error}</p>
                    </Message>
                  ) : (
                    <div>
                      <p>
                        <strong>
                          Found {inboundResults.inboundLinks?.length || 0}{' '}
                          inbound links
                        </strong>
                      </p>
                      {inboundResults.inboundLinks?.length > 0 && (
                        <List
                          divided
                          relaxed
                          style={{ maxHeight: '150px', overflow: 'auto' }}
                        >
                          {inboundResults.inboundLinks.map((link, index) => (
                            <List.Item key={index}>
                              <List.Content>
                                <code style={{ fontSize: '0.9em' }}>
                                  {link}
                                </code>
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
};

export default React.memo(MediaCoreContentGraphPanel);
