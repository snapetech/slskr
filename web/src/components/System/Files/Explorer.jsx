import { deleteDirectory, deleteFile, list } from '../../../lib/files';
import { toDisplayError } from '../../../lib/errors';
import { formatBytes, formatDate } from '../../../lib/util';
import { LoaderSegment } from '../../Shared';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import { Header, Icon, Modal, Table } from 'semantic-ui-react';

const fileStoragePath = (subdirectory, fullName) =>
  [...subdirectory, fullName].filter(Boolean).join('/');

const reportFileOperationError = (error) => {
  console.error('[Files] Remote file operation failed:', error);
  toast.error(toDisplayError(error, 'File operation failed'));
};

const FileRow = ({
  fullName,
  length,
  modifiedAt,
  name,
  remoteFileManagement,
  root,
  subdirectory,
  onRefresh,
}) => (
  <Table.Row key={fullName}>
    <Table.Cell>
      <Icon name="file outline" />
      {name}
    </Table.Cell>
    <Table.Cell>{modifiedAt ? formatDate(modifiedAt) : ''}</Table.Cell>
    <Table.Cell>{length ? formatBytes(length) : ''}</Table.Cell>
    <Table.Cell>
      {remoteFileManagement ? (
        <Modal
          actions={[
            'Cancel',
            {
              content: 'Delete',
              key: 'done',
              negative: true,
              onClick: async () => {
                try {
                  await deleteFile({
                    path: fileStoragePath(subdirectory, fullName),
                    root,
                  });
                  await onRefresh();
                } catch (error) {
                  reportFileOperationError(error);
                }
              },
            },
          ]}
          centered
          content={`Are you sure you want to delete file '${fullName}'?`}
          header={
            <Header
              content="Confirm File Delete"
              icon="trash alternate"
            />
          }
          size="small"
          trigger={
            <Icon
              color="red"
              name="trash alternate"
              style={{ cursor: 'pointer' }}
            />
          }
        />
      ) : null}
    </Table.Cell>
  </Table.Row>
);

const DirectoryRow = ({
  deletable = true,
  fullName,
  modifiedAt,
  name,
  onClick = () => {},
  remoteFileManagement,
  root,
  subdirectory,
  onRefresh,
}) => (
  <Table.Row key={name}>
    <Table.Cell
      onClick={onClick}
      style={{ cursor: 'pointer' }}
    >
      <Icon name="folder" />
      {name}
    </Table.Cell>
    <Table.Cell>{modifiedAt ? formatDate(modifiedAt) : ''}</Table.Cell>
    <Table.Cell />
    <Table.Cell>
      {remoteFileManagement && deletable ? (
        <Modal
          actions={[
            'Cancel',
            {
              content: 'Delete',
              key: 'done',
              negative: true,
              onClick: async () => {
                try {
                  await deleteDirectory({
                    path: fileStoragePath(subdirectory, fullName),
                    root,
                  });
                  await onRefresh();
                } catch (error) {
                  reportFileOperationError(error);
                }
              },
            },
          ]}
          centered
          content={`Are you sure you want to delete directory '${fullName}'?`}
          header={
            <Header
              content="Confirm Directory Delete"
              icon="trash alternate"
            />
          }
          size="small"
          trigger={
            <Icon
              color="red"
              name="trash alternate"
              style={{ cursor: 'pointer' }}
            />
          }
        />
      ) : (
        ''
      )}
    </Table.Cell>
  </Table.Row>
);

const Explorer = ({ active = true, remoteFileManagement, root }) => {
  const [directory, setDirectory] = useState({ directories: [], files: [] });
  const [subdirectory, setSubdirectory] = useState([]);
  const [loading, setLoading] = useState(false);
  const mountedRef = useRef(false);
  const requestIdRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      requestIdRef.current += 1;
    };
  }, []);

  const refresh = useCallback(async () => {
    const requestId = ++requestIdRef.current;
    if (!active) {
      return;
    }

    setLoading(true);
    try {
      const directoryResult = await list({
        root,
        subdirectory: subdirectory.join('/'),
      });
      if (
        mountedRef.current &&
        requestId === requestIdRef.current
      ) {
        setDirectory(directoryResult || { directories: [], files: [] });
      }
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === requestIdRef.current
      ) {
        setDirectory({ directories: [], files: [] });
        reportFileOperationError(error);
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === requestIdRef.current
      ) {
        setLoading(false);
      }
    }
  }, [active, root, subdirectory]);

  useEffect(() => {
    if (!active) {
      requestIdRef.current += 1;
      setLoading(false);
      return;
    }

    void refresh();
  }, [active, refresh]);

  useEffect(() => {
    setSubdirectory([]);
  }, [root]);

  const select = ({ path }) => {
    setSubdirectory([...subdirectory, path]);
  };

  const upOneSubdirectory = () => {
    const copy = [...subdirectory];
    copy.pop();
    setSubdirectory(copy);
  };

  if (!active) {
    return (
      <Header
        className="explorer-working-directory"
        size="small"
      >
        <Icon name="folder" />
        {'/' + root + '/'}
      </Header>
    );
  }

  if (loading) {
    return <LoaderSegment />;
  }

  const total =
    (directory?.directories?.length ?? 0) +
    (directory?.files?.length ?? 0);

  return (
    <>
      <Header
        className="explorer-working-directory"
        size="small"
      >
        <Icon name="folder open" />
        {'/' + root + '/' + subdirectory.join('/')}
      </Header>
      <Table
        className="unstackable"
        size="large"
      >
        <Table.Header>
          <Table.Row>
            <Table.HeaderCell className="explorer-list-name">
              Name
            </Table.HeaderCell>
            <Table.HeaderCell className="explorer-list-date">
              Date Modified
            </Table.HeaderCell>
            <Table.HeaderCell className="explorer-list-size">
              Size
            </Table.HeaderCell>
            <Table.HeaderCell className="explorer-list-action" />
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {total === 0 ? (
            <Table.Row>
              <Table.Cell
                colSpan={99}
                style={{
                  opacity: 0.5,
                  padding: '10px !important',
                  textAlign: 'center',
                }}
              >
                No files or directories
              </Table.Cell>
            </Table.Row>
          ) : (
            <>
              {subdirectory.length > 0 && (
                <DirectoryRow
                  deletable={false}
                  fullName=".."
                  name=".."
                  onClick={upOneSubdirectory}
                  remoteFileManagement={remoteFileManagement}
                  root={root}
                  subdirectory={subdirectory}
                />
              )}
              {directory?.directories?.map((d) => (
                <DirectoryRow
                  key={d.name}
                  onClick={() => select({ path: d.name })}
                  remoteFileManagement={remoteFileManagement}
                  root={root}
                  subdirectory={subdirectory}
                  {...d}
                  onRefresh={refresh}
                />
              ))}
              {directory?.files?.map((f) => (
                <FileRow
                  key={f.name}
                  remoteFileManagement={remoteFileManagement}
                  root={root}
                  subdirectory={subdirectory}
                  {...f}
                  onRefresh={refresh}
                />
              ))}
            </>
          )}
        </Table.Body>
      </Table>
    </>
  );
};

export default Explorer;
