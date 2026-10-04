import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import PodWorkflowNotice from './PodWorkflowNotice';
import PodDhtPublishingPanel from './PodDhtPublishingPanel';
import PodMembershipManagementPanel from './PodMembershipManagementPanel';
import PodMembershipVerificationPanel from './PodMembershipVerificationPanel';
import PodMessageStoragePanel from './PodMessageStoragePanel';
import PodDiscoveryPanel from './PodDiscoveryPanel';
import PodJoinLeavePanel from './PodJoinLeavePanel';
import PodMessageRoutingPanel from './PodMessageRoutingPanel';
import PodMessageBackfillPanel from './PodMessageBackfillPanel';
import PodChannelManagementPanel from './PodChannelManagementPanel';
import PodContentLinkingPanel from './PodContentLinkingPanel';
import PodOpinionsPanel from './PodOpinionsPanel';
import PodMessageSigningPanel from './PodMessageSigningPanel';
import MediaCoreStatisticsDashboardPanel from './MediaCoreStatisticsDashboardPanel';
import MediaCoreDescriptorManagementPanel from './MediaCoreDescriptorManagementPanel';
import MediaCoreContentRegistryPanel from './MediaCoreContentRegistryPanel';
import MediaCoreContentGraphPanel from './MediaCoreContentGraphPanel';
import MediaCoreMetadataPortabilityPanel from './MediaCoreMetadataPortabilityPanel';
import MediaCoreContentHashingPanel from './MediaCoreContentHashingPanel';
import MediaCoreSimilarityAnalysisPanel from './MediaCoreSimilarityAnalysisPanel';
import { podWorkflowFilterOptions, podWorkflowSections } from './mediaCoreWorkflows';
import { usePolling } from '../../../lib/usePolling';
import React, { useCallback, useEffect, useRef } from 'react';
import { toast } from 'react-toastify';
import {
  Card,
  Checkbox,
  Dropdown,
  Form,
  Grid,
  Header,
  Icon,
  Input,
  Label,
  List,
  Loader,
  Message,
  Segment,
  Statistic,
  TextArea,
} from 'semantic-ui-react';

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

