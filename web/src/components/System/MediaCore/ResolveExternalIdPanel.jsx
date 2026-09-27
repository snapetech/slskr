import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, {
  forwardRef,
  memo,
  useCallback,
  useImperativeHandle,
  useState,
} from 'react';
import {
  Button,
  Card,
  Form,
  Grid,
  Icon,
  Input,
  Message,
} from 'semantic-ui-react';

const useMountedState = (mountedRef, initialValue) => {
  const [value, setValue] = useState(initialValue);
  const setMountedValue = useCallback(
    (nextValue) => {
      if (mountedRef.current) {
        setValue(nextValue);
      }
    },
    [mountedRef],
  );

  return [value, setMountedValue];
};

const ResolveExternalIdPanel = forwardRef((_props, ref) => {
  const mountedRef = useMountedRef();
  const [resolveId, setResolveId] = useMountedState(mountedRef, '');
  const [resolvedContent, setResolvedContent] = useMountedState(
    mountedRef,
    null,
  );
  const [resolving, setResolving] = useMountedState(mountedRef, false);

  useImperativeHandle(
    ref,
    () => ({
      setQuery: setResolveId,
    }),
    [setResolveId],
  );

  const handleResolve = async () => {
    if (!resolveId.trim()) return;

    try {
      setResolving(true);
      setResolvedContent(null);
      const result = await mediacore.resolveContentId(resolveId.trim());
      setResolvedContent(result);
    } catch (error_) {
      setResolvedContent({ error: toDisplayError(error_) });
    } finally {
      setResolving(false);
    }
  };

  return (
    <Grid.Column width={8}>
      <Card fluid>
        <Card.Content>
          <Card.Header>
            <Icon name="search" />
            Resolve External ID
          </Card.Header>
          <Card.Description>
            Find the ContentID for an external identifier
          </Card.Description>
        </Card.Content>
        <Card.Content>
          <Form>
            <Form.Field>
              <label>External ID to Resolve</label>
              <Input
                action={
                  <Button
                    disabled={!resolveId.trim() || resolving}
                    loading={resolving}
                    onClick={handleResolve}
                    primary
                  >
                    Resolve
                  </Button>
                }
                onChange={(e) => setResolveId(e.target.value)}
                placeholder="Enter external ID to resolve..."
                value={resolveId}
              />
            </Form.Field>
          </Form>

          {resolvedContent && (
            <div style={{ marginTop: '1em' }}>
              {resolvedContent.error ? (
                <Message error>
                  <p>{resolvedContent.error}</p>
                </Message>
              ) : (
                <Message success>
                  <Message.Header>Resolved Successfully</Message.Header>
                  <p>
                    <strong>External ID:</strong> {resolvedContent.externalId}
                    <br />
                    <strong>Content ID:</strong> {resolvedContent.contentId}
                  </p>
                </Message>
              )}
            </div>
          )}
        </Card.Content>
      </Card>
    </Grid.Column>
  );
});

export default memo(ResolveExternalIdPanel);
