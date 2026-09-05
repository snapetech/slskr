// <copyright file="index.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import * as quarantineJuryApi from '../../../lib/quarantineJury';
import { toDisplayError } from '../../../lib/errors';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  Button,
  Card,
  Form,
  Header,
  Icon,
  Label,
  Loader,
  Message,
  Modal,
  Popup,
  Segment,
  Statistic,
  Table,
} from 'semantic-ui-react';

const verdictNames = {
  0: 'Needs Manual Review',
  1: 'Uphold Quarantine',
  2: 'Release Candidate',
  NeedsManualReview: 'Needs Manual Review',
  ReleaseCandidate: 'Release Candidate',
  UpholdQuarantine: 'Uphold Quarantine',
};

const asArray = (value) => (Array.isArray(value) ? value : []);
const asObject = (value) =>
  value && typeof value === 'object' && !Array.isArray(value) ? value : {};
const toText = (value, fallback = '') => {
  if (typeof value === 'string' || typeof value === 'number') return String(value);
  return fallback;
};
const toCount = (value, fallback = 0) => {
  const count = Number(value);
  return Number.isFinite(count) && count >= 0 ? count : fallback;
};
const toBoolean = (value, fallback = false) => {
  if (typeof value === 'boolean') return value;
  if (value === 1 || value === '1' || value === 'true') return true;
  if (value === 0 || value === '0' || value === 'false') return false;
  return fallback;
};
const asTextArray = (value) =>
  asArray(value)
    .map((item) => toText(item))
    .filter(Boolean);

const verdictColor = (verdict) => {
  const label = normalizeVerdict(verdict);
  if (label === 'Release Candidate') return 'green';
  if (label === 'Uphold Quarantine') return 'red';
  return 'yellow';
};

const normalizeVerdict = (verdict) =>
  verdictNames[verdict] || toText(verdict, 'Needs Manual Review');

const valueOrDash = (value) => {
  if (Array.isArray(value)) return value.length > 0 ? asTextArray(value).join(', ') || '-' : '-';
  return toText(value, '-') || '-';
};

const normalizeRequest = (request) => {
  const source = asObject(request);
  return {
    createdAt: toText(source.createdAt ?? source.CreatedAt),
    evidence: asArray(source.evidence ?? source.Evidence),
    id: toText(source.id ?? source.Id),
    jurors: asTextArray(source.jurors ?? source.Jurors),
    localReason: toText(source.localReason ?? source.LocalReason),
    minJurorVotes: toCount(source.minJurorVotes ?? source.MinJurorVotes, 2),
  };
};

const normalizeEvidence = (evidence) => {
  const source = asObject(evidence);
  return {
    reference: toText(source.reference ?? source.Reference),
    summary: toText(source.summary ?? source.Summary),
    type: toText(source.type ?? source.Type),
  };
};

const normalizeAggregate = (aggregate) => {
  const source = asObject(aggregate);
  return {
    dissentingJurors: asTextArray(
      source.dissentingJurors ?? source.DissentingJurors,
    ),
    quorumReached: toBoolean(source.quorumReached ?? source.QuorumReached),
    reason: toText(source.reason ?? source.Reason),
    recommendation:
      toText(
        source.recommendation ?? source.Recommendation,
        'NeedsManualReview',
      ),
    requiredVotes: toCount(source.requiredVotes ?? source.RequiredVotes),
    totalVerdicts: toCount(source.totalVerdicts ?? source.TotalVerdicts),
    verdictCounts: Object.entries(
      source.verdictCounts &&
        typeof source.verdictCounts === 'object' &&
        !Array.isArray(source.verdictCounts)
        ? source.verdictCounts
        : {},
    ).reduce((counts, [key, value]) => {
      counts[key] = toCount(value);
      return counts;
    }, {}),
  };
};

const normalizeVerdictRecord = (verdict) => {
  const source = asObject(verdict);
  return {
    createdAt: toText(source.createdAt ?? source.CreatedAt),
    evidence: asArray(source.evidence ?? source.Evidence),
    id: toText(source.id ?? source.Id),
    juror: toText(source.juror ?? source.Juror),
    reason: toText(source.reason ?? source.Reason),
    verdict: source.verdict ?? source.Verdict ?? 'NeedsManualReview',
  };
};

