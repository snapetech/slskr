import {
  getYaml,
  getYamlLocation,
  updateYaml,
  validateYaml,
} from '../../../lib/options';
import { toDisplayError } from '../../../lib/errors';
import { Div, PlaceholderSegment, Switch } from '../../Shared';
import CodeEditor from '../../Shared/CodeEditor';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useEffect, useRef, useState } from 'react';
import { Button, Icon, Message, Modal } from 'semantic-ui-react';

const EditModal = ({ onClose, open, theme }) => {
  // eslint-disable-next-line react/hook-use-state
  const [{ error, loading }, setLoading] = useState({
    error: false,
    loading: true,
  });
  // eslint-disable-next-line react/hook-use-state
  const [{ isDirty, location, yaml }, setYaml] = useState({
    isDirty: false,
    location: undefined,
    yaml: undefined,
  });
  const [yamlError, setYamlError] = useState();
  const [updateError, setUpdateError] = useState();
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const mountedRef = useMountedRef();
  const loadRequestIdRef = useRef(0);
  const validationRequestIdRef = useRef(0);
  const saveRequestIdRef = useRef(0);

  const validate = async (newYaml) => {
    const requestId = ++validationRequestIdRef.current;
    if (mountedRef.current) {
      setValidating(true);
      setYamlError(undefined);
    }

    try {
      const response = await validateYaml({ yaml: newYaml });
      const error =
        response === undefined || response === null || response === ''
          ? undefined
          : typeof response === 'string'
            ? response
            : toDisplayError(response, 'YAML validation failed');
      if (
        mountedRef.current &&
        requestId === validationRequestIdRef.current
      ) {
        setYamlError(error);
      }
      return { error, requestId };
    } catch (validationError) {
      const error = toDisplayError(validationError, 'YAML validation failed');
      if (
        mountedRef.current &&
        requestId === validationRequestIdRef.current
      ) {
        setYamlError(error);
      }
      return { error, requestId };
    } finally {
      if (
        mountedRef.current &&
        requestId === validationRequestIdRef.current
      ) {
        setValidating(false);
      }
    }
  };

  const update = (newYaml) => {
    if (!mountedRef.current) return;
    setYaml({ isDirty: true, location, yaml: newYaml });
    setUpdateError(undefined);
    void validate(newYaml);
  };

  const save = async (newYaml) => {
    if (!mountedRef.current || saving || loading || newYaml === undefined) {
      return;
    }

    const requestId = ++saveRequestIdRef.current;
    setSaving(true);
    setUpdateError(undefined);

    try {
      const validation = await validate(newYaml);

      if (
        !mountedRef.current ||
        requestId !== saveRequestIdRef.current ||
        validation.requestId !== validationRequestIdRef.current
      ) {
        return;
      }

      if (!validation.error) {
        await updateYaml({ yaml: newYaml });
        if (
          mountedRef.current &&
          requestId === saveRequestIdRef.current
        ) {
          onClose();
        }
      }
    } catch (nextUpdateError) {
      if (
        mountedRef.current &&
        requestId === saveRequestIdRef.current
      ) {
        setUpdateError(
          toDisplayError(nextUpdateError, 'Failed to update YAML'),
        );
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === saveRequestIdRef.current
      ) {
        setSaving(false);
      }
    }
  };

  useEffect(() => {
    const requestId = ++loadRequestIdRef.current;
    validationRequestIdRef.current += 1;
    if (!open) return undefined;

    setLoading({ error: false, loading: true });
    setYamlError(undefined);
    setUpdateError(undefined);

    const load = async () => {
      try {
        const [locationResult, yamlResult] = await Promise.all([
          getYamlLocation(),
          getYaml(),
        ]);

        if (
          mountedRef.current &&
          requestId === loadRequestIdRef.current
        ) {
          setYaml({
            isDirty: false,
            location: locationResult,
            yaml: yamlResult,
          });
          setLoading({ error: false, loading: false });
        }
      } catch (getError) {
        if (
          mountedRef.current &&
          requestId === loadRequestIdRef.current
        ) {
          setLoading({
            error: toDisplayError(getError, 'Failed to load YAML'),
            loading: false,
          });
        }
      }
    };

    void load();
    return () => {
      loadRequestIdRef.current += 1;
      validationRequestIdRef.current += 1;
      saveRequestIdRef.current += 1;
    };
  }, [mountedRef, open]);

  return (
    <Modal
      onClose={onClose}
      open={open}
      size="large"
    >
      <Modal.Header>
        <Icon name="edit" />
        Edit Options
        <Div hidden={loading}>
          <Message
            className="no-grow edit-code-header"
            warning
          >
            <Icon name="warning sign" />
            Editing {location}
          </Message>
        </Div>
      </Modal.Header>
      <Modal.Content
        className="edit-code-content"
        scrolling
      >
        <Switch
          error={error && <PlaceholderSegment icon="close" />}
          loading={loading && <PlaceholderSegment loading />}
        >
          <div
            {...{
              className:
                yamlError || updateError
                  ? 'edit-code-container-error'
                  : 'edit-code-container',
            }}
          >
            <CodeEditor
              onChange={(value) => update(value)}
              style={{ minHeight: 500 }}
              theme={theme}
              value={yaml}
            />
          </div>
        </Switch>
      </Modal.Content>
      <Modal.Actions>
        {(yamlError || updateError) && (
          <Message
            className="no-grow left-align"
            negative
          >
            <Icon name="x" />
            {(yamlError ?? '') + (updateError ?? '')}
          </Message>
        )}
        <Button
          disabled={!isDirty || saving || validating || loading}
          loading={saving || validating}
          onClick={() => save(yaml)}
          primary
        >
          <Icon name="save" />
          Save
        </Button>
        <Button
          negative
          onClick={onClose}
        >
          <Icon name="close" />
          Cancel
        </Button>
      </Modal.Actions>
    </Modal>
  );
};

export default EditModal;
