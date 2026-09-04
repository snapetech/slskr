import { getCurrentDebugView } from '../../../lib/options';
import { toDisplayError } from '../../../lib/errors';
import { CodeEditor, PlaceholderSegment, Switch } from '../../Shared';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import { Button, Icon, Modal } from 'semantic-ui-react';

const DebugModal = ({ onClose, open, theme }) => {
  const [loading, setLoading] = useState(true);
  const [debugView, setDebugView] = useState();
  const mountedRef = useMountedRef();
  const requestIdRef = useRef(0);

  useEffect(() => {
    const requestId = ++requestIdRef.current;
    if (!open) return undefined;

    setLoading(true);
    const load = async () => {
      try {
        const result = await getCurrentDebugView();
        if (
          mountedRef.current &&
          requestId === requestIdRef.current
        ) {
          setDebugView(result);
        }
      } catch (error) {
        console.error(error);
        if (
          mountedRef.current &&
          requestId === requestIdRef.current
        ) {
          toast.error(toDisplayError(error, 'Failed to load debug view'));
        }
      } finally {
        if (
          mountedRef.current &&
          requestId === requestIdRef.current
        ) {
          setLoading(false);
        }
      }
    };

    void load();
    return () => {
      requestIdRef.current += 1;
    };
  }, [mountedRef, open]);

  return (
    <Modal
      onClose={onClose}
      open={open}
      size="large"
    >
      <Modal.Header>
        <Icon name="bug" />
        Options (Debug View)
      </Modal.Header>
      <Modal.Content
        className="debug-view-content"
        scrolling
      >
        <Switch loading={loading && <PlaceholderSegment loading />}>
          <CodeEditor
            basicSetup={false}
            editable={false}
            style={{ minHeight: 500 }}
            theme={theme}
            value={debugView}
          />
        </Switch>
      </Modal.Content>
      <Modal.Actions>
        <Button onClick={onClose}>Close</Button>
      </Modal.Actions>
    </Modal>
  );
};

export default DebugModal;
