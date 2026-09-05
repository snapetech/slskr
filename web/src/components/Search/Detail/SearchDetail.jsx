import {
  createBatch,
  blockUser,
  filterResponse,
  getBlockedUsers,
  getResponses,
  getUserDownloadStats,
  parseFiltersFromString,
  unblockUser,
} from '../../../lib/searches';
import {
  buildAlbumCandidates,
  getAlbumCandidateFilter,
} from '../../../lib/albumCandidatePicker';
import { saveAlbumDecisionRule } from '../../../lib/albumDecisionRules';
import { buildDiscoveryGraph } from '../../../lib/discoveryGraph';
import { rankSearchResponses } from '../../../lib/searchCandidateRanking';
import { deduplicateSearchResponses } from '../../../lib/searchResultDeduplication';
import { isSearchComplete } from '../../../lib/searchState';
import {
  getSavedSearchFilters,
  removeSavedSearchFilter,
  saveSearchFilter,
} from '../../../lib/savedSearchFilters';
import {
  getLocalStorageItem,
  removeLocalStorageItem,
  setLocalStorageItem,
} from '../../../lib/storage';
import { getAllNotes } from '../../../lib/userNotes';
import { getDirectoryName, sleep } from '../../../lib/util';
import { toDisplayError } from '../../../lib/errors';
import * as wishlistAPI from '../../../lib/wishlist';
import ErrorSegment from '../../Shared/ErrorSegment';
import LoaderSegment from '../../Shared/LoaderSegment';
import Switch from '../../Shared/Switch';
import DiscoveryGraphModal from '../DiscoveryGraphModal';
import Response from '../Response';
import SearchDetailHeader from './SearchDetailHeader';
import SearchFilterModal from './SearchFilterModal';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import {
  Button,
  Checkbox,
  Confirm,
  Dropdown,
  Header,
  Icon,
  Input,
  Label,
  List,
  Popup,
  Segment,
} from 'semantic-ui-react';

const sortDropdownOptions = [
  {
    key: 'smart',
    text: '⭐ Smart Ranking (Best Overall)',
    value: 'smart',
  },
  {
    key: 'uploadSpeed',
    text: 'Upload Speed (Fastest to Slowest)',
    value: 'uploadSpeed',
  },
  {
    key: 'queueLength',
    text: 'Queue Depth (Least to Most)',
    value: 'queueLength',
  },
  {
    key: 'fileCount',
    text: 'File Count (Most to Least)',
    value: 'fileCount',
  },
];

const asArray = (value) => (Array.isArray(value) ? value : []);

const normalizeWishlistDirectory = (value) =>
  String(value ?? '')
    .replaceAll('\\', '/')
    .trim()
    .replace(/^\/+|\/+$/gu, '')
    .toLowerCase();

