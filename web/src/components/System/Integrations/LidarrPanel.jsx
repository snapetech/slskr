import * as lidarr from '../../../lib/lidarr';
import { toDisplayError } from '../../../lib/errors';
import { useMemo, useState } from 'react';
import {
  Button,
  Card,
  Header,
  Icon,
  Input,
  Label,
  Message,
  Popup,
  Segment,
  Table,
} from 'semantic-ui-react';
import {
  boolLabel,
  getLidarrOptions,
  getOption,
  useAsyncGuard,
  valueOrDash,
} from './integrationsShared';

const LidarrPanel = ({ options }) => {
  const lidarrOptions = getLidarrOptions(options);
  const [status, setStatus] = useState(null);
  const [wanted, setWanted] = useState([]);
  const [importHistory, setImportHistory] = useState([]);
  const [syncResult, setSyncResult] = useState(null);
  const [importDirectory, setImportDirectory] = useState('');
  const [importResult, setImportResult] = useState(null);
  const [importRetryResult, setImportRetryResult] = useState(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState('');
  const asyncGuard = useAsyncGuard();
  const enabled = getOption(lidarrOptions, 'enabled', 'Enabled');

  const maskedApiKey = useMemo(() => {
    const apiKey = getOption(lidarrOptions, 'apiKey', 'ApiKey');
    return apiKey ? 'Configured' : 'Not configured';
  }, [lidarrOptions]);

  const run = async (name, action) => {
    const requestId = asyncGuard.begin();
    if (requestId === null) return;
    setLoading(name);
    setError('');

    try {
      await action(() => asyncGuard.isCurrent(requestId));
    } catch (error) {
      if (asyncGuard.isCurrent(requestId)) {
        setError(toDisplayError(error, 'Lidarr request failed'));
      }
    } finally {
      if (asyncGuard.isCurrent(requestId)) setLoading('');
      asyncGuard.finish(requestId);
    }
  };

  const loadImportHistory = async (isCurrent = () => true) => {
    const data = await lidarr.getImportHistory({ limit: 50 });
    if (isCurrent()) {
      setImportHistory(
        Array.isArray(data)
          ? data.filter(
              (record) =>
                record && typeof record === 'object' && !Array.isArray(record),
            )
          : [],
      );
    }
  };

  const retryImport = async (historyId) => {
    await run('retry', async (isCurrent) => {
      const result = await lidarr.retryImport(historyId);
      if (isCurrent()) setImportRetryResult(result);
      await loadImportHistory(isCurrent);
    });
  };

  return (
    <Card fluid>
      <Card.Content>
        <Card.Header>
          <Icon name="music" />
          Lidarr
        </Card.Header>
        <Card.Meta>Wanted-album sync and completed-download import bridge.</Card.Meta>
      </Card.Content>
      <Card.Content>
        <div className="integration-status-row">
          {boolLabel(enabled)}
          {boolLabel(
            getOption(lidarrOptions, 'syncWantedToWishlist', 'SyncWantedToWishlist'),
            'Wanted Sync',
            'Wanted Sync Off',
          )}
          {boolLabel(
            getOption(lidarrOptions, 'autoImportCompleted', 'AutoImportCompleted'),
            'Auto Import',
            'Auto Import Off',
          )}
          <Label>
            <Icon name={maskedApiKey === 'Configured' ? 'key' : 'warning sign'} />
            API Key {maskedApiKey}
          </Label>
        </div>
        <Table
          basic="very"
          compact
          definition
        >
          <Table.Body>
            <Table.Row>
              <Table.Cell>URL</Table.Cell>
              <Table.Cell>{valueOrDash(getOption(lidarrOptions, 'url', 'Url'))}</Table.Cell>
            </Table.Row>
            <Table.Row>
              <Table.Cell>Timeout</Table.Cell>
              <Table.Cell>
                {valueOrDash(getOption(lidarrOptions, 'timeoutSeconds', 'TimeoutSeconds'))}
                {' s'}
              </Table.Cell>
            </Table.Row>
            <Table.Row>
              <Table.Cell>Sync Interval</Table.Cell>
              <Table.Cell>
                {valueOrDash(getOption(lidarrOptions, 'syncIntervalSeconds', 'SyncIntervalSeconds'))}
                {' s'}
              </Table.Cell>
            </Table.Row>
            <Table.Row>
              <Table.Cell>Import Mode</Table.Cell>
              <Table.Cell>{valueOrDash(getOption(lidarrOptions, 'importMode', 'ImportMode'))}</Table.Cell>
            </Table.Row>
            <Table.Row>
              <Table.Cell>Import Path Map</Table.Cell>
              <Table.Cell>
                {valueOrDash(getOption(lidarrOptions, 'importPathFrom', 'ImportPathFrom'))}
                {' -> '}
                {valueOrDash(getOption(lidarrOptions, 'importPathTo', 'ImportPathTo'))}
              </Table.Cell>
            </Table.Row>
          </Table.Body>
        </Table>
        {error && (
          <Message
            negative
            size="small"
          >
            {error}
          </Message>
        )}
        <div className="integration-actions">
          <Popup
            content="Fetch Lidarr system status using the configured URL and API key."
            trigger={
              <Button
                icon
                labelPosition="left"
                loading={loading === 'status'}
                onClick={() =>
                  run('status', async (isCurrent) => {
                    const result = await lidarr.getStatus();
                    if (isCurrent()) setStatus(result);
                  })
                }
              >
                <Icon name="heartbeat" />
                Check Status
              </Button>
            }
          />
          <Popup
            content="Preview Lidarr wanted albums that can be synced into slskr Wishlist."
            trigger={
              <Button
                icon
                labelPosition="left"
                loading={loading === 'wanted'}
                onClick={() =>
                  run('wanted', async (isCurrent) => {
                    const result = await lidarr.getWantedMissing({ pageSize: 25 });
                    if (isCurrent()) setWanted(Array.isArray(result) ? result : []);
                  })
                }
              >
                <Icon name="list" />
                Load Wanted
              </Button>
            }
          />
          <Popup
            content="Create or refresh slskr Wishlist entries from Lidarr wanted albums."
            trigger={
              <Button
                icon
                labelPosition="left"
                loading={loading === 'sync'}
                onClick={() =>
                  run('sync', async (isCurrent) => {
                    const result = await lidarr.syncWanted();
                    if (isCurrent()) setSyncResult(result);
                  })
                }
                primary
              >
                <Icon name="sync" />
                Sync Wanted
              </Button>
            }
          />
          <Popup
            content="Load the recent completed-download import history. Failed and skipped records can be retried when the recorded source is still available."
            trigger={
              <Button
                icon
                labelPosition="left"
                loading={loading === 'history'}
                onClick={() => run('history', loadImportHistory)}
              >
                <Icon name="history" />
                Load Import History
              </Button>
            }
          />
        </div>
        {status && (
          <Message
            positive
            size="small"
          >
            Lidarr responded: {status.appName || status.AppName || 'Lidarr'}{' '}
            {status.version || status.Version || ''}
          </Message>
        )}
        {syncResult && (
          <Message
            info
            size="small"
          >
            Wanted sync: {syncResult.createdCount ?? syncResult.CreatedCount ?? 0} created,{' '}
            {syncResult.duplicateCount ?? syncResult.DuplicateCount ?? 0} duplicates,{' '}
            {syncResult.skippedCount ?? syncResult.SkippedCount ?? 0} skipped.
          </Message>
        )}
        {wanted.length > 0 && (
          <Table
            celled
            compact
          >
            <Table.Header>
              <Table.Row>
                <Table.HeaderCell>Artist</Table.HeaderCell>
                <Table.HeaderCell>Album</Table.HeaderCell>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {wanted.slice(0, 10).map((album) => (
                <Table.Row key={album.id || album.Id || `${album.title}-${album.foreignAlbumId}`}>
                  <Table.Cell>
                    {album.artist?.artistName || album.Artist?.ArtistName || '-'}
                  </Table.Cell>
                  <Table.Cell>{album.title || album.Title || '-'}</Table.Cell>
                </Table.Row>
              ))}
            </Table.Body>
          </Table>
        )}
        {importRetryResult && (
          <Message
            info
            size="small"
          >
            {importRetryResult.commandId || importRetryResult.CommandId
              ? `Lidarr import retry queued: ${importRetryResult.safeCandidateCount || importRetryResult.SafeCandidateCount || 0} file(s)`
              : importRetryResult.skippedReason ||
                importRetryResult.SkippedReason ||
                'Lidarr could not find a safe file to import on retry'}
          </Message>
        )}
        {importHistory.length > 0 && (
          <Segment>
            <Header as="h4">
              <Icon name="history" />
              Import History
            </Header>
            <Table
              celled
              compact
            >
              <Table.Header>
                <Table.Row>
                  <Table.HeaderCell>Status</Table.HeaderCell>
                  <Table.HeaderCell>Directory</Table.HeaderCell>
                  <Table.HeaderCell>Completed</Table.HeaderCell>
                  <Table.HeaderCell>Details</Table.HeaderCell>
                  <Table.HeaderCell>Action</Table.HeaderCell>
                </Table.Row>
              </Table.Header>
              <Table.Body>
                {importHistory.map((record) => {
                  const historyId = record.id || record.Id || record.historyId;
                  const directory = record.directory || record.Directory || '-';
                  const detail =
                    record.errorMessage ||
                    record.ErrorMessage ||
                    record.skippedReason ||
                    record.SkippedReason ||
                    `${record.safeCandidateCount || record.SafeCandidateCount || 0} safe candidate(s)`;

                  return (
                    <Table.Row key={historyId || `${directory}-${record.completedAt || record.CompletedAt}`}>
                      <Table.Cell>{record.status || record.Status || '-'}</Table.Cell>
                      <Table.Cell>{directory}</Table.Cell>
                      <Table.Cell>
                        {record.completedAt || record.CompletedAt || record.startedAt || record.StartedAt || '-'}
                      </Table.Cell>
                      <Table.Cell>{detail}</Table.Cell>
                      <Table.Cell>
                        <Button
                          aria-label={`Retry Lidarr import for ${directory}`}
                          disabled={!historyId}
                          loading={loading === 'retry'}
                          onClick={() => retryImport(historyId)}
                          size="small"
                        >
                          Retry
                        </Button>
                      </Table.Cell>
                    </Table.Row>
                  );
                })}
              </Table.Body>
            </Table>
          </Segment>
        )}
        <Segment className="integration-manual-import">
          <Header as="h4">Manual Import</Header>
          <Input
            action={{
              content: 'Import',
              disabled: !importDirectory.trim(),
              icon: 'download',
              loading: loading === 'import',
                onClick: () =>
                  run('import', async (isCurrent) => {
                    const result = await lidarr.importCompletedDirectory({
                      directory: importDirectory.trim(),
                    });
                    if (isCurrent()) setImportResult(result);
                  }),
            }}
            fluid
            onChange={(_, { value }) => setImportDirectory(value)}
            placeholder="Completed download directory visible to slskr..."
            value={importDirectory}
          />
          {importResult && (
            <Message
              size="small"
              warning={Boolean(importResult.skippedReason || importResult.SkippedReason)}
            >
              {importResult.skippedReason || importResult.SkippedReason
                ? `Skipped: ${importResult.skippedReason || importResult.SkippedReason}`
                : `Queued Lidarr command ${importResult.commandId || importResult.CommandId || '-'}`}
            </Message>
          )}
        </Segment>
      </Card.Content>
    </Card>
  );
};

export default LidarrPanel;
