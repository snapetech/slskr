import {
  buildMediaServerExecutionContract,
  buildMediaServerPathDiagnostic,
  buildMediaServerSyncPreview,
  formatMediaServerExecutionContractReport,
  formatMediaServerSyncReport,
  mediaServerAutomationContracts,
  mediaServerAdapters,
} from '../../../lib/mediaServerIntegrations';
import { useState } from 'react';
import {
  Button,
  Card,
  Checkbox,
  Header,
  Icon,
  Input,
  Label,
  Message,
  Popup,
  Segment,
  Table,
} from 'semantic-ui-react';
import { useAsyncGuard } from './integrationsShared';

const MediaServerPanel = () => {
  const [activeAdapterId, setActiveAdapterId] = useState(mediaServerAdapters[0].id);
  const [automationEnabled, setAutomationEnabled] = useState(() =>
    mediaServerAutomationContracts.reduce(
      (accumulator, automation) => ({
        ...accumulator,
        [automation.id]:
          automation.id === 'playHistoryImport' || automation.id === 'completedScan',
      }),
      {},
    ),
  );
  const [baseUrl, setBaseUrl] = useState('');
  const [confirmationRequired, setConfirmationRequired] = useState(true);
  const [dedupeWindowHours, setDedupeWindowHours] = useState('24');
  const [localPath, setLocalPath] = useState('');
  const [rateLimitPerMinute, setRateLimitPerMinute] = useState('6');
  const [serverPath, setServerPath] = useState('');
  const [remotePathFrom, setRemotePathFrom] = useState('');
  const [remotePathTo, setRemotePathTo] = useState('');
  const [tokenConfigured, setTokenConfigured] = useState(false);
  const [userMappingConfigured, setUserMappingConfigured] = useState(false);
  const [copyStatus, setCopyStatus] = useState('');
  const asyncGuard = useAsyncGuard();
  const diagnostic = buildMediaServerPathDiagnostic({
    localPath,
    remotePathFrom,
    remotePathTo,
    serverPath,
  });
  const syncPreview = buildMediaServerSyncPreview({
    adapterId: activeAdapterId,
    baseUrl,
    localPath,
    remotePathFrom,
    remotePathTo,
    serverPath,
    tokenConfigured,
  });
  const executionContract = buildMediaServerExecutionContract({
    confirmationRequired,
    dedupeWindowHours,
    enabledAutomations: automationEnabled,
    rateLimitPerMinute,
    syncPreview,
    userMappingConfigured,
  });

  const copySyncReport = async () => {
    const requestId = asyncGuard.begin();
    if (requestId === null) return;
    const report = formatMediaServerSyncReport(syncPreview);
    if (!navigator.clipboard?.writeText) {
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Clipboard unavailable; copy the report from the preview text.');
      }
      asyncGuard.finish(requestId);
      return;
    }

    try {
      await navigator.clipboard.writeText(report);
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Media-server sync review copied.');
      }
    } catch {
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Unable to copy media-server sync review.');
      }
    } finally {
      asyncGuard.finish(requestId);
    }
  };

  const copyExecutionContract = async () => {
    const requestId = asyncGuard.begin();
    if (requestId === null) return;
    const report = formatMediaServerExecutionContractReport(executionContract);
    if (!navigator.clipboard?.writeText) {
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Clipboard unavailable; copy the execution contract manually.');
      }
      asyncGuard.finish(requestId);
      return;
    }

    try {
      await navigator.clipboard.writeText(report);
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Media-server execution contract copied.');
      }
    } catch {
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Unable to copy media-server execution contract.');
      }
    } finally {
      asyncGuard.finish(requestId);
    }
  };

  const toggleAutomation = (automationId, checked) => {
    setAutomationEnabled((current) => ({
      ...current,
      [automationId]: Boolean(checked),
    }));
  };

  return (
    <Card fluid>
      <Card.Content>
        <Card.Header>
          <Icon name="server" />
          Media Servers
        </Card.Header>
        <Card.Meta>
          Optional Plex, Jellyfin/Emby, and Navidrome integration planning and path diagnostics.
        </Card.Meta>
      </Card.Content>
      <Card.Content>
        <Card.Group
          itemsPerRow={3}
          stackable
        >
          {mediaServerAdapters.map((adapter) => (
            <Card
              className="media-server-adapter-card"
              key={adapter.id}
            >
              <Card.Content>
                <Card.Header>{adapter.label}</Card.Header>
                <Card.Meta>
                  {adapter.requiresToken ? 'Token required' : 'No token required'}
                </Card.Meta>
                <div className="integration-status-row">
                  {adapter.capabilities.map((capability) => (
                    <Label
                      basic
                      key={capability}
                      size="tiny"
                    >
                      {capability}
                    </Label>
                  ))}
                </div>
                <Popup
                  content={`Review ${adapter.label} sync readiness. This is local planning only and does not contact the media server.`}
                  position="top center"
                  trigger={
                    <Button
                      aria-label={`Review ${adapter.label} sync readiness`}
                      basic={activeAdapterId !== adapter.id}
                      color={activeAdapterId === adapter.id ? 'purple' : undefined}
                      onClick={() => setActiveAdapterId(adapter.id)}
                      size="tiny"
                    >
                      <Icon name="clipboard check" />
                      Review
                    </Button>
                  }
                />
              </Card.Content>
            </Card>
          ))}
        </Card.Group>

        <Segment className="integration-manual-import">
          <Header as="h4">Path Diagnostics</Header>
          <p>
            Check whether a completed file path reported by slskr maps to the
            path a media server can scan.
          </p>
          <div className="media-server-path-grid">
            <Input
              aria-label="Media server base URL"
              fluid
              label="Server URL"
              onChange={(_, { value }) => setBaseUrl(value)}
              placeholder="http://media.example.invalid"
              value={baseUrl}
            />
            <Checkbox
              aria-label="Media server token configured"
              checked={tokenConfigured}
              label="API token stored"
              onChange={(_, { checked }) => setTokenConfigured(Boolean(checked))}
              toggle
            />
            <Input
              aria-label="slskr local file path"
              fluid
              label="slskr path"
              onChange={(_, { value }) => setLocalPath(value)}
              placeholder="/downloads/complete/Artist/Album/track.flac"
              value={localPath}
            />
            <Input
              aria-label="Media server file path"
              fluid
              label="Server path"
              onChange={(_, { value }) => setServerPath(value)}
              placeholder="/library/music/Artist/Album/track.flac"
              value={serverPath}
            />
            <Input
              aria-label="Remote path map from"
              fluid
              label="Map from"
              onChange={(_, { value }) => setRemotePathFrom(value)}
              placeholder="/downloads/complete"
              value={remotePathFrom}
            />
            <Input
              aria-label="Remote path map to"
              fluid
              label="Map to"
              onChange={(_, { value }) => setRemotePathTo(value)}
              placeholder="/library/music"
              value={remotePathTo}
            />
          </div>
          <Message
            color={diagnostic.color}
            size="small"
          >
            <Message.Header>{diagnostic.status}</Message.Header>
            <p>{diagnostic.message}</p>
            {diagnostic.mappedPath && <p>Mapped path: {diagnostic.mappedPath}</p>}
          </Message>
        </Segment>

        <Segment className="media-server-sync-preview">
          <div className="integration-section-header">
            <Header as="h4">
              <Icon name="clipboard list" />
              Sync Review Plan
            </Header>
            <Popup
              content="Copy the local media-server readiness report. This does not call Plex, Jellyfin, Emby, or Navidrome."
              position="top center"
              trigger={
                <Button
                  aria-label="Copy media-server sync review"
                  onClick={copySyncReport}
                  size="small"
                >
                  <Icon name="copy" />
                  Copy Plan
                </Button>
              }
            />
          </div>
          <div className="integration-status-row">
            <Label color={syncPreview.status === 'Ready for live adapter' ? 'green' : 'orange'}>
              <Icon
                name={
                  syncPreview.status === 'Ready for live adapter'
                    ? 'check circle'
                    : 'warning sign'
                }
              />
              {syncPreview.status}
            </Label>
            <Label color="purple">{syncPreview.adapter.label}</Label>
            <Label>
              {syncPreview.readyCount}/{syncPreview.total} checks ready
            </Label>
          </div>
          <Table
            aria-label="Media server synchronization review plan"
            celled
            compact
            tabIndex={0}
          >
            <Table.Body>
              {syncPreview.checks.map((check) => (
                <Table.Row key={check.label}>
                  <Table.Cell>{check.label}</Table.Cell>
                  <Table.Cell>
                    <Label color={check.ready ? 'green' : 'orange'}>
                      {check.ready ? 'Ready' : 'Todo'}
                    </Label>
                  </Table.Cell>
                  <Table.Cell>{check.ready ? 'No action needed.' : check.action}</Table.Cell>
                </Table.Row>
              ))}
            </Table.Body>
          </Table>
          {copyStatus && (
            <Message
              info
              size="small"
            >
              {copyStatus}
            </Message>
          )}
        </Segment>

        <Segment className="media-server-sync-preview">
          <div className="integration-section-header">
            <Header as="h4">
              <Icon name="tasks" />
              Live Execution Contracts
            </Header>
            <Popup
              content="Copy the live media-server execution contract. This documents enabled automations, blockers, rate limits, dedupe, and confirmation gates without calling a media server."
              position="top center"
              trigger={
                <Button
                  aria-label="Copy media-server execution contract"
                  onClick={copyExecutionContract}
                  size="small"
                >
                  <Icon name="copy" />
                  Copy Contract
                </Button>
              }
            />
          </div>
          <div className="integration-status-row">
            <Label
              color={
                executionContract.status === 'Execution contract ready'
                  ? 'green'
                  : 'orange'
              }
            >
              <Icon
                name={
                  executionContract.status === 'Execution contract ready'
                    ? 'check circle'
                    : 'warning sign'
                }
              />
              {executionContract.status}
            </Label>
            <Label>
              {executionContract.readyCount}/{executionContract.total} checks ready
            </Label>
            <Label>
              {executionContract.enabledReadyCount}/{executionContract.enabledCount}{' '}
              enabled automations ready
            </Label>
          </div>
          <div className="media-server-path-grid">
            <Checkbox
              aria-label="Media server user mapping configured"
              checked={userMappingConfigured}
              label="User mapping configured"
              onChange={(_, { checked }) => setUserMappingConfigured(Boolean(checked))}
              toggle
            />
            <Checkbox
              aria-label="Require confirmation before live media-server actions"
              checked={confirmationRequired}
              label="Require confirmation gates"
              onChange={(_, { checked }) => setConfirmationRequired(Boolean(checked))}
              toggle
            />
            <Input
              aria-label="Media server rate limit per minute"
              fluid
              label="Rate limit/min"
              min={1}
              onChange={(_, { value }) => setRateLimitPerMinute(value)}
              type="number"
              value={rateLimitPerMinute}
            />
            <Input
              aria-label="Media server dedupe window hours"
              fluid
              label="Dedupe hours"
              min={1}
              onChange={(_, { value }) => setDedupeWindowHours(value)}
              type="number"
              value={dedupeWindowHours}
            />
          </div>
          <Table
            aria-label="Media server automation execution contracts"
            celled
            compact
            tabIndex={0}
          >
            <Table.Header>
              <Table.Row>
                <Table.HeaderCell>Automation</Table.HeaderCell>
                <Table.HeaderCell>Enabled</Table.HeaderCell>
                <Table.HeaderCell>Readiness</Table.HeaderCell>
                <Table.HeaderCell>Contract</Table.HeaderCell>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {executionContract.automations.map((automation) => (
                <Table.Row key={automation.id}>
                  <Table.Cell>
                    <strong>{automation.label}</strong>
                    <div className="integration-muted-copy">
                      {automation.description}
                    </div>
                  </Table.Cell>
                  <Table.Cell>
                    <Checkbox
                      aria-label={`Enable ${automation.label}`}
                      checked={automation.enabled}
                      onChange={(_, { checked }) =>
                        toggleAutomation(automation.id, checked)
                      }
                      toggle
                    />
                  </Table.Cell>
                  <Table.Cell>
                    <Label color={automation.ready ? 'green' : 'orange'}>
                      {automation.ready ? 'Ready' : 'Blocked'}
                    </Label>
                  </Table.Cell>
                  <Table.Cell>
                    {automation.blockedReasons.length === 0
                      ? 'All required gates are satisfied.'
                      : automation.blockedReasons.join(' ')}
                  </Table.Cell>
                </Table.Row>
              ))}
            </Table.Body>
          </Table>
          <Message
            info
            size="small"
          >
            Live execution remains disabled until a backend adapter consumes this
            contract. The Web UI exposes every automation toggle and blocker so
            enablement is visible before any media-server import, scrobble,
            acquisition queue, scan, or file action exists.
          </Message>
        </Segment>
      </Card.Content>
    </Card>
  );
};

export default MediaServerPanel;