// eslint-disable-next-line complexity
const SearchDetail = ({
  creating,
  disabled,
  onCreate,
  onRemove,
  onStop,
  removing,
  search,
  stopping,
}) => {
  const { fileCount, id, lockedFileCount, responseCount, state } = search;
  const isComplete = isSearchComplete(search);
  const searchText = search.searchText ?? search.query ?? '';
  const acquisitionProfile = search.acquisitionProfile || 'lossless-exact';

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(undefined);

  // `null` means the result endpoint has not hydrated yet.  An empty array is
  // a real terminal response and must be rendered as such rather than making
  // the detail page look blank while it is still loading.
  const [results, setResults] = useState(null);

  // filters and sorting options
  const [hiddenResults, setHiddenResults] = useState([]);
  const [ignoredResults, setIgnoredResults] = useState([]);
  const [ignoreRequest, setIgnoreRequest] = useState(null);
  const [blockedUsers, setBlockedUsers] = useState(getBlockedUsers());
  const [hideBlockedUsers, setHideBlockedUsers] = useState(true);
  const [resultSort, setResultSort] = useState('smart');
  const [hideLocked, setHideLocked] = useState(true);
  const [hideNoFreeSlots, setHideNoFreeSlots] = useState(false);
  const [foldResults, setFoldResults] = useState(false);
  const [foldDuplicateResults, setFoldDuplicateResults] = useState(
    getLocalStorageItem('slskr-search-fold-duplicate-results', 'true') !==
      'false',
  );
  const [resultFilters, setResultFilters] = useState(
    getLocalStorageItem('slskr-default-search-filter', ''),
  );
  const [savedFilters, setSavedFilters] = useState(getSavedSearchFilters());
  const [pageSize, setPageSize] = useState(
    Number.parseInt(getLocalStorageItem('slskr-search-page-size', '25'), 10),
  );
  const [displayCount, setDisplayCount] = useState(pageSize);
  const [userStats, setUserStats] = useState({});
  const [userNotes, setUserNotes] = useState({});
  const [qualitySignalVersion, setQualitySignalVersion] = useState(0);
  const [graphData, setGraphData] = useState(null);
  const [graphLoading, setGraphLoading] = useState(false);
  const [graphOpen, setGraphOpen] = useState(false);
  const [graphRequest, setGraphRequest] = useState(null);
  const mountedRef = useMountedRef();
  const requestIdsRef = useRef({
    graph: 0,
    ignore: 0,
    notes: 0,
    stats: 0,
  });

  const fetchUserNotes = useCallback(async () => {
    const requestId = ++requestIdsRef.current.notes;
    try {
      const response = await getAllNotes();
      const notes = Array.isArray(response.data) ? response.data : [];
      const notesMap = notes.reduce((accumulator, note) => {
        accumulator[note.username] = note;
        return accumulator;
      }, {});
      if (
        !mountedRef.current ||
        requestId !== requestIdsRef.current.notes
      ) {
        return;
      }
      setUserNotes(notesMap);
    } catch (error_) {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.notes
      ) {
        console.error('Failed to fetch user notes', error_);
      }
    }
  }, [mountedRef]);

  useEffect(() => {
    void fetchUserNotes();
  }, [fetchUserNotes]);

  useEffect(() => {
    let cancelled = false;
    const requestId = ++requestIdsRef.current.ignore;

    const loadIgnoredResults = async () => {
      if (!search.wishlistItemId) {
        setIgnoredResults([]);
        return;
      }

      try {
        const rules = await wishlistAPI.getIgnoredResults(search.wishlistItemId);
        if (
          !cancelled &&
          mountedRef.current &&
          requestId === requestIdsRef.current.ignore
        ) {
          setIgnoredResults(asArray(rules));
        }
      } catch (error_) {
        if (
          !cancelled &&
          mountedRef.current &&
          requestId === requestIdsRef.current.ignore
        ) {
          console.error(error_);
          toast.error(
            toDisplayError(error_, 'Failed to load ignored wishlist results'),
          );
        }
      }
    };

    void loadIgnoredResults();
    return () => {
      cancelled = true;
      requestIdsRef.current.ignore += 1;
    };
  }, [mountedRef, search.wishlistItemId]);

  const isIgnoredWishlistFile = useCallback(
    (username, filename) => {
      const directory = normalizeWishlistDirectory(getDirectoryName(filename));
      const normalizedUsername = String(username ?? '').toLowerCase();

      return ignoredResults.some(
        (rule) =>
          String(rule?.username ?? '').toLowerCase() === normalizedUsername &&
          normalizeWishlistDirectory(rule?.directory) === directory,
      );
    },
    [ignoredResults],
  );

  const handleIgnoreWishlistDirectory = async () => {
    const request = ignoreRequest;
    const wishlistItemId = search.wishlistItemId;
    const requestId = ++requestIdsRef.current.ignore;
    setIgnoreRequest(null);

    if (!request || !wishlistItemId) {
      return;
    }

    try {
      const rule = await wishlistAPI.ignoreResult(wishlistItemId, request);
      if (
        rule &&
        mountedRef.current &&
        requestId === requestIdsRef.current.ignore &&
        search.wishlistItemId === wishlistItemId
      ) {
        setIgnoredResults((current) => [
          rule,
          ...current.filter((candidate) => candidate.id !== rule.id),
        ]);
      }
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.ignore &&
        search.wishlistItemId === wishlistItemId
      ) {
        toast.info(
          `Ignored ${request.directory} from ${request.username} for this wishlist item`,
        );
      }
    } catch (error_) {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.ignore &&
        search.wishlistItemId === wishlistItemId
      ) {
        console.error(error_);
        toast.error(toDisplayError(error_, 'Failed to ignore wishlist folder'));
      }
    }
  };

  const [hasSavedDefault, setHasSavedDefault] = useState(
    Boolean(getLocalStorageItem('slskr-default-search-filter')),
  );

  // Sync hasSavedDefault across tabs/searches when localStorage changes
  useEffect(() => {
    const handleStorageChange = (event) => {
      if (event.key === 'slskr-default-search-filter') {
        setHasSavedDefault(Boolean(event.newValue));
      }
    };

    window.addEventListener('storage', handleStorageChange);
    return () => window.removeEventListener('storage', handleStorageChange);
  }, []);

  // Fetch user download stats for smart ranking
  useEffect(() => {
    let cancelled = false;
    const requestId = ++requestIdsRef.current.stats;

    const fetchStats = async () => {
      try {
        const stats = await getUserDownloadStats();
        if (
          !cancelled &&
          mountedRef.current &&
          requestId === requestIdsRef.current.stats
        ) {
          setUserStats(stats && typeof stats === 'object' ? stats : {});
        }
      } catch {
        // Stats are optional, don't fail if unavailable
      }
    };

    void fetchStats();
    return () => {
      cancelled = true;
      requestIdsRef.current.stats += 1;
    };
  }, [mountedRef]);

  // Handle blocking/unblocking users
  const handleBlockUser = useCallback((username) => {
    const updated = blockUser(username);
    setBlockedUsers(updated);
    toast.info(`Blocked ${username} from search results`);
  }, []);

  const handleUnblockUser = useCallback((username) => {
    const updated = unblockUser(username);
    setBlockedUsers(updated);
    toast.info(`Unblocked ${username}`);
  }, []);

  // Fetch results once counts appear. Mesh responses can now arrive before
  // the Soulseek search reaches its timeout.
  useEffect(() => {
    const hasResults = responseCount > 0 || fileCount > 0 || lockedFileCount > 0;

    if (!isComplete && !hasResults) {
      return undefined;
    }

    let cancelled = false;

    const get = async () => {
      try {
        setLoading(true);

        // Search completion and result persistence are separate events. Retry
        // a bounded number of times when the completed projection already
        // reports results but the response rows have not caught up yet.
        const attempts = hasResults ? 3 : 1;
        for (let attempt = 0; attempt < attempts; attempt += 1) {
          if (cancelled) {
            return;
          }
          if (attempt > 0) {
            await sleep(isComplete && attempt === 1 ? 500 : 250);
          }

          const responses = asArray(await getResponses({ id })).filter(
            (response) =>
              response &&
              typeof response === 'object' &&
              !Array.isArray(response),
          );
          if (responses.length > 0 || attempt === attempts - 1) {
            if (!cancelled) {
              setError(undefined);
              setResults(responses);
              setLoading(false);
            }
            return;
          }
        }
      } catch (getError) {
        if (!cancelled) {
          setError(toDisplayError(getError, 'Failed to load search results'));
          setLoading(false);
        }
      }
    };

    const timeout = setTimeout(get, isComplete ? 0 : 250);
    return () => {
      cancelled = true;
      clearTimeout(timeout);
    };
  }, [fileCount, id, isComplete, lockedFileCount, responseCount]);

  // apply sorting and filters.  this can take a while for larger result
  // sets, so memoize it.
  const rankedAndFilteredResults = useMemo(() => {
    const sortOptions = {
      fileCount: { field: 'fileCount', order: 'desc' },
      queueLength: { field: 'queueLength', order: 'asc' },
      smart: { field: 'smartScore', order: 'desc' },
      uploadSpeed: { field: 'uploadSpeed', order: 'desc' },
    };

    const { field, order } = sortOptions[resultSort] ?? sortOptions.smart;

    const filters = parseFiltersFromString(resultFilters);

    return asArray(results)
      .filter((r) => !hiddenResults.includes(r.username))
      .filter((r) => !(hideBlockedUsers && blockedUsers.includes(r.username)))
      .map((response) => {
        if (!search.wishlistItemId || ignoredResults.length === 0) {
          return response;
        }

        const files = asArray(response.files).filter(
          (file) => !isIgnoredWishlistFile(response.username, file.filename),
        );
        const lockedFiles = asArray(response.lockedFiles).filter(
          (file) => !isIgnoredWishlistFile(response.username, file.filename),
        );

        return {
          ...response,
          fileCount: files.length,
          files,
          lockedFileCount: lockedFiles.length,
          lockedFiles,
        };
      })
      .map((r) => {
        if (hideLocked) {
          return { ...r, lockedFileCount: 0, lockedFiles: [] };
        }

        return r;
      })
      .map((response) => filterResponse({ filters, response }))
      .filter((r) => r.fileCount + r.lockedFileCount > 0)
      .filter((r) => !(hideNoFreeSlots && !r.hasFreeUploadSlot))
      .map((r) =>
        rankSearchResponses({
          acquisitionProfile,
          preferredConditions: filters,
          responses: [r],
          searchText,
          userStats,
        })[0],
      )
      .sort((a, b) => {
        const left = a[field] ?? 0;
        const right = b[field] ?? 0;

        if (order === 'asc') {
          return left - right;
        }

        return right - left;
      });
  }, [
    acquisitionProfile,
    blockedUsers,
    hiddenResults,
    hideBlockedUsers,
    hideLocked,
    hideNoFreeSlots,
    resultFilters,
    resultSort,
    results,
    searchText,
    search.wishlistItemId,
    userStats,
    qualitySignalVersion,
    ignoredResults,
    isIgnoredWishlistFile,
  ]);

  const deduplicatedResults = useMemo(
    () =>
      deduplicateSearchResponses({
        enabled: foldDuplicateResults,
        responses: rankedAndFilteredResults,
      }),
    [foldDuplicateResults, rankedAndFilteredResults],
  );

  const sortedAndFilteredResults = deduplicatedResults.responses;

  const albumCandidates = useMemo(
    () =>
      buildAlbumCandidates({
        responses: sortedAndFilteredResults,
        searchText,
      }),
    [searchText, sortedAndFilteredResults],
  );

  // when a user uses the action buttons, we will *probably* re-use this component,
  // but with a new search ID.  clear everything to prepare for the transition
  const reset = () => {
    setLoading(false);
    setError(undefined);
    setResults(null);
    setHiddenResults([]);
    setDisplayCount(pageSize);
  };

  const handlePageSizeChange = (newSize) => {
    setPageSize(newSize);
    setLocalStorageItem('slskr-search-page-size', newSize);
    // If we're showing less than the new page size, expand to fill it
    if (displayCount < newSize) {
      setDisplayCount(newSize);
    }
  };

  const handleFoldDuplicateResultsChange = () => {
    const nextValue = !foldDuplicateResults;
    setFoldDuplicateResults(nextValue);
    setLocalStorageItem(
      'slskr-search-fold-duplicate-results',
      String(nextValue),
    );
  };

  const create = async ({ navigate, search: searchForCreate }) => {
    reset();
    onCreate({ navigate, search: searchForCreate });
  };

  const openDiscoveryGraph = async (request) => {
    if (!mountedRef.current) return;
    const requestId = ++requestIdsRef.current.graph;
    setGraphLoading(true);
    setGraphOpen(true);
    setGraphRequest(request);
    setGraphData(null);

    try {
      const graph = await buildDiscoveryGraph(request);
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.graph
      ) {
        setGraphData(graph);
      }
    } catch (error_) {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.graph
      ) {
        console.error(error_);
        toast.error(toDisplayError(error_, 'Failed to build discovery graph'));
        setGraphOpen(false);
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === requestIdsRef.current.graph
      ) {
        setGraphLoading(false);
      }
    }
  };

  const closeDiscoveryGraph = useCallback(() => {
    requestIdsRef.current.graph += 1;
    setGraphOpen(false);
    setGraphLoading(false);
  }, []);

  const openSearchGraph = async () => {
    await openDiscoveryGraph({
      artist: searchText,
      scope: 'songid_run',
      title: searchText,
    });
  };

  const handleGraphRecenter = async (nodeId) => {
    if (typeof nodeId !== 'string' || nodeId.length === 0) {
      return;
    }

    const [nodeType, rawId] = nodeId.split(':');

    if (nodeType === 'artist') {
      await openDiscoveryGraph({ scope: 'artist', artistId: rawId });
      return;
    }

    if (nodeType === 'album' || nodeType === 'release-group') {
      await openDiscoveryGraph({ scope: 'album', releaseId: rawId });
      return;
    }

    if (nodeType === 'track') {
      await openDiscoveryGraph({ scope: 'track', recordingId: rawId });
      return;
    }

    await openSearchGraph();
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

  const handleQueueNearby = async (graph) => {
    if (!mountedRef.current) return;
    const queries = asArray(graph?.nodes)
      .filter((node) => node.nodeType === 'track')
      .map((node) => node.label || '')
      .filter(Boolean)
      .slice(0, 8);

    if (queries.length === 0) {
      toast.error('No nearby track nodes were available to queue');
      return;
    }

    try {
      const count = await createBatch({ queries });
      if (mountedRef.current) {
        toast.success(`Started ${count} nearby graph searches`);
      }
    } catch (error_) {
      if (mountedRef.current) {
        console.error(error_);
        toast.error(
          toDisplayError(error_, 'Failed to queue nearby graph searches'),
        );
      }
    }
  };

  const remove = async () => {
    reset();
    onRemove(search);
  };

  const saveAsDefault = () => {
    setLocalStorageItem('slskr-default-search-filter', resultFilters);
    setHasSavedDefault(true);
    toast.success('Search filters saved as default');
  };

  const clearSavedDefault = () => {
    removeLocalStorageItem('slskr-default-search-filter');
    setHasSavedDefault(false);
    toast.info('Saved default filter cleared');
  };

  const saveNamedFilter = () => {
    const name = window.prompt('Filter name', searchText || 'Search filter');
    const next = saveSearchFilter({ name, value: resultFilters });
    setSavedFilters(next);

    if (name?.trim() && resultFilters.trim()) {
      toast.success('Search filter saved');
    }
  };

  const loadNamedFilter = (name) => {
    const filter = savedFilters.find((item) => item.name === name);
    if (!filter) {
      return;
    }

    setResultFilters(filter.value);
  };

  const deleteNamedFilter = () => {
    const current = savedFilters.find((filter) => filter.value === resultFilters);
    if (!current) {
      return;
    }

    setSavedFilters(removeSavedSearchFilter(current.name));
    toast.info('Saved search filter removed');
  };

  const focusAlbumCandidate = (candidate) => {
    const filter = getAlbumCandidateFilter(candidate);
    if (!filter) {
      return;
    }

    if (resultFilters.toLowerCase().includes(filter)) {
      return;
    }

    setResultFilters(`${resultFilters} ${filter}`.trim());
  };

  const saveAlbumCandidateRule = (candidate) => {
    const { rule } = saveAlbumDecisionRule({
      candidate,
      searchText,
    });

    toast.success(`Saved local album rule for ${rule.albumTitle}`);
  };

  const resultCount = Array.isArray(results) ? results.length : 0;
  const filteredCount = Math.max(
    0,
    resultCount - sortedAndFilteredResults.length,
  );
  const remainingCount = sortedAndFilteredResults.length - displayCount;
  const loaded = !removing && !creating && !loading && results !== null;

  if (error && results === null) {
    return <ErrorSegment caption={toDisplayError(error, 'Failed to load search results')} />;
  }

  return (
    <>
      {error && (
        <div data-testid="search-results-load-error">
          <ErrorSegment caption={toDisplayError(error, 'Failed to load search results')} />
        </div>
      )}
      <SearchDetailHeader
        creating={creating}
        disabled={disabled}
        loaded={loaded}
        loading={loading}
        onCreate={create}
        onOpenGraph={openSearchGraph}
        onRemove={remove}
        onStop={onStop}
        removing={removing}
        search={search}
        stopping={stopping}
      />
      <Switch
        loading={loading && <LoaderSegment />}
        searching={
          !isComplete &&
          (results === null || results.length === 0) && (
            <LoaderSegment>
              {state === 'InProgress'
                ? `Found ${fileCount} files ${
                    lockedFileCount > 0
                      ? `(plus ${lockedFileCount} locked) `
                      : ''
                  }from ${responseCount} users`
                : 'Loading results...'}
            </LoaderSegment>
          )
        }
      >
        <DiscoveryGraphModal
          graph={graphData}
          loading={graphLoading}
          onClose={closeDiscoveryGraph}
          onCompare={handleGraphCompare}
          onQueueNearby={handleQueueNearby}
          onRecenter={handleGraphRecenter}
          onRestoreBranch={(branch) => branch?.request && openDiscoveryGraph(branch.request)}
          open={graphOpen}
        />
        {loaded && (
          <Segment
            className="search-options"
            raised
          >
            <Dropdown
              button
              className="search-options-sort icon"
              floating
              icon="sort"
              labeled
              onChange={(_event, { value }) => setResultSort(value)}
              options={sortDropdownOptions}
              text={
                (sortDropdownOptions.find((o) => o.value === resultSort) ??
                  sortDropdownOptions[0]).text
              }
            />
            <Dropdown
              button
              className="search-options-pagesize"
              floating
              onChange={(_event, { value }) => handlePageSizeChange(value)}
              options={[
                { key: '10', text: '10 per page', value: 10 },
                { key: '25', text: '25 per page', value: 25 },
                { key: '50', text: '50 per page', value: 50 },
                { key: '100', text: '100 per page', value: 100 },
                { key: 'all', text: 'Show All', value: 999_999 },
              ]}
              style={{ marginLeft: '0.5em' }}
              text={pageSize >= 999_999 ? 'Show All' : `${pageSize} per page`}
            />
            <div className="search-option-toggles">
              <Checkbox
                checked={hideLocked}
                className="search-options-hide-locked"
                label="Hide Locked Results"
                onChange={() => setHideLocked(!hideLocked)}
                toggle
              />
              <Checkbox
                checked={hideNoFreeSlots}
                className="search-options-hide-no-slots"
                label="Hide Results with No Free Slots"
                onChange={() => setHideNoFreeSlots(!hideNoFreeSlots)}
                toggle
              />
              <Checkbox
                checked={hideBlockedUsers}
                className="search-options-hide-blocked"
                label={`Hide Blocked Users (${blockedUsers.length})`}
                onChange={() => setHideBlockedUsers(!hideBlockedUsers)}
                toggle
              />
              <Checkbox
                checked={foldResults}
                className="search-options-fold-results"
                label="Fold Results"
                onChange={() => setFoldResults(!foldResults)}
                toggle
              />
              <Popup
                content="Fold duplicate file candidates that appear from multiple providers or peers, keeping the highest-ranked visible result and showing the folded sources as metadata."
                position="top center"
                trigger={
                  <Checkbox
                    checked={foldDuplicateResults}
                    className="search-options-fold-duplicates"
                    label={`Fold Duplicates${
                      deduplicatedResults.foldedCount > 0
                        ? ` (${deduplicatedResults.foldedCount})`
                        : ''
                    }`}
                    onChange={handleFoldDuplicateResultsChange}
                    toggle
                  />
                }
              />
            </div>
            <div
              className="search-wishlist-ignore-guidance"
              role="note"
            >
              <Icon name={search.wishlistItemId ? 'ban' : 'info circle'} />
              {search.wishlistItemId
                ? 'Wishlist search: use “Ignore for Wishlist” below a result folder to hide that peer and folder from future runs of this wishlist item.'
                : 'Folder ignores are available only for wishlist searches because each rule belongs to one wishlist item. Open a result from Wishlist history to use them.'}
            </div>
            <Input
              action={
                <Button.Group>
                  {savedFilters.length > 0 && (
                    <Dropdown
                      button
                      className="icon"
                      floating
                      icon="bookmark"
                      onChange={(_event, { value }) => loadNamedFilter(value)}
                      options={savedFilters.map((filter) => ({
                        key: filter.name,
                        text: filter.name,
                        value: filter.name,
                      }))}
                      title="Load saved filter"
                    />
                  )}
                  {Boolean(resultFilters) && (
                    <Button
                      color="red"
                      icon="x"
                      onClick={() => setResultFilters('')}
                      title="Clear current filter"
                    />
                  )}
                  {Boolean(resultFilters) && (
                    <Button
                      color="teal"
                      icon="bookmark"
                      onClick={saveNamedFilter}
                      title="Save named filter"
                    />
                  )}
                  {savedFilters.some((filter) => filter.value === resultFilters) && (
                    <Button
                      color="orange"
                      icon="minus circle"
                      onClick={deleteNamedFilter}
                      title="Delete matching saved filter"
                    />
                  )}
                  <Button
                    color="blue"
                    icon="save"
                    onClick={saveAsDefault}
                    title="Save as default filter"
                  />
                  {hasSavedDefault && (
                    <Button
                      color="orange"
                      icon="trash"
                      onClick={clearSavedDefault}
                      title="Clear saved default filter"
                    />
                  )}
                  <SearchFilterModal
                    filterString={resultFilters}
                    onChange={setResultFilters}
                    trigger={
                      <Button
                        icon
                        title="Advanced Filters"
                      >
                        <Icon name="sliders horizontal" />
                      </Button>
                    }
                  />
                </Button.Group>
              }
              className="search-filter"
              label={{ content: 'Filter', icon: 'filter' }}
              onChange={(_event, data) => setResultFilters(data.value)}
              placeholder="
                lackluster container -bothersome iscbr|isvbr islossless|islossy 
                minbr:320 minfilesize:100mb maxfilesize:2gb minfilesinfolder:8 minlength:5000
              "
              value={resultFilters}
            />
          </Segment>
        )}
        {loaded && albumCandidates.length > 0 && (
          <Segment
            className="search-album-picker-segment"
            raised
          >
            <Header as="h4">
              Album candidates
              <Label
                color="blue"
                size="mini"
              >
                {albumCandidates.length}
              </Label>
            </Header>
            <List
              className="search-album-candidate-list"
              divided
              relaxed
            >
              {albumCandidates.map((candidate) => (
                <List.Item
                  className="search-album-candidate"
                  key={candidate.key}
                >
                  <List.Content floated="right">
                    <Popup
                      content="Save this visible album review as a browser-local rule preview for similar future searches. This does not alter download behavior or contact peers."
                      position="top center"
                      trigger={
                        <Button
                          aria-label={`Save album rule ${candidate.albumTitle}`}
                          icon="bookmark outline"
                          onClick={() => saveAlbumCandidateRule(candidate)}
                          size="mini"
                        />
                      }
                    />
                    <Popup
                      content="Focus the current result filter on this album folder name without starting another search or download."
                      position="top center"
                      trigger={
                        <Button
                          aria-label={`Focus album candidate ${candidate.albumTitle}`}
                          icon="filter"
                          onClick={() => focusAlbumCandidate(candidate)}
                          size="mini"
                        />
                      }
                    />
                  </List.Content>
                  <List.Content>
                    <List.Header>
                      {candidate.albumTitle}
                      <Label
                        color="purple"
                        size="tiny"
                      >
                        {candidate.score}/100
                      </Label>
                    </List.Header>
                    <List.Description>
                      {candidate.trackCount}/{candidate.expectedTrackCount}{' '}
                      visible tracks · {candidate.sourceCount} source
                      {candidate.sourceCount === 1 ? '' : 's'} ·{' '}
                      {Math.round(candidate.completenessRatio * 100)}%
                    </List.Description>
                    <div className="search-album-candidate-review">
                      <span>
                        Formats:{' '}
                        {candidate.formatMix
                          .map((item) => `${item.format} ${item.count}`)
                          .join(', ')}
                      </span>
                      {candidate.missingTrackNumbers.length > 0 && (
                        <span>
                          Missing:{' '}
                          {candidate.missingTrackNumbers.slice(0, 8).join(', ')}
                        </span>
                      )}
                      {candidate.durationVarianceSeconds > 0 && (
                        <span>
                          Duration spread:{' '}
                          {Math.round(candidate.durationVarianceSeconds / 60)}m
                        </span>
                      )}
                      {candidate.substitutionOptions.length > 0 && (
                        <span>
                          Substitutions:{' '}
                          {candidate.substitutionOptions
                            .map(
                              (option) =>
                                `track ${option.trackNumber} (${option.optionCount})`,
                            )
                            .join(', ')}
                        </span>
                      )}
                    </div>
                    {candidate.substitutionOptions.length > 0 && (
                      <div className="search-album-candidate-substitutions">
                        {candidate.substitutionOptions.slice(0, 4).map((option) => (
                          <Popup
                            content={`Manual review options from ${option.sources.join(', ')} in ${option.formats.join(', ')}. This only describes visible alternatives; it does not select or download them.`}
                            key={option.trackNumber}
                            position="top center"
                            trigger={
                              <Label
                                color="teal"
                                size="tiny"
                              >
                                <Icon name="exchange" />
                                Track {option.trackNumber}: {option.optionCount}{' '}
                                options
                              </Label>
                            }
                          />
                        ))}
                      </div>
                    )}
                    <div className="search-album-candidate-labels">
                      {candidate.reasons.map((reason) => (
                        <Label
                          key={reason}
                          size="tiny"
                        >
                          {reason}
                        </Label>
                      ))}
                      {candidate.warnings.map((warning) => (
                        <Popup
                          content="This is a local confidence warning from visible search result metadata only; it does not reject the candidate or contact peers."
                          key={warning}
                          position="top center"
                          trigger={
                            <Label
                              color="yellow"
                              size="tiny"
                            >
                              <Icon name="warning sign" />
                              {warning}
                            </Label>
                          }
                        />
                      ))}
                    </div>
                    <div className="search-album-candidate-paths">
                      {candidate.directories.join(' | ')}
                    </div>
                  </List.Content>
                </List.Item>
              ))}
            </List>
          </Segment>
        )}
        {loaded && sortedAndFilteredResults.length === 0 && (
          <Segment
            className="search-empty-state"
            role="status"
          >
            {resultCount === 0
              ? 'No results were returned for this search.'
              : 'No results match the current filters.'}
          </Segment>
        )}
        {loaded &&
          sortedAndFilteredResults.slice(0, displayCount).map((r, index) => (
            <Response
              disabled={disabled}
              downloadStats={r.downloadStats}
              isBlocked={blockedUsers.includes(r.username)}
              isInitiallyFolded={foldResults}
              key={r.username}
              onBlock={() => handleBlockUser(r.username)}
              onHide={() => setHiddenResults([...hiddenResults, r.username])}
              onIgnoreDirectory={search.wishlistItemId
                ? (directory) =>
                    setIgnoreRequest({
                      directory,
                      username: r.username,
                    })
                : undefined}
              onNoteUpdate={fetchUserNotes}
              onQualitySignalUpdate={() =>
                setQualitySignalVersion((version) => version + 1)
              }
              onUnblock={() => handleUnblockUser(r.username)}
              response={r}
              responseIndex={index}
              searchId={id}
              candidateRank={r.candidateRank}
              userNote={userNotes[r.username]}
            />
          ))}
        <Confirm
          cancelButton="Keep Result"
          confirmButton="Ignore Folder"
          content={ignoreRequest
            ? `Hide “${ignoreRequest.directory}” from ${ignoreRequest.username} in every future run of this wishlist item? Other results from this user will remain visible.`
            : ''}
          header="Ignore Wishlist Folder"
          onCancel={() => setIgnoreRequest(null)}
          onConfirm={handleIgnoreWishlistDirectory}
          open={Boolean(ignoreRequest)}
          size="small"
        />
        {loaded &&
          (remainingCount > 0 ? (
            <Button
              className="showmore-button"
              fluid
              onClick={() => setDisplayCount(displayCount + pageSize)}
              primary
              size="large"
            >
              Show {remainingCount > pageSize ? pageSize : remainingCount} More
              Results{' '}
              {`(${remainingCount} remaining, ${filteredCount} hidden by filter(s))`}
            </Button>
          ) : filteredCount > 0 ? (
            <Button
              className="showmore-button"
              disabled
              fluid
              size="large"
            >{`All results shown. ${filteredCount} results hidden by filter(s)`}</Button>
          ) : (
            ''
          ))}
      </Switch>
    </>
  );
};

export default SearchDetail;