const MediaCore = () => {
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [stats, setStats] = useMountedState(mountedRef, null);
  const [loading, setLoading] = useMountedState(mountedRef, true);
  const [error, setError] = useMountedState(mountedRef, null);
  const [podWorkflowFilter, setPodWorkflowFilter] = useMountedState(mountedRef, 'all');

  // Form state
  const [contentId, setContentId] = useMountedState(mountedRef, '');




  const [supportedAlgorithms, setSupportedAlgorithms] = useMountedState(mountedRef, null);















  // Shared with Pod Membership Management and Message Signing.
  const [verifyingMembership, setVerifyingMembership] = useMountedState(mountedRef, false);
  // These values are shared with existing verification handlers in this screen.
  const [messageToVerify, setMessageToVerify] = useMountedState(mountedRef, '');
  const [verificationResult, setVerificationResult] = useMountedState(
    mountedRef,
    null,
  );
  const fetchStats = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await mediacore.getContentIdStats();
      setStats(data);
    } catch (error_) {
      setError(toDisplayError(error_));
    } finally {
      setLoading(false);
    }
  }, []);

  usePolling(fetchStats, 60_000);

  const loadSupportedAlgorithms = async () => {
    try {
      const result = await mediacore.getSupportedHashAlgorithms();
      setSupportedAlgorithms({
        ...result,
        algorithms: Array.isArray(result?.algorithms) ? result.algorithms : [],
        descriptions:
          result?.descriptions && typeof result.descriptions === 'object'
            ? result.descriptions
            : {},
      });
    } catch (error_) {
      console.error('Failed to load hash algorithms:', error_);
    }
  };

  useEffect(() => {
    loadSupportedAlgorithms();
  }, []);

  const isPodWorkflowVisible = (sectionId) =>
    podWorkflowFilter === 'all' || podWorkflowFilter === sectionId;
  const selectedPodWorkflow = podWorkflowSections.find(
    (section) => section.href.slice(1) === podWorkflowFilter,
  );
  if (loading && !stats) {
    return (
      <Segment>
        <Loader
          active
          inline="centered"
        >
          Loading MediaCore statistics...
        </Loader>
      </Segment>
    );
  }

  if (error && !stats) {
    return (
      <Message error>
        <Message.Header>Failed to load MediaCore statistics</Message.Header>
        <p>{error}</p>
      </Message>
    );
  }

  return (
    <div>
      <Header as="h2">
        <Icon name="database" />
        MediaCore ContentID Registry
      </Header>

      <Segment>
        <Header as="h3">
          <Icon name="sitemap" />
          Pod Workflow Index
        </Header>
        <Message warning>
          Pod workflows mix read-only diagnostics with operations that publish
          metadata, membership records, messages, opinions, or key material.
          Use this index to jump to the intended workflow before running an
          action.
        </Message>
        <Form>
          <Form.Field>
            <label>Workflow focus</label>
            <Dropdown
              aria-label="Pod workflow focus"
              onChange={(_, { value }) => setPodWorkflowFilter(value)}
              options={podWorkflowFilterOptions}
              selection
              value={podWorkflowFilter}
            />
          </Form.Field>
        </Form>
        {podWorkflowFilter !== 'all' && (
          <Message
            info
            size="small"
          >
            <Message.Content>
              <Message.Header>Focused pod workflow</Message.Header>
              Showing {selectedPodWorkflow?.label || 'one pod workflow'}. Choose
              "Show all pod workflows" or use the reset action to return to the
              complete MediaCore pod surface.
              <div style={{ marginTop: '0.75em' }}>
                <Button
                  basic
                  onClick={() => setPodWorkflowFilter('all')}
                  size="tiny"
                >
                  Show all pod workflows
                </Button>
              </div>
            </Message.Content>
          </Message>
        )}
        <Card.Group itemsPerRow={3} stackable>
          {podWorkflowSections.map((section) => (
            <Card
              as="a"
              color={podWorkflowFilter === section.href.slice(1) ? 'blue' : undefined}
              href={section.href}
              key={section.href}
              onClick={() => setPodWorkflowFilter(section.href.slice(1))}
              raised={podWorkflowFilter === section.href.slice(1)}
            >
              <Card.Content>
                <Card.Header>{section.label}</Card.Header>
                <Card.Meta>{section.risk}</Card.Meta>
                <Card.Description>{section.description}</Card.Description>
              </Card.Content>
            </Card>
          ))}
        </Card.Group>
      </Segment>

      <Grid stackable>
        {/* Statistics Overview */}
        <Grid.Column width={16}>
          <Segment>
            <Header as="h3">Registry Statistics</Header>
            <Statistic.Group size="small">
              <Statistic>
                <Statistic.Value>{stats?.totalMappings || 0}</Statistic.Value>
                <Statistic.Label>Total Mappings</Statistic.Label>
              </Statistic>
              <Statistic>
                <Statistic.Value>{stats?.totalDomains || 0}</Statistic.Value>
                <Statistic.Label>Domains</Statistic.Label>
              </Statistic>
            </Statistic.Group>

            {stats?.mappingsByDomain &&
              Object.keys(stats.mappingsByDomain).length > 0 && (
                <div style={{ marginTop: '1em' }}>
                  <Header as="h4">Mappings by Domain</Header>
                  <List horizontal>
                    {Object.entries(stats.mappingsByDomain).map(
                      ([domain, count]) => (
                        <List.Item key={domain}>
                          <Label>
                            {domain}
                            <Label.Detail>{count}</Label.Detail>
                          </Label>
                        </List.Item>
                      ),
                    )}
                  </List>
                </div>
              )}
          </Segment>
        </Grid.Column>

        <MediaCoreContentRegistryPanel
          setContentId={setContentId}
          setError={setError}
          setStats={setStats}
        />

        <MediaCoreContentGraphPanel />

        <MediaCoreContentHashingPanel supportedAlgorithms={supportedAlgorithms} />

        <MediaCoreSimilarityAnalysisPanel />

        <MediaCoreMetadataPortabilityPanel />

        <MediaCoreDescriptorManagementPanel setVerificationResult={setVerificationResult} />

        <MediaCoreStatisticsDashboardPanel />

        <PodDhtPublishingPanel visible={isPodWorkflowVisible('podcore-dht-publishing')} />

        <PodMembershipManagementPanel
          setVerifyingMembership={setVerifyingMembership}
          verifyingMembership={verifyingMembership}
          visible={isPodWorkflowVisible('pod-membership-management')}
        />

        <PodMembershipVerificationPanel
          messageToVerify={messageToVerify}
          setMessageToVerify={setMessageToVerify}
          setVerifyingMembership={setVerifyingMembership}
          verifyingMembership={verifyingMembership}
          visible={isPodWorkflowVisible('pod-membership-verification')}
        />

        <PodDiscoveryPanel visible={isPodWorkflowVisible('pod-discovery')} />

        <PodJoinLeavePanel visible={isPodWorkflowVisible('pod-join-leave')} />

        <PodMessageRoutingPanel visible={isPodWorkflowVisible('pod-message-routing')} />

        <PodMessageStoragePanel visible={isPodWorkflowVisible('pod-message-storage')} />

        <PodMessageBackfillPanel visible={isPodWorkflowVisible('pod-message-backfill')} />

        <PodChannelManagementPanel visible={isPodWorkflowVisible('pod-channel-management')} />

        <PodContentLinkingPanel
          contentId={contentId}
          setContentId={setContentId}
          visible={isPodWorkflowVisible('pod-content-linking')}
        />

        <PodOpinionsPanel visible={isPodWorkflowVisible('pod-opinion-management')} />

        <PodMessageSigningPanel
          messageToVerify={messageToVerify}
          setMessageToVerify={setMessageToVerify}
          setVerificationResult={setVerificationResult}
          verificationResult={verificationResult}
          visible={isPodWorkflowVisible('pod-message-signing')}
        />

        {/* Supported Algorithms Info */}
        {supportedAlgorithms && (
          <Grid.Column width={16}>
            <Segment>
              <Header as="h3">
                <Icon name="cogs" />
                Supported Hash Algorithms
              </Header>
              <List
                divided
                relaxed
              >
                {supportedAlgorithms.algorithms.map((alg) => (
                  <List.Item key={alg}>
                    <List.Content>
                      <List.Header>{alg}</List.Header>
                      <List.Description>
                        {supportedAlgorithms.descriptions[alg]}
                      </List.Description>
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            </Segment>
          </Grid.Column>
        )}
      </Grid>
    </div>
  );
};

export default MediaCore;
