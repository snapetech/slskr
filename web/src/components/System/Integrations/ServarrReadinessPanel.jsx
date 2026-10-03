import * as lidarr from '../../../lib/lidarr';
import { toDisplayError } from '../../../lib/errors';
import { useState } from 'react';
import {
  Button,
  Card,
  Header,
  Icon,
  Label,
  Message,
  Popup,
  Table,
} from 'semantic-ui-react';
import {
  boolLabel,
  getLidarrOptions,
  getOption,
  useAsyncGuard,
} from './integrationsShared';
import {
  buildServarrCompatibilityPreview,
  buildServarrReadiness,
  formatServarrCompatibilityReport,
  summarizeServarrReadiness,
} from '../../../lib/servarrReadiness';

const ServarrReadinessPanel = ({ options }) => {
  const lidarrOptions = getLidarrOptions(options);
  const [copyStatus, setCopyStatus] = useState('');
  const [running, setRunning] = useState(false);
  const asyncGuard = useAsyncGuard();
  const checks = buildServarrReadiness({
    apiKey: getOption(lidarrOptions, 'apiKey', 'ApiKey'),
    autoImportCompleted: getOption(
      lidarrOptions,
      'autoImportCompleted',
      'AutoImportCompleted',
    ),
    enabled: getOption(lidarrOptions, 'enabled', 'Enabled'),
    importPathFrom: getOption(lidarrOptions, 'importPathFrom', 'ImportPathFrom'),
    importPathTo: getOption(lidarrOptions, 'importPathTo', 'ImportPathTo'),
    syncWantedToWishlist: getOption(
      lidarrOptions,
      'syncWantedToWishlist',
      'SyncWantedToWishlist',
    ),
    url: getOption(lidarrOptions, 'url', 'Url'),
  });
  const summary = summarizeServarrReadiness(checks);
  const compatibility = buildServarrCompatibilityPreview({
    apiKey: getOption(lidarrOptions, 'apiKey', 'ApiKey'),
    autoImportCompleted: getOption(
      lidarrOptions,
      'autoImportCompleted',
      'AutoImportCompleted',
    ),
    enabled: getOption(lidarrOptions, 'enabled', 'Enabled'),
    importMode: getOption(lidarrOptions, 'importMode', 'ImportMode') || 'copy',
    importPathFrom: getOption(lidarrOptions, 'importPathFrom', 'ImportPathFrom'),
    importPathTo: getOption(lidarrOptions, 'importPathTo', 'ImportPathTo'),
    syncWantedToWishlist: getOption(
      lidarrOptions,
      'syncWantedToWishlist',
      'SyncWantedToWishlist',
    ),
    url: getOption(lidarrOptions, 'url', 'Url'),
  });

  const copyCompatibilityReport = async () => {
    const requestId = asyncGuard.begin();
    if (requestId === null) return;
    const report = formatServarrCompatibilityReport(compatibility);
    if (!navigator.clipboard?.writeText) {
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Clipboard unavailable; copy the Servarr review manually.');
      }
      asyncGuard.finish(requestId);
      return;
    }

    try {
      await navigator.clipboard.writeText(report);
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Servarr compatibility review copied.');
      }
    } catch {
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus('Unable to copy Servarr compatibility review.');
      }
    } finally {
      asyncGuard.finish(requestId);
    }
  };

  const runReadyActions = async () => {
    const requestId = asyncGuard.begin();
    if (requestId === null) return;
    setRunning(true);
    setCopyStatus('');

    try {
      if (!compatibility.supportsWantedPull) {
        if (asyncGuard.isCurrent(requestId)) {
          setCopyStatus('Wanted pull is not ready; no Servarr action was run.');
        }
        return;
      }

      const result = await lidarr.syncWanted();
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus(
          `Wanted sync ran: ${result.createdCount ?? result.CreatedCount ?? 0} created, ${
            result.duplicateCount ?? result.DuplicateCount ?? 0
          } duplicates, ${result.skippedCount ?? result.SkippedCount ?? 0} skipped.`,
        );
      }
    } catch (error) {
      if (asyncGuard.isCurrent(requestId)) {
        setCopyStatus(toDisplayError(error, 'Servarr action failed.'));
      }
    } finally {
      if (asyncGuard.isCurrent(requestId)) setRunning(false);
      asyncGuard.finish(requestId);
    }
  };

  return (
    <Card fluid>
      <Card.Content>
        <Card.Header>
          <Icon name="settings" />
          Servarr Setup
        </Card.Header>
        <Card.Meta>
          Local readiness checklist for indexer/download-client style integration.
        </Card.Meta>
      </Card.Content>
      <Card.Content>
        <div className="integration-section-header">
          <Header as="h4">
            <Icon name="clipboard check" />
            Compatibility Review
          </Header>
          <Popup
            content="Copy the local Servarr compatibility review. This does not call Lidarr, create download clients, pull wanted items, or trigger imports."
            position="top center"
            trigger={
              <Button
                aria-label="Copy Servarr compatibility review"
                onClick={copyCompatibilityReport}
                size="small"
              >
                <Icon name="copy" />
                Copy Review
              </Button>
              }
            />
          <Popup
            content="Run ready Servarr actions now. Currently this calls the configured Lidarr wanted-sync endpoint when wanted pull is ready; imports still require an explicit directory in the Lidarr panel."
            position="top center"
            trigger={
              <Button
                aria-label="Run ready Servarr actions"
                disabled={!compatibility.supportsWantedPull}
                loading={running}
                onClick={runReadyActions}
                primary
                size="small"
              >
                <Icon name="play" />
                Run Ready
              </Button>
            }
          />
        </div>
        <div className="integration-status-row">
          <Label color={summary.status === 'Ready' ? 'green' : 'orange'}>
            <Icon name={summary.status === 'Ready' ? 'check circle' : 'warning sign'} />
            {summary.status}
          </Label>
          <Label>
            {summary.ready}/{summary.total} checks ready
          </Label>
          <Label color={compatibility.supportsWantedPull ? 'green' : 'grey'}>
            Wanted Pull {compatibility.supportsWantedPull ? 'Ready' : 'Not Ready'}
          </Label>
          <Label color={compatibility.supportsCompletedImport ? 'green' : 'grey'}>
            Import {compatibility.supportsCompletedImport ? 'Ready' : 'Not Ready'}
          </Label>
        </div>
        <Table
          aria-label="Servarr integration readiness checks"
          celled
          compact
          tabIndex={0}
        >
          <Table.Header>
            <Table.Row>
              <Table.HeaderCell>Check</Table.HeaderCell>
              <Table.HeaderCell>Status</Table.HeaderCell>
              <Table.HeaderCell>Why it matters</Table.HeaderCell>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {checks.map((check) => (
              <Table.Row key={check.id}>
                <Table.Cell>{check.title}</Table.Cell>
                <Table.Cell>
                  {boolLabel(check.ready, 'Ready', 'Needs Setup')}
                </Table.Cell>
                <Table.Cell>{check.description}</Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
        <Message
          info
          size="small"
        >
          This checklist is diagnostic only. It does not register indexers,
          create download clients, pull wanted items, or trigger imports.
        </Message>
        {compatibility.actions.length > 0 && (
          <Message
            size="small"
            warning
          >
            <Message.Header>Compatibility Actions</Message.Header>
            <ul>
              {compatibility.actions.map((action) => (
                <li key={action}>{action}</li>
              ))}
            </ul>
          </Message>
        )}
        {copyStatus && (
          <Message
            info
            size="small"
          >
            {copyStatus}
          </Message>
        )}
      </Card.Content>
    </Card>
  );
};

export default ServarrReadinessPanel;