const normalizeRouteAttempt = (attempt) => {
  const source = asObject(attempt);
  return {
    channelId: toText(source.channelId ?? source.ChannelId),
    createdAt: toText(source.createdAt ?? source.CreatedAt),
    errorMessage: toText(source.errorMessage ?? source.ErrorMessage),
    failedJurors: asTextArray(source.failedJurors ?? source.FailedJurors),
    id: toText(source.id ?? source.Id),
    podId: toText(source.podId ?? source.PodId),
    routedJurors: asTextArray(source.routedJurors ?? source.RoutedJurors),
    success: toBoolean(source.success ?? source.Success),
    targetJurors: asTextArray(source.targetJurors ?? source.TargetJurors),
  };
};

const normalizeAcceptance = (acceptance) => {
  const source = asObject(acceptance);
  return {
    acceptedBy: toText(source.acceptedBy ?? source.AcceptedBy),
    createdAt: toText(source.createdAt ?? source.CreatedAt),
    id: toText(source.id ?? source.Id),
    note: toText(source.note ?? source.Note),
  };
};

const normalizeReview = (review) => {
  const source = asObject(review);
  return {
    acceptance: (source.acceptance ?? source.Acceptance)
      ? normalizeAcceptance(source.acceptance ?? source.Acceptance)
      : null,
    acceptanceReason: toText(
      source.acceptanceReason ?? source.AcceptanceReason,
    ),
    aggregate: normalizeAggregate(source.aggregate ?? source.Aggregate),
    canAcceptReleaseCandidate:
      toBoolean(
        source.canAcceptReleaseCandidate ?? source.CanAcceptReleaseCandidate,
      ),
    request: normalizeRequest(source.request ?? source.Request),
    routeAttempts: asArray(
      source.routeAttempts ?? source.RouteAttempts,
    ).map(normalizeRouteAttempt),
    verdicts: asArray(source.verdicts ?? source.Verdicts).map(
      normalizeVerdictRecord,
    ),
  };
};

const formatDate = (value) => {
  const text = toText(value);
  if (!text) return '-';
  const date = new Date(text);
  return Number.isNaN(date.getTime()) ? '-' : date.toLocaleString();
};

const getVerdictCount = (counts = {}, key, numericKey) =>
  toCount(counts[key] ?? counts[numericKey] ?? counts[String(numericKey)]);

const parseJurors = (value = '') =>
  value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);

const EvidenceTable = ({ evidence = [] }) => (
  <Table
    compact
    size="small"
  >
    <Table.Header>
      <Table.Row>
        <Table.HeaderCell>Type</Table.HeaderCell>
        <Table.HeaderCell>Reference</Table.HeaderCell>
        <Table.HeaderCell>Summary</Table.HeaderCell>
      </Table.Row>
    </Table.Header>
    <Table.Body>
      {evidence.length === 0 ? (
        <Table.Row>
          <Table.Cell colSpan={3}>No evidence supplied.</Table.Cell>
        </Table.Row>
      ) : (
        evidence.map((item, index) => {
          const normalized = normalizeEvidence(item);
          return (
            <Table.Row key={`${normalized.type}-${normalized.reference}-${index}`}>
              <Table.Cell>{normalized.type}</Table.Cell>
              <Table.Cell>{normalized.reference || '-'}</Table.Cell>
              <Table.Cell>{normalized.summary || '-'}</Table.Cell>
            </Table.Row>
          );
        })
      )}
    </Table.Body>
  </Table>
);

