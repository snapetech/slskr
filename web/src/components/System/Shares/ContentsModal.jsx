import { browse } from '../../../lib/shares';
import { toDisplayError } from '../../../lib/errors';
import { CodeEditor, LoaderSegment, Switch } from '../../Shared';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useEffect, useRef, useState } from 'react';
import { Button, Icon, Message, Modal } from 'semantic-ui-react';

const ContentsModal = ({ onClose, share, theme }) => {
  const [loading, setLoading] = useState(true);
  const [contents, setContents] = useState();
  const [error, setError] = useState();
  const mountedRef = useMountedRef();
  const requestIdRef = useRef(0);

  const { id, localPath, remotePath } = share || {};

  useEffect(() => {
    const requestId = ++requestIdRef.current;
    const fetchContents = async () => {
      setLoading(true);
      setError(undefined);

      try {
        const result = await browse({ id });
        const directories = (Array.isArray(result) ? result : []).map(
          (directory) => {
            const directoryName = String(directory?.name ?? '');
            const lines = [directoryName.replace(remotePath ?? '', localPath ?? '')];
            const directoryFilesOrderedByFilename = Array.isArray(
              directory?.files,
            )
              ? [...directory.files].sort((file1, file2) =>
                  String(file1?.filename ?? '').localeCompare(
                    String(file2?.filename ?? ''),
                  ),
                )
              : [];

            for (const file of directoryFilesOrderedByFilename) {
              lines.push(
                '\t' + String(file?.filename ?? '').replace(remotePath ?? '', ''),
              );
            }

            lines.push('');

            return lines.join('\n');
          },
        );

        if (
          mountedRef.current &&
          requestId === requestIdRef.current
        ) {
          setContents(directories.join('\n'));
        }
      } catch (browseError) {
        if (
          mountedRef.current &&
          requestId === requestIdRef.current
        ) {
          setError(toDisplayError(browseError, 'Failed to load share contents'));
          setContents();
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

    if (id) {
      void fetchContents();
    } else {
      setLoading(true);
      setContents();
      setError(undefined);
    }

    return () => {
      requestIdRef.current += 1;
    };
  }, [id, localPath, mountedRef, remotePath]);

  return (
    <Modal
      onClose={onClose}
      open={share}
      size="large"
    >
      <Modal.Header>
        <Icon name="folder" />
        {localPath}
      </Modal.Header>
      <Modal.Content
        className="share-ls-content"
        scrolling
      >
        <Switch
          error={error && <Message negative>{error}</Message>}
          loading={loading && <LoaderSegment className="modal-loader" />}
        >
          <CodeEditor
            basicSetup={false}
            editable={false}
            style={{ minHeight: 500 }}
            theme={theme}
            value={contents || ''}
          />
        </Switch>
      </Modal.Content>
      <Modal.Actions>
        <Button onClick={onClose}>Close</Button>
      </Modal.Actions>
    </Modal>
  );
};

export default ContentsModal;
