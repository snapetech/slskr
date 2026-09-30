import * as discoveryGraph from '../../lib/discoveryGraph';
import { toDisplayError } from '../../lib/errors';
import * as jobs from '../../lib/jobs';
import * as musicBrainz from '../../lib/musicBrainz';
import * as searches from '../../lib/searches';
import * as songId from '../../lib/songid';
import { useMountedRef } from '../../lib/useMountedRef';
import DiscoveryGraphModal from './DiscoveryGraphModal';
import SongIDAnalysisResults from './SongIDAnalysisResults';
import React, { useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import {
  Button,
  Divider,
  Form,
  Header,
  Input,
  Label,
  List,
  Popup,
  Progress,
  Grid,
  Segment,
} from 'semantic-ui-react';
import { v4 as uuidv4 } from 'uuid';
import { formatPercent, getRunTitle, getStatusColor } from './songIdPanelHelpers';

const SongIDPanel = ({ disabled }) => {
  const [source, setSource] = useState('');
  const [targetDirectory, setTargetDirectory] = useState('');
  const [loading, setLoading] = useState(false);
  const [run, setRun] = useState(null);
  const [runs, setRuns] = useState([]);
  const [graphLoading, setGraphLoading] = useState(false);
  const [graphOpen, setGraphOpen] = useState(false);
  const [graphData, setGraphData] = useState(null);
  const [graphRequest, setGraphRequest] = useState(null);
  const [actionLoading, setActionLoading] = useState(false);
  const [copyLoading, setCopyLoading] = useState(false);
  const mountedRef = useMountedRef();
  const analyzeRequestIdRef = useRef(0);
  const graphRequestIdRef = useRef(0);
  const currentRunIdRef = useRef(null);
  const actionInFlightRef = useRef(false);
  const copyInFlightRef = useRef(false);

  useEffect(() => {
    currentRunIdRef.current = run?.id || null;
  }, [run]);

  useEffect(() => {
    const connection = songId.createHub();
    let active = true;

    connection.on('LIST', (runs) => {
      if (active && Array.isArray(runs)) {
        setRuns(runs);
        if (runs.length > 0 && !currentRunIdRef.current) {
          setRun(runs[0]);
        }
      }
    });

    connection.on('CREATE', (nextRun) => {
      if (active && nextRun?.id) {
        setRuns((currentRuns) => {
          const existing = currentRuns.filter((item) => item.id !== nextRun.id);
          return [nextRun, ...existing].slice(0, 25);
        });
        if (nextRun.id === currentRunIdRef.current || !currentRunIdRef.current) {
          setRun(nextRun);
        }
      }
    });

    connection.on('UPDATE', (nextRun) => {
      if (active && nextRun?.id) {
        setRuns((currentRuns) => {
          const existing = currentRuns.filter((item) => item.id !== nextRun.id);
          return [nextRun, ...existing]
            .sort((left, right) => new Date(right.createdAt) - new Date(left.createdAt))
            .slice(0, 25);
        });
        if (nextRun.id === currentRunIdRef.current) {
          setRun(nextRun);
        }
      }
    });

    connection.start().catch((error) => {
      if (mountedRef.current && active) {
        console.warn('SongID hub connection failed', error);
      }
    });

    return () => {
      active = false;
      connection.stop().catch(() => {});
    };
  }, [mountedRef]);

  const handleAnalyze = async () => {
    if (!mountedRef.current || disabled || loading) return;
    const trimmed = source.trim();
    if (!trimmed) {
      toast.error('Provide a URL, server-side file path, or text query');
      return;
    }

    const requestId = ++analyzeRequestIdRef.current;
    const isCurrentRequest = () =>
      mountedRef.current && analyzeRequestIdRef.current === requestId;
    setLoading(true);
    try {
      const result = await songId.createRun(trimmed);
      if (!isCurrentRequest()) return;
      setRun(result);
      setRuns((currentRuns) => [result, ...currentRuns.filter((item) => item.id !== result.id)].slice(0, 25));
      toast.success('SongID analysis queued');
    } catch (error) {
      if (isCurrentRequest()) {
        console.error(error);
        toast.error(toDisplayError(error, 'SongID analysis failed'));
      }
    } finally {
      if (isCurrentRequest()) setLoading(false);
    }
  };

  const runAction = async (operation, failureMessage) => {
    if (!mountedRef.current || disabled || actionInFlightRef.current) {
      return false;
    }

    actionInFlightRef.current = true;
    setActionLoading(true);
    try {
      await operation();
      return true;
    } catch (error) {
      if (mountedRef.current) {
        console.error(error);
        toast.error(toDisplayError(error, failureMessage));
      }
      return false;
    } finally {
      actionInFlightRef.current = false;
      if (mountedRef.current) {
        setActionLoading(false);
      }
    }
  };

  const handleTrackSearch = async (candidate) => {
    const searchText = typeof candidate?.searchText === 'string'
      ? candidate.searchText.trim()
      : '';
    if (!searchText) {
      toast.error('This candidate has no searchable query');
      return false;
    }

    const artist = typeof candidate?.artist === 'string' ? candidate.artist : '';
    const title = typeof candidate?.title === 'string' ? candidate.title : searchText;
    return runAction(async () => {
      await searches.create({
        id: uuidv4(),
        searchText,
      });
      if (mountedRef.current) {
        toast.success(`Started search for ${artist} - ${title}`);
      }
    }, 'Failed to start song search');
  };

  const handleTrackSearchBatch = async (queries) => {
    const validQueries = Array.isArray(queries)
      ? queries
        .filter((query) => typeof query === 'string')
        .map((query) => query.trim())
        .filter(Boolean)
      : [];
    if (validQueries.length === 0) {
      toast.error('No candidate searches were available');
      return;
    }

    return runAction(async () => {
      const count = await searches.createBatch({ queries: validQueries });
      const startedCount = typeof count === 'number'
        ? count
        : typeof count?.count === 'number'
          ? count.count
          : validQueries.length;
      if (mountedRef.current) {
        toast.success(`Started ${startedCount} candidate searches`);
      }
    }, 'Failed to start candidate searches');
  };

  const handleMixSearch = async (mix) => {
    if (typeof mix?.searchText !== 'string') {
      toast.error('No mix queries were available');
      return;
    }

    const queries = mix.searchText
      .split('+')
      .map((value) => (value || '').trim())
      .filter(Boolean);
    if (queries.length === 0) {
      toast.error('No mix queries were available');
      return;
    }

    return runAction(async () => {
      const count = await searches.createBatch({ queries });
      const startedCount = typeof count === 'number'
        ? count
        : typeof count?.count === 'number'
          ? count.count
          : queries.length;
      const segmentCount = typeof mix.segmentCount === 'number'
        ? mix.segmentCount
        : queries.length;
      if (mountedRef.current) {
        toast.success(`Started ${startedCount} mix search(es) for ${segmentCount} segments`);
      }
    }, 'Failed to start mix searches');
  };

  const handleAlbumPrepare = async (candidate) => {
    if (!candidate?.releaseId) {
      toast.error('This album has no MusicBrainz release ID');
      return false;
    }

    return runAction(async () => {
      await musicBrainz.resolveTarget({ releaseId: candidate.releaseId });
      if (mountedRef.current) {
        toast.success(`Prepared album target for ${candidate.title || 'album'}`);
      }
    }, 'Failed to prepare album target');
  };

  const handleDiscography = async (candidate) => {
    if (!candidate?.artistId) {
      toast.error('This artist has no MusicBrainz artist ID');
      return false;
    }

    return runAction(async () => {
      const response = await jobs.createDiscographyJob({
        artistId: candidate.artistId,
        profile: candidate.recommendedProfile || 'CoreDiscography',
        targetDirectory: targetDirectory.trim(),
      });
      const jobId = response?.job_id || response?.jobId || 'queued';
      if (mountedRef.current) {
        toast.success(`Planned discography job ${jobId}`);
      }
    }, 'Failed to create discography job');
  };

  const handleMbReleaseJob = async (candidate) => {
    if (!candidate?.releaseId) {
      toast.error('This album has no MusicBrainz release ID');
      return false;
    }

    return runAction(async () => {
      const response = await jobs.createMbReleaseJob({
        mbReleaseId: candidate.releaseId,
        targetDir: targetDirectory.trim(),
      });
      const jobId = response?.job_id || response?.jobId || 'queued';
      if (mountedRef.current) {
        toast.success(`Planned album job ${jobId}`);
      }
    }, 'Failed to create album download job');
  };

  const copyForensicMatrix = async () => {
    if (!run?.id || !run?.forensicMatrix) {
      toast.error('No forensic matrix is available for this SongID run');
      return;
    }

    if (!navigator.clipboard?.writeText || copyInFlightRef.current) {
      toast.error('Clipboard access is unavailable');
      return;
    }

    copyInFlightRef.current = true;
    setCopyLoading(true);
    try {
      const matrix = await songId.getForensicMatrix(run.id);
      if (!mountedRef.current) return;
      await navigator.clipboard.writeText(JSON.stringify(matrix, null, 2) || '{}');
      if (mountedRef.current) {
        toast.success('SongID forensic matrix copied');
      }
    } catch (error) {
      if (mountedRef.current) {
        console.error(error);
        toast.error(toDisplayError(error, 'Failed to copy SongID forensic matrix'));
      }
    } finally {
      copyInFlightRef.current = false;
      if (mountedRef.current) {
        setCopyLoading(false);
      }
    }
  };

  const openDiscoveryGraph = async (request) => {
    if (!mountedRef.current || disabled || graphLoading) return;
    const requestId = ++graphRequestIdRef.current;
    const isCurrentRequest = () =>
      mountedRef.current && graphRequestIdRef.current === requestId;
    setGraphLoading(true);
    setGraphOpen(true);
    setGraphData(null);
    setGraphRequest(request);

    try {
      const graph = await discoveryGraph.buildDiscoveryGraph(request);
      if (!isCurrentRequest()) return;
      setGraphData(graph);
    } catch (error) {
      if (isCurrentRequest()) {
        console.error(error);
        toast.error(toDisplayError(error, 'Failed to build discovery graph'));
        setGraphOpen(false);
      }
    } finally {
      if (isCurrentRequest()) setGraphLoading(false);
    }
  };

  const handleGraphRecenter = async (nodeId) => {
    if (!nodeId) {
      return;
    }

    const [nodeType, rawId] = nodeId.split(':');
    const nextRequest = {
      songIdRunId: run?.id,
    };

    if (nodeType === 'track') {
      nextRequest.scope = 'track';
      nextRequest.recordingId = rawId;
    } else if (nodeType === 'album' || nodeType === 'release-group') {
      nextRequest.scope = 'album';
      nextRequest.releaseId = rawId;
    } else if (nodeType === 'artist') {
      nextRequest.scope = 'artist';
      nextRequest.artistId = rawId;
    } else {
      nextRequest.scope = graphRequest?.scope || 'songid_run';
    }

    await openDiscoveryGraph(nextRequest);
  };

  const handleGraphCompare = async (nodeId, label) => {
    if (!graphRequest || !nodeId) {
      return;
    }

    await openDiscoveryGraph({
      ...graphRequest,
      compareLabel: label,
      compareNodeId: nodeId,
    });
  };

  const handleQueueNearbyFromGraph = async (graph) => {
    const nodes = Array.isArray(graph?.nodes) ? graph.nodes : [];
    const queries = nodes
      .filter((node) => node.nodeType === 'track')
      .map((node) => {
        const recordingId = typeof node.nodeId === 'string'
          ? node.nodeId.split(':')[1]
          : '';
        const candidate = (Array.isArray(run?.tracks) ? run.tracks : [])
          .find((item) => item.recordingId === recordingId);
        if (candidate?.searchText) {
          return candidate.searchText;
        }

        return node.label || '';
      })
      .filter(Boolean)
      .slice(0, 8);

    await handleTrackSearchBatch(queries);
  };

  const handlePlanAction = async (plan) => {
    if (!plan || typeof plan !== 'object') {
      toast.error('SongID plan is unavailable');
      return false;
    }

    if (plan.kind === 'track') {
      const title = typeof plan.title === 'string' ? plan.title : '';
      const segments = title.split(' - ');
      await handleTrackSearch({
        artist: segments[0] || '',
        title: segments.slice(1).join(' - ') || title,
        searchText: plan.searchText,
      });
      return;
    }

    if (plan.kind === 'album') {
      await handleAlbumPrepare({ releaseId: plan.targetId, title: plan.title });
      return;
    }

    if (plan.kind === 'artist') {
      await handleDiscography({
        artistId: plan.targetId,
        recommendedProfile: plan.profile,
      });
    }
  };

  const handleOptionAction = async (option) => {
    if (!option || typeof option !== 'object') {
      toast.error('SongID option is unavailable');
      return false;
    }

    if (option.actionKind === 'track_search') {
      const title = typeof option.title === 'string' ? option.title : '';
      const segments = title.split(' - ');
      await handleTrackSearch({
        artist: segments[0] || '',
        title: segments.slice(1).join(' - ') || title,
        searchText: option.searchText,
      });
      return;
    }

    if (option.actionKind === 'track_search_batch') {
      await handleTrackSearchBatch(option.searchTexts);
      return;
    }

    if (option.actionKind === 'album_prepare') {
      await handleAlbumPrepare({ releaseId: option.targetId, title: option.title });
      return;
    }

    if (option.actionKind === 'mb_release_job') {
      await handleMbReleaseJob({ releaseId: option.targetId });
      return;
    }

    if (option.actionKind === 'discography_job') {
      await handleDiscography({
        artistId: option.targetId,
        recommendedProfile: option.profile,
      });
    }
  };

  return (
    <>
    <Segment
      className="songid-panel"
      raised
    >
      <Header as="h4">SongID</Header>
      <p style={{ marginTop: 0 }}>
        Identify a likely track, album, or artist from a YouTube URL, Spotify
        URL, server-side file path, or direct text query, then fan the result
        out into slskr actions.
      </p>
      <Form>
        <Form.Field>
          <Input
            disabled={disabled || loading}
            onChange={(event) => setSource(event.target.value)}
            placeholder="Paste a source URL, local server path, or text query"
            value={source}
          />
        </Form.Field>
        <Form.Field>
          <Input
            disabled={disabled || loading}
            onChange={(event) => setTargetDirectory(event.target.value)}
            placeholder="Optional target directory for album or discography jobs"
            value={targetDirectory}
          />
        </Form.Field>
        <Popup
          content="Analyze the source and rank likely track, album, and artist candidates so you can search or plan downloads from the result."
          position="top right"
          trigger={
            <Button
              disabled={disabled || loading}
              loading={loading}
              onClick={handleAnalyze}
              primary
            >
              Analyze with SongID
            </Button>
          }
        />
      </Form>

      <Grid stackable columns={2} style={{ marginTop: '1em' }}>
        <Grid.Column width={5}>
          <Segment
            className="songid-queue-panel"
            secondary
          >
            <Header as="h5" style={{ marginTop: 0 }}>
              Queue
            </Header>
            {runs.length === 0 ? (
              <p style={{ marginBottom: 0 }}>No SongID runs yet.</p>
            ) : (
              <List divided relaxed>
                {runs.map((item) => (
                  <List.Item key={item.id}>
                    <List.Content floated="right">
                      <Popup
                        content="Open this SongID run and follow its queue position, evidence, and download options."
                        position="top center"
                        trigger={
                          <Button
                            size="mini"
                            onClick={() => setRun(item)}
                            primary={run?.id === item.id}
                          >
                            View
                          </Button>
                        }
                      />
                      <Popup
                        content="Open the Discovery Graph centered on this SongID run so you can inspect its neighborhood without leaving the queue."
                        position="top center"
                        trigger={
                          <Button
                            onClick={() =>
                              openDiscoveryGraph({
                                scope: 'songid_run',
                                songIdRunId: item.id,
                                title: item.query || item.summary,
                              })
                            }
                            size="mini"
                            style={{ marginLeft: '0.5em' }}
                          >
                            Graph
                          </Button>
                        }
                      />
                    </List.Content>
                    <List.Content>
                      <List.Header>{getRunTitle(item)}</List.Header>
                      <List.Description>
                        <Label
                          color={getStatusColor(item.status)}
                          size="tiny"
                        >
                          {item.status || 'unknown'}
                        </Label>
                        <Label size="tiny">{item.sourceType || 'unknown'}</Label>
                        {item.percentComplete ? (
                          <Label size="tiny">
                            {formatPercent(item.percentComplete)}%
                          </Label>
                        ) : null}
                      </List.Description>
                      <List.Description style={{ marginTop: '0.35em' }}>
                        {item.queuePosition !== null &&
                        item.queuePosition !== undefined ? (
                          <Label size="tiny">Queue {item.queuePosition}</Label>
                        ) : null}
                        {item.workerSlot !== null &&
                        item.workerSlot !== undefined ? (
                          <Label size="tiny">Worker {item.workerSlot}</Label>
                        ) : null}
                        {item.currentStage ? (
                          <Label size="tiny">{item.currentStage}</Label>
                        ) : null}
                      </List.Description>
                      {item.summary ? (
                        <List.Description style={{ marginTop: '0.35em' }}>
                          {item.summary}
                        </List.Description>
                      ) : null}
                    </List.Content>
                  </List.Item>
                ))}
              </List>
            )}
          </Segment>
        </Grid.Column>
        <SongIDAnalysisResults
          actionLoading={actionLoading}
          copyForensicMatrix={copyForensicMatrix}
          copyLoading={copyLoading}
          disabled={disabled}
          graphData={graphData}
          graphRequest={graphRequest}
          handleAlbumPrepare={handleAlbumPrepare}
          handleDiscography={handleDiscography}
          handleGraphRecenter={handleGraphRecenter}
          handleMbReleaseJob={handleMbReleaseJob}
          handleMixSearch={handleMixSearch}
          handleOptionAction={handleOptionAction}
          handlePlanAction={handlePlanAction}
          handleTrackSearch={handleTrackSearch}
          handleTrackSearchBatch={handleTrackSearchBatch}
          loading={loading}
          openDiscoveryGraph={openDiscoveryGraph}
          run={run}
          source={source}
        />
      </Grid>
    </Segment>
    <DiscoveryGraphModal
      graph={graphData}
      loading={graphLoading}
      onCompare={handleGraphCompare}
      onClose={() => {
        graphRequestIdRef.current += 1;
        setGraphOpen(false);
      }}
      onQueueNearby={handleQueueNearbyFromGraph}
      onRecenter={handleGraphRecenter}
      onRestoreBranch={(branch) => branch?.request && openDiscoveryGraph(branch.request)}
      open={graphOpen}
    />
    </>
  );
};

export default SongIDPanel;