const QuarantineJury = () => {
  const [requests, setRequests] = useState([]);
  const [selectedId, setSelectedId] = useState('');
  const [review, setReview] = useState(null);
  const [loadingRequests, setLoadingRequests] = useState(true);
  const [loadingReview, setLoadingReview] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [errorContext, setErrorContext] = useState('action');
  const [routeForm, setRouteForm] = useState({
    channelId: '',
    podId: 'quarantine-jury',
    senderPeerId: 'local-quarantine-jury',
    targetJurors: '',
  });
  const [acceptForm, setAcceptForm] = useState({
    acceptedBy: 'local-user',
    note: '',
  });
  const [acceptOpen, setAcceptOpen] = useState(false);
  const [message, setMessage] = useState('');
  const mountedRef = useMountedRef();
  const selectedIdRef = useRef('');
  const requestIdsRef = useRef({
    mutation: 0,
    requests: 0,
    review: 0,
  });

  const normalizedRequests = useMemo(
    () => requests.map(normalizeRequest),
    [requests],
  );
  const selectedRequest = normalizedRequests.find(
    (request) => request.id === selectedId,
  );

  const loadReview = async (requestId) => {
    const reviewRequestId = ++requestIdsRef.current.review;
    if (!requestId || !mountedRef.current) return;
    setLoadingReview(true);
    setError('');
    setErrorContext('load');

    try {
      const nextReview = normalizeReview(
        await quarantineJuryApi.getReview(requestId),
      );
      if (
        mountedRef.current &&
        reviewRequestId === requestIdsRef.current.review &&
        selectedIdRef.current === requestId
      ) {
        setReview(nextReview);
        setRouteForm((current) => ({
          ...current,
          targetJurors: nextReview.request.jurors.join(', '),
        }));
      }
    } catch (loadError) {
      if (
        mountedRef.current &&
        reviewRequestId === requestIdsRef.current.review &&
        selectedIdRef.current === requestId
      ) {
        setError(toDisplayError(loadError, 'Unable to load Quarantine Jury review'));
      }
    } finally {
      if (
        mountedRef.current &&
        reviewRequestId === requestIdsRef.current.review
      ) {
        setLoadingReview(false);
      }
    }
  };

  const loadRequests = async () => {
    const requestId = ++requestIdsRef.current.requests;
    if (!mountedRef.current) return;
    setLoadingRequests(true);
    setError('');
    setErrorContext('load');

    try {
      const response = await quarantineJuryApi.getRequests();
      const nextRequests = asArray(response);
      if (
        !mountedRef.current ||
        requestId !== requestIdsRef.current.requests
      ) {
        return;
      }
      setRequests(nextRequests);
      const normalized = nextRequests.map(normalizeRequest);
      const nextSelected =
        selectedId && normalized.some((request) => request.id === selectedId)
          ? selectedId
          : normalized[0]?.id || '';
      if (nextSelected !== selectedIdRef.current) {
        setReview(null);
      }
      selectedIdRef.current = nextSelected;
      setSelectedId(nextSelected);
      if (nextSelected) {
        await loadReview(nextSelected);
      } else if (
        mountedRef.current &&
        requestId === requestIdsRef.current.requests
      ) {
        setReview(null);
      }
    } catch (loadError) {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.requests
      ) {
        setError(toDisplayError(loadError, 'Unable to load Quarantine Jury requests'));
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.requests
      ) {
        setLoadingRequests(false);
      }
    }
  };

  useEffect(() => {
    void loadRequests();
    return () => {
      requestIdsRef.current.requests += 1;
      requestIdsRef.current.review += 1;
      requestIdsRef.current.mutation += 1;
    };
  }, []);

  const selectRequest = (requestId) => {
    if (!mountedRef.current) return;
    selectedIdRef.current = requestId;
    setSelectedId(requestId);
    setReview(null);
    setMessage('');
    void loadReview(requestId);
  };

  const submitRoute = async () => {
    if (!selectedId || !mountedRef.current || saving) return;
    const requestId = ++requestIdsRef.current.mutation;
    const requestToRoute = selectedId;
    setSaving(true);
    setError('');
    setErrorContext('action');

    try {
      await quarantineJuryApi.routeRequest(requestToRoute, {
        channelId: routeForm.channelId,
        podId: routeForm.podId,
        senderPeerId: routeForm.senderPeerId,
        targetJurors: parseJurors(routeForm.targetJurors),
      });
      if (
        !mountedRef.current ||
        requestId !== requestIdsRef.current.mutation ||
        selectedIdRef.current !== requestToRoute
      ) {
        return;
      }
      setMessage('Quarantine Jury route attempt recorded.');
      await loadReview(requestToRoute);
    } catch (routeError) {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.mutation
      ) {
        setError(toDisplayError(routeError, 'Unable to route Quarantine Jury request'));
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.mutation
      ) {
        setSaving(false);
      }
    }
  };

  const submitAccept = async () => {
    if (!selectedId || !mountedRef.current || saving) return;
    const requestId = ++requestIdsRef.current.mutation;
    const requestToAccept = selectedId;
    setSaving(true);
    setError('');
    setErrorContext('action');

    try {
      await quarantineJuryApi.acceptReleaseCandidate(requestToAccept, acceptForm);
      if (
        !mountedRef.current ||
        requestId !== requestIdsRef.current.mutation ||
        selectedIdRef.current !== requestToAccept
      ) {
        return;
      }
      setMessage('Release-candidate recommendation accepted for this review.');
      setAcceptOpen(false);
      await loadReview(requestToAccept);
    } catch (acceptError) {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.mutation
      ) {
        setError(
          toDisplayError(
            acceptError,
            'Unable to accept release-candidate recommendation',
          ),
        );
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.mutation
      ) {
        setSaving(false);
      }
    }
  };

  const aggregate = review?.aggregate || normalizeAggregate();
  const accepted = Boolean(review?.acceptance);

  return (
    <div className="quarantine-jury">
      <Segment>
        <div className="quarantine-jury-header">
          <Header as="h3">
            <Icon name="shield" />
            <Header.Content>
              Quarantine Jury
              <Header.Subheader>
                Review trusted juror evidence before local quarantine release decisions.
              </Header.Subheader>
            </Header.Content>
          </Header>
          <Popup
            content="Reload requests and the selected review without changing quarantine state."
            trigger={
              <Button
                aria-label="Refresh Quarantine Jury reviews"
                icon="refresh"
                loading={loadingRequests}
                onClick={loadRequests}
                type="button"
              />
            }
          />
        </div>
        <Statistic.Group
          className="quarantine-jury-summary"
          size="small"
          widths="four"
        >
          <Statistic>
            <Statistic.Value>{normalizedRequests.length}</Statistic.Value>
            <Statistic.Label>Requests</Statistic.Label>
          </Statistic>
          {/* Nothing to review yet isn't a warning state — only color these
              once a verdict has actually been cast. */}
          <Statistic
            color={
              aggregate.totalVerdicts === 0
                ? undefined
                : aggregate.quorumReached ? 'green' : 'orange'
            }
          >
            <Statistic.Value>{aggregate.totalVerdicts}</Statistic.Value>
            <Statistic.Label>Verdicts</Statistic.Label>
          </Statistic>
          <Statistic>
            <Statistic.Value>{aggregate.requiredVotes}</Statistic.Value>
            <Statistic.Label>Required</Statistic.Label>
          </Statistic>
          <Statistic color={aggregate.totalVerdicts === 0 ? undefined : verdictColor(aggregate.recommendation)}>
            <Statistic.Value>
              {aggregate.totalVerdicts === 0 ? 'No verdicts yet' : normalizeVerdict(aggregate.recommendation)}
            </Statistic.Value>
            <Statistic.Label>Recommendation</Statistic.Label>
          </Statistic>
        </Statistic.Group>
      </Segment>

      {error && (
        <Message
          data-testid={
            errorContext === 'load'
              ? 'quarantine-jury-load-error'
              : 'quarantine-jury-action-error'
          }
          error
          header={
            errorContext === 'load'
              ? 'Quarantine Jury load failed'
              : 'Quarantine Jury action failed'
          }
          content={String(error)}
        />
      )}
      {message && (
        <Message
          info
          header="Quarantine Jury updated"
          content={message}
        />
      )}

      <div className="quarantine-jury-layout">
        <Segment className="quarantine-jury-list">
          <Header as="h4">Requests</Header>
          {loadingRequests ? (
            <Loader active inline="centered" />
          ) : normalizedRequests.length === 0 ? (
            <Message
              info
              content="No Quarantine Jury requests are available."
            />
          ) : (
            <Card.Group itemsPerRow={1}>
              {normalizedRequests.map((request) => (
                <Card
                  className={
                    request.id === selectedId
                      ? 'quarantine-jury-request active'
                      : 'quarantine-jury-request'
                  }
                  key={request.id}
                  onClick={() => selectRequest(request.id)}
                >
                  <Card.Content>
                    <Card.Header>{request.id}</Card.Header>
                    <Card.Meta>{formatDate(request.createdAt)}</Card.Meta>
                    <Card.Description>{request.localReason || '-'}</Card.Description>
                  </Card.Content>
                  <Card.Content extra>
                    <Label basic>
                      <Icon name="users" />
                      {request.jurors.length} jurors
                    </Label>
                    <Label basic>
                      <Icon name="check" />
                      {request.minJurorVotes} votes
                    </Label>
                  </Card.Content>
                </Card>
              ))}
            </Card.Group>
          )}
        </Segment>

        <Segment className="quarantine-jury-detail">
          {loadingReview ? (
            <Loader active inline="centered" />
          ) : !review ? (
            <Message
              info
              content="Select a request to inspect jury evidence."
            />
          ) : (
            <>
              <div className="quarantine-jury-detail-head">
                <Header as="h4">
                  <Icon name="file alternate outline" />
                  <Header.Content>
                    Review {selectedRequest?.id}
                    <Header.Subheader>
                      {review.acceptanceReason || aggregate.reason}
                    </Header.Subheader>
                  </Header.Content>
                </Header>
                <Label color={verdictColor(aggregate.recommendation)}>
                  {normalizeVerdict(aggregate.recommendation)}
                </Label>
              </div>

              <Label.Group>
                <Label color={aggregate.quorumReached ? 'green' : 'orange'}>
                  {aggregate.quorumReached ? 'Quorum Reached' : 'Awaiting Quorum'}
                </Label>
                <Label basic>
                  Release {getVerdictCount(aggregate.verdictCounts, 'ReleaseCandidate', 2)}
                </Label>
                <Label basic>
                  Uphold {getVerdictCount(aggregate.verdictCounts, 'UpholdQuarantine', 1)}
                </Label>
                <Label basic>
                  Manual {getVerdictCount(aggregate.verdictCounts, 'NeedsManualReview', 0)}
                </Label>
              </Label.Group>

              <Header as="h5">Request Evidence</Header>
              <EvidenceTable evidence={review.request.evidence} />

              <Header as="h5">Juror Verdicts</Header>
              <Table
                compact
                size="small"
              >
                <Table.Header>
                  <Table.Row>
                    <Table.HeaderCell>Juror</Table.HeaderCell>
                    <Table.HeaderCell>Verdict</Table.HeaderCell>
                    <Table.HeaderCell>Reason</Table.HeaderCell>
                    <Table.HeaderCell>Evidence</Table.HeaderCell>
                  </Table.Row>
                </Table.Header>
                <Table.Body>
                  {review.verdicts.length === 0 ? (
                    <Table.Row>
                      <Table.Cell colSpan={4}>No juror verdicts yet.</Table.Cell>
                    </Table.Row>
                  ) : (
                    review.verdicts.map((verdict) => (
                      <Table.Row key={verdict.id}>
                        <Table.Cell>{verdict.juror || '-'}</Table.Cell>
                        <Table.Cell>
                          <Label color={verdictColor(verdict.verdict)}>
                            {normalizeVerdict(verdict.verdict)}
                          </Label>
                        </Table.Cell>
                        <Table.Cell>{verdict.reason || '-'}</Table.Cell>
                        <Table.Cell>{verdict.evidence.length}</Table.Cell>
                      </Table.Row>
                    ))
                  )}
                </Table.Body>
              </Table>

              {aggregate.dissentingJurors.length > 0 && (
                <Message
                  warning
                  header="Dissenting jurors"
                  content={aggregate.dissentingJurors.join(', ')}
                />
              )}

              <Header as="h5">Route Attempts</Header>
              <Table
                compact
                size="small"
              >
                <Table.Header>
                  <Table.Row>
                    <Table.HeaderCell>Created</Table.HeaderCell>
                    <Table.HeaderCell>Pod / Channel</Table.HeaderCell>
                    <Table.HeaderCell>Routed</Table.HeaderCell>
                    <Table.HeaderCell>Failed</Table.HeaderCell>
                    <Table.HeaderCell>Status</Table.HeaderCell>
                  </Table.Row>
                </Table.Header>
                <Table.Body>
                  {review.routeAttempts.length === 0 ? (
                    <Table.Row>
                      <Table.Cell colSpan={5}>No route attempts recorded.</Table.Cell>
                    </Table.Row>
                  ) : (
                    review.routeAttempts.map((attempt) => (
                      <Table.Row key={attempt.id}>
                        <Table.Cell>{formatDate(attempt.createdAt)}</Table.Cell>
                        <Table.Cell>
                          {attempt.podId || '-'} / {attempt.channelId || '-'}
                        </Table.Cell>
                        <Table.Cell>{valueOrDash(attempt.routedJurors)}</Table.Cell>
                        <Table.Cell>{valueOrDash(attempt.failedJurors)}</Table.Cell>
                        <Table.Cell>
                          <Label color={attempt.success ? 'green' : 'red'}>
                            {attempt.success ? 'Routed' : attempt.errorMessage || 'Failed'}
                          </Label>
                        </Table.Cell>
                      </Table.Row>
                    ))
                  )}
                </Table.Body>
              </Table>

              <Segment>
                <Header as="h5">Manual Route Dispatch</Header>
                <Form>
                  <Form.Group widths="equal">
                    <Form.Input
                      label="Sender Peer ID"
                      onChange={(_event, { value }) =>
                        setRouteForm((current) => ({
                          ...current,
                          senderPeerId: value,
                        }))
                      }
                      value={routeForm.senderPeerId}
                    />
                    <Form.Input
                      label="Pod ID"
                      onChange={(_event, { value }) =>
                        setRouteForm((current) => ({
                          ...current,
                          podId: value,
                        }))
                      }
                      value={routeForm.podId}
                    />
                    <Form.Input
                      label="Channel ID"
                      onChange={(_event, { value }) =>
                        setRouteForm((current) => ({
                          ...current,
                          channelId: value,
                        }))
                      }
                      value={routeForm.channelId}
                    />
                  </Form.Group>
                  <Form.Input
                    label="Target Jurors"
                    onChange={(_event, { value }) =>
                      setRouteForm((current) => ({
                        ...current,
                        targetJurors: value,
                      }))
                    }
                    value={routeForm.targetJurors}
                  />
                  <Popup
                    content="Route this request only to the explicit target jurors listed here. Raw files are not attached."
                    trigger={
                      <Button
                        disabled={saving}
                        icon
                        labelPosition="left"
                        loading={saving}
                        onClick={submitRoute}
                        type="button"
                      >
                        <Icon name="paper plane" />
                        Route to Jurors
                      </Button>
                    }
                  />
                </Form>
              </Segment>

              <Segment>
                <Header as="h5">Acceptance</Header>
                {accepted ? (
                  <Message
                    positive
                    header="Release candidate accepted"
                    content={`${normalizeAcceptance(review.acceptance).acceptedBy} accepted this recommendation at ${formatDate(normalizeAcceptance(review.acceptance).createdAt)}.`}
                  />
                ) : (
                  <>
                    <Message
                      info
                      content={review.acceptanceReason}
                    />
                    <Popup
                      content="Open the confirmation dialog. Accepting records a local decision only when the aggregate recommends Release Candidate."
                      trigger={
                        <Button
                          color="green"
                          disabled={!review.canAcceptReleaseCandidate || saving}
                          icon
                          labelPosition="left"
                          onClick={() => setAcceptOpen(true)}
                          type="button"
                        >
                          <Icon name="check circle" />
                          Accept Release Candidate
                        </Button>
                      }
                    />
                  </>
                )}
              </Segment>
            </>
          )}
        </Segment>
      </div>

      <Modal
        onClose={() => setAcceptOpen(false)}
        open={acceptOpen}
        size="small"
      >
        <Modal.Header>Accept Release Candidate</Modal.Header>
        <Modal.Content>
          <Message
            warning
            content="This records a local acceptance decision for the jury recommendation. It does not automatically move files or broadcast a release."
          />
          <Form>
            <Form.Input
              label="Accepted By"
              onChange={(_event, { value }) =>
                setAcceptForm((current) => ({
                  ...current,
                  acceptedBy: value,
                }))
              }
              value={acceptForm.acceptedBy}
            />
            <Form.TextArea
              label="Review Note"
              onChange={(_event, { value }) =>
                setAcceptForm((current) => ({
                  ...current,
                  note: value,
                }))
              }
              value={acceptForm.note}
            />
          </Form>
        </Modal.Content>
        <Modal.Actions>
          <Popup
            content="Close without recording an acceptance decision."
            trigger={
              <Button
                disabled={saving}
                onClick={() => setAcceptOpen(false)}
                type="button"
              >
                Cancel
              </Button>
            }
          />
          <Popup
            content="Record the local acceptance decision for this release-candidate recommendation."
            trigger={
              <Button
                color="green"
                disabled={saving}
                loading={saving}
                onClick={submitAccept}
                type="button"
              >
                Accept Recommendation
              </Button>
            }
          />
        </Modal.Actions>
      </Modal>
    </div>
  );
};

export default QuarantineJury;
