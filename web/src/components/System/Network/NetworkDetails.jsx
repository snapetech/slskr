import React from 'react';
import { ShrinkableButton } from '../../Shared';
import {
  Card,
  Divider,
  Grid,
  Header,
  Icon,
  Label,
  List,
  Message,
  Popup,
  Progress,
  Segment,
  Statistic,
  Table,
} from 'semantic-ui-react';

const formatTimeAgo = (dateString) => {
  if (!dateString) return 'never';
  const date = new Date(dateString);
  const now = new Date();
  const seconds = Math.floor((now - date) / 1_000);

  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3_600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86_400) return `${Math.floor(seconds / 3_600)}h ago`;
  return `${Math.floor(seconds / 86_400)}d ago`;
};

const NetworkDetails = ({ actions, data }) => {
  const {
    backfill,
    backfillProgress,
    backfilling,
    darkTheme,
    discoveredPeers,
    discoveredPeersError,
    formatBytes,
    formatNumber,
    hasStats,
    hashDb,
    mesh,
    meshPeers,
    meshPeersError,
    swarmJobs,
    syncing,
  } = data;
  const { handleBackfillFromHistory, handleSync } = actions;

  return (
    <>
      <Grid
        columns={2}
        stackable
      >
        {/* Mesh Peers */}
        <Grid.Column>
          <Segment>
            <Header as="h4">
              <Icon name="sitemap" />
              <Header.Content>
                Mesh Peers
                <Header.Subheader>
                  Connected slskr clients for hash sync
                </Header.Subheader>
              </Header.Content>
            </Header>

            {meshPeersError && meshPeers.length === 0 ? null : meshPeers.length === 0 ? (
              <Segment
                basic
                placeholder
                textAlign="center"
              >
                <Header icon>
                  <Icon name="users" />
                  No mesh peers connected
                </Header>
                <p>Other slskr clients will appear here when discovered</p>
              </Segment>
            ) : (
              <Table
                basic="very"
                compact
              >
                <Table.Header>
                  <Table.Row>
                    <Table.HeaderCell>Peer</Table.HeaderCell>
                    <Table.HeaderCell>Seq ID</Table.HeaderCell>
                    <Table.HeaderCell>Last Sync</Table.HeaderCell>
                    <Table.HeaderCell>Actions</Table.HeaderCell>
                  </Table.Row>
                </Table.Header>
                <Table.Body>
                  {meshPeers.map((peer) => (
                    <Table.Row key={peer.username}>
                      <Table.Cell>
                        <Icon
                          color="green"
                          name="circle"
                          size="tiny"
                        />{' '}
                        {peer.username}
                      </Table.Cell>
                      <Table.Cell>{peer.lastSeqId ?? '-'}</Table.Cell>
                      <Table.Cell>{formatTimeAgo(peer.lastSyncAt)}</Table.Cell>
                      <Table.Cell>
                        <ShrinkableButton
                          compact
                          disabled={syncing[peer.username]}
                          icon="sync"
                          loading={syncing[peer.username]}
                          mediaQuery="(max-width: 500px)"
                          onClick={() => handleSync(peer.username)}
                          primary
                          size="mini"
                        >
                          Sync
                        </ShrinkableButton>
                      </Table.Cell>
                    </Table.Row>
                  ))}
                </Table.Body>
              </Table>
            )}
          </Segment>
        </Grid.Column>

        {/* Discovered Peers */}
        <Grid.Column>
          <Segment>
            <Header as="h4">
              <Icon name="search" />
              <Header.Content>
                Discovered slskr Peers
                <Header.Subheader>
                  Peers with slskr capabilities detected
                </Header.Subheader>
              </Header.Content>
            </Header>

            {discoveredPeersError && discoveredPeers.length === 0 ? null : discoveredPeers.length === 0 ? (
              <Segment
                basic
                placeholder
                textAlign="center"
              >
                <Header icon>
                  <Icon name="crosshairs" />
                  No slskr peers discovered yet
                </Header>
                <p>Peers are discovered through searches and downloads</p>
              </Segment>
            ) : (
              <List
                divided
                relaxed
              >
                {discoveredPeers.slice(0, 10).map((peer) => (
                  <List.Item key={peer.username}>
                    <List.Icon
                      color="blue"
                      name="user"
                      verticalAlign="middle"
                    />
                    <List.Content>
                      <List.Header>{peer.username}</List.Header>
                      <List.Description>
                        {peer.version ?? 'slskr'} • Last seen:{' '}
                        {formatTimeAgo(peer.lastSeenAt)}
                      </List.Description>
                    </List.Content>
                  </List.Item>
                ))}
                {discoveredPeers.length > 10 && (
                  <List.Item>
                    <List.Content>
                      <em>...and {discoveredPeers.length - 10} more</em>
                    </List.Content>
                  </List.Item>
                )}
              </List>
            )}
          </Segment>
        </Grid.Column>
      </Grid>

      <Divider />

      {/* Mesh Sync Security */}
      <Segment>
        <Header as="h4">
          <Icon name="shield alternate" />
          <Header.Content>
            Mesh Sync Security
            <Header.Subheader>
              Counters for signatures, reputation, rate limits, and quarantine
            </Header.Subheader>
          </Header.Content>
        </Header>
        {mesh?.warnings?.length > 0 && (
          <Message
            attached="top"
            negative
            size="small"
          >
            <Message.Header>
              <Icon name="exclamation triangle" />
              Security threshold alerts
            </Message.Header>
            <List
              bulleted
              size="small"
            >
              {mesh.warnings.map((w, index) => (
                <List.Item key={index}>{w}</List.Item>
              ))}
            </List>
          </Message>
        )}
        <Statistic.Group
          inverted={darkTheme}
          size="small"
          widths={7}
        >
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatNumber(mesh?.signatureVerificationFailures ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Sig. failures</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatNumber(mesh?.reputationBasedRejections ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Rep. rejections</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatNumber(mesh?.rateLimitViolations ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Rate limits</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatNumber(mesh?.quarantinedPeers ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Quarantined</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatNumber(mesh?.quarantineEvents ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Quarantine events</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatNumber(mesh?.rejectedMessages ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Rejected msgs</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatNumber(mesh?.skippedEntries ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Skipped entries</Statistic.Label>
          </Statistic>
        </Statistic.Group>
      </Segment>

      <Divider />

      {/* Hash Database Details */}
      <Segment>
        <Header as="h4">
          <Icon name="database" />
          <Header.Content>
            Hash Database
            <Header.Subheader>
              Content-addressed FLAC fingerprints
            </Header.Subheader>
          </Header.Content>
          <span style={{ float: 'right', marginTop: '-0.5em' }}>
            {backfillProgress && !backfillProgress.complete && (
              <Label
                size="tiny"
                style={{ marginRight: '0.5em' }}
              >
                {backfillProgress.remainingSearches} searches left
              </Label>
            )}
            <Popup
              content={
                backfillProgress && !backfillProgress.complete
                  ? `Continue processing search history. ${backfillProgress.remainingSearches} of ${backfillProgress.totalSearches} searches remaining.`
                  : 'Scan your search history to discover FLAC files from past searches. This populates the inventory with files that can be probed for content hashes, enabling multi-source downloads for those files. Processes in batches - click multiple times for large histories.'
              }
              position="top right"
              trigger={
                <ShrinkableButton
                  compact
                  disabled={backfilling}
                  icon={
                    backfillProgress && !backfillProgress.complete
                      ? 'play'
                      : 'history'
                  }
                  loading={backfilling}
                  mediaQuery="(max-width: 500px)"
                  onClick={() => handleBackfillFromHistory(false)}
                  primary
                  size="mini"
                >
                  {backfillProgress && !backfillProgress.complete
                    ? 'Continue'
                    : 'Backfill from History'}
                </ShrinkableButton>
              }
            />
            {backfillProgress && (
              <Popup
                content="Reset progress and start backfill from the beginning"
                position="top right"
                trigger={
                  <ShrinkableButton
                    compact
                    disabled={backfilling}
                    icon="redo"
                    mediaQuery="(max-width: 500px)"
                    onClick={() => handleBackfillFromHistory(true)}
                    size="mini"
                    style={{ marginLeft: '0.3em' }}
                  >
                    Reset
                  </ShrinkableButton>
                }
              />
            )}
          </span>
        </Header>

        <Statistic.Group
          inverted={darkTheme}
          size="tiny"
          widths={4}
        >
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatNumber(hashDb?.totalEntries ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Total Entries</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats
                ? formatNumber(hashDb?.uniqueFiles ?? hashDb?.totalEntries ?? 0)
                : '—'}
            </Statistic.Value>
            <Statistic.Label>Unique Files</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats ? formatBytes(hashDb?.dbSizeBytes ?? 0) : '—'}
            </Statistic.Value>
            <Statistic.Label>Database Size</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>
              {hasStats ? hashDb?.currentSeqId ?? 0 : '—'}
            </Statistic.Value>
            <Statistic.Label>Sequence ID</Statistic.Label>
          </Statistic>
        </Statistic.Group>

        {hasStats && hashDb?.coveragePercent !== undefined && (
          <>
            <Divider hidden />
            <Progress
              color="green"
              percent={hashDb.coveragePercent}
              progress
              size="small"
            >
              Coverage of shared FLACs
            </Progress>
          </>
        )}
      </Segment>

      {/* Backfill Scheduler */}
      <Segment>
        <Header as="h4">
          <Icon name="clock" />
          <Header.Content>
            Backfill Scheduler
            <Header.Subheader>
              Conservative discovery of hashes from non-slskr peers
            </Header.Subheader>
          </Header.Content>
        </Header>

        <Grid
          columns={4}
          stackable
        >
          <Grid.Column>
            <Statistic
              inverted={darkTheme}
              size="mini"
            >
              <Statistic.Value>
                <Icon
                  color={hasStats && backfill?.isActive ? 'green' : 'grey'}
                  name="circle"
                />{' '}
                {hasStats ? (backfill?.isActive ? 'Active' : 'Idle') : '—'}
              </Statistic.Value>
              <Statistic.Label>Status</Statistic.Label>
            </Statistic>
          </Grid.Column>
          <Grid.Column>
            <Statistic
              inverted={darkTheme}
              size="mini"
            >
              <Statistic.Value>
                {hasStats ? backfill?.pendingCount ?? 0 : '—'}
              </Statistic.Value>
              <Statistic.Label>Pending Files</Statistic.Label>
            </Statistic>
          </Grid.Column>
          <Grid.Column>
            <Statistic
              inverted={darkTheme}
              size="mini"
            >
              <Statistic.Value>
                {hasStats ? backfill?.completedToday ?? 0 : '—'}
              </Statistic.Value>
              <Statistic.Label>Completed Today</Statistic.Label>
            </Statistic>
          </Grid.Column>
          <Grid.Column>
            <Statistic
              inverted={darkTheme}
              size="mini"
            >
              <Statistic.Value>
                {hasStats ? `${backfill?.discoveryRate ?? 0}/hr` : '—'}
              </Statistic.Value>
              <Statistic.Label>Discovery Rate</Statistic.Label>
            </Statistic>
          </Grid.Column>
        </Grid>
      </Segment>

      {/* Active Swarm Downloads */}
      {swarmJobs && swarmJobs.length > 0 && (
        <Segment>
          <Header as="h4">
            <Icon name="bolt" />
            <Header.Content>
              Active Swarm Downloads
              <Header.Subheader>
                Multi-source downloads in progress
              </Header.Subheader>
            </Header.Content>
          </Header>

          {swarmJobs.map((job) => (
            <Card
              fluid
              key={job.jobId}
            >
              <Card.Content>
                <Card.Header>
                  <Icon
                    color="yellow"
                    name="bolt"
                  />
                  {job.filename?.split('/').pop() ?? 'Unknown file'}
                </Card.Header>
                <Card.Meta>
                  {job.activeSources ?? 0} sources •{' '}
                  {formatBytes(job.downloadedBytes ?? 0)} /{' '}
                  {formatBytes(job.totalBytes ?? 0)}
                </Card.Meta>
                <Progress
                  active
                  color="blue"
                  percent={job.progressPercent ?? 0}
                  progress
                  size="small"
                />
                {job.workers && job.workers.length > 0 && (
                  <List
                    horizontal
                    size="small"
                  >
                    {job.workers.slice(0, 5).map((worker) => (
                      <List.Item key={worker.username}>
                        <Label size="tiny">
                          <Icon name="user" />
                          {worker.username}
                          <Label.Detail>
                            {formatBytes(worker.speedBps ?? 0)}/s
                          </Label.Detail>
                        </Label>
                      </List.Item>
                    ))}
                    {job.workers.length > 5 && (
                      <List.Item>
                        <Label size="tiny">
                          +{job.workers.length - 5} more
                        </Label>
                      </List.Item>
                    )}
                  </List>
                )}
              </Card.Content>
            </Card>
          ))}
        </Segment>
      )}
    </>
  );
};

export default NetworkDetails;
