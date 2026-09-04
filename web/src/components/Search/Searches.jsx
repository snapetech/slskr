import './Search.css';
import { toDisplayError } from '../../lib/errors';
import {
  acquisitionProfiles,
  getAcquisitionProfile,
  getStoredAcquisitionProfileId,
  setStoredAcquisitionProfileId,
} from '../../lib/acquisitionProfiles';
import { createSearchHubConnection } from '../../lib/hubFactory';
import { getCapabilities } from '../../lib/slskr';
import { mergeSearchRecords } from '../../lib/searchState';
import { getLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import { useMountedRef } from '../../lib/useMountedRef';
import * as library from '../../lib/searches';
import ErrorSegment from '../Shared/ErrorSegment';
import LoaderSegment from '../Shared/LoaderSegment';
import PlaceholderSegment from '../Shared/PlaceholderSegment';
import AlbumCompletionPanel from './AlbumCompletionPanel';
import ArtistReleaseRadarPanel from './ArtistReleaseRadarPanel';
import DiscographyCoveragePanel from './DiscographyCoveragePanel';
import DiscoveryGraphAtlasPanel from './DiscoveryGraphAtlasPanel';
import FederatedTasteRecommendationsPanel from './FederatedTasteRecommendationsPanel';
import SearchDetail from './Detail/SearchDetail';
import SearchList from './List/SearchList';
import MusicBrainzLookup from './MusicBrainzLookup';
import SongIDPanel from './SongIDPanel';
import SoulseekDiscoveryPanel from './SoulseekDiscoveryPanel';
import React, { useEffect, useRef, useState } from 'react';
import {
  useLocation,
  useNavigate,
  useParams,
} from 'react-router-dom';
import { toast } from 'react-toastify';
import {
  Button,
  Checkbox,
  Dropdown,
  Header,
  Icon,
  Input,
  Popup,
  Segment,
} from 'semantic-ui-react';
import { v4 as uuidv4 } from 'uuid';

const CollapsibleSection = ({
  children,
  defaultOpen = true,
  storageKey,
  title,
}) => {
  const [open, setOpen] = useState(() => {
    if (!storageKey) {
      return defaultOpen;
    }

    const stored = getLocalStorageItem(storageKey);
    if (stored === null) {
      return defaultOpen;
    }

    return stored === 'open';
  });

  const toggleOpen = () => {
    setOpen((current) => {
      const next = !current;

      if (storageKey) {
        setLocalStorageItem(storageKey, next ? 'open' : 'closed');
      }

      return next;
    });
  };

  return (
    <Segment raised>
      <div
        style={{
          alignItems: 'center',
          display: 'flex',
          justifyContent: 'space-between',
          marginBottom: open ? '1em' : 0,
        }}
      >
        <Header
          as="h4"
          style={{ margin: 0 }}
        >
          {title}
        </Header>
        <Popup
          content={
            open
              ? `Collapse the ${title.toLowerCase()} panel to free up room on the page.`
              : `Expand the ${title.toLowerCase()} panel to inspect its contents.`
          }
          position="top center"
          trigger={
            <Button
              aria-label={`${open ? 'Collapse' : 'Expand'} ${title}`}
              icon
              onClick={toggleOpen}
              size="mini"
            >
              <Icon name={open ? 'angle up' : 'angle down'} />
            </Button>
          }
        />
      </div>
      {open ? children : null}
    </Segment>
  );
};

const searchEventIdentifier = (eventOrSearch) =>
  eventOrSearch?.resource ??
  eventOrSearch?.searchId ??
  eventOrSearch?.id ??
  eventOrSearch?.token;
const SEARCH_CREATE_HYDRATION_TIMEOUT_MS = 1_500;

const Searches = ({ runtimeProfile, server } = {}) => {
  const normalizedServer = server ?? { isConnected: false };
  const [connecting, setConnecting] = useState(true);
  const [error, setError] = useState(undefined);
  const [searches, setSearches] = useState({});
  const [initialSearchesLoaded, setInitialSearchesLoaded] = useState(false);

  const [removing, setRemoving] = useState(false);
  const [removingAll, setRemovingAll] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [creating, setCreating] = useState(false);

  // Scene ↔ Pod Bridging provider selection (opt-in; normal search stays Soulseek-compatible by default)
  const [scenePodBridgeEnabled, setScenePodBridgeEnabled] = useState(false);
  const [providerPod, setProviderPod] = useState(true);
  const [providerScene, setProviderScene] = useState(true); // Enabled by default when feature is on
  const [showProviderOptions, setShowProviderOptions] = useState(false);
  const [acquisitionProfileId, setAcquisitionProfileId] = useState(() =>
    getStoredAcquisitionProfileId(getLocalStorageItem),
  );

  const mountedRef = useMountedRef();
  const loadRequestIdRef = useRef(0);
  const createRequestIdRef = useRef(0);
  const removeRequestIdRef = useRef(0);
  const removeAllRequestIdRef = useRef(0);
  const stopRequestIdRef = useRef(0);
  const refreshRequestIdsRef = useRef(new Map());

  const inputRef = useRef();

  const location = useLocation();
  const routerNavigate = useNavigate();
  const { id: searchId } = useParams();
  const acquisitionProfile = getAcquisitionProfile(acquisitionProfileId);
  const acquisitionProfileOptions = acquisitionProfiles.map((profile) => ({
    content: (
      <div>
        <strong>{profile.label}</strong>
        <div className="search-acquisition-profile-option-summary">
          {profile.summary}
        </div>
      </div>
    ),
    icon: profile.icon,
    key: profile.id,
    text: profile.label,
    value: profile.id,
  }));

  const updateAcquisitionProfile = (event, { value }) => {
    setAcquisitionProfileId(
      setStoredAcquisitionProfileId(setLocalStorageItem, value).id,
    );
  };

  // Handle URL query parameters for predictable search URLs
  useEffect(() => {
    const urlParameters = new URLSearchParams(location.search);
    const queryParameter = urlParameters.get('q');

    if (queryParameter && !creating && !searchId) {
      // Automatically create a search from the URL query parameter
      create({
        navigate: false,
        search: queryParameter,
      }).then((id) => {
        if (!mountedRef.current) {
          return;
        }
        if (id) {
          routerNavigate(`/searches/${id}`, { replace: true });
          return;
        }

        routerNavigate('/searches', { replace: true });
      });
    }
  }, [location.search, creating, mountedRef, routerNavigate, searchId]); // eslint-disable-line react-hooks/exhaustive-deps

  const onConnecting = () => {
    if (!mountedRef.current) {
      return;
    }
    setConnecting(true);
  };

  const onConnected = () => {
    if (!mountedRef.current) {
      return;
    }
    setConnecting(false);
    setError(undefined);
  };

  const onConnectionError = (connectionError) => {
    if (!mountedRef.current) {
      return;
    }
    setConnecting(false);
    setError(toDisplayError(connectionError, 'Disconnected'));
  };

  const onUpdate = (update) => {
    if (!mountedRef.current) {
      return;
    }
    setSearches(update);
    onConnected();
  };

  useEffect(() => {
    let mounted = true;
    let restHydrated = false;

    const loadSearches = async () => {
      const requestId = ++loadRequestIdRef.current;
      try {
        const records = await library.getAll();
        if (
          !mounted ||
          !mountedRef.current ||
          requestId !== loadRequestIdRef.current
        ) {
          return;
        }
        const searchRecords = Array.isArray(records) ? records : [];
        onUpdate(
          searchRecords.reduce((accumulator, search) => {
            const id = searchEventIdentifier(search);
            if (id !== undefined && id !== null) {
              accumulator[id] = { ...search, id };
            }
            return accumulator;
          }, {}),
        );
      } catch (loadError) {
        if (
          !mounted ||
          !mountedRef.current ||
          requestId !== loadRequestIdRef.current
        ) {
          return;
        }
        onConnectionError(loadError?.message ?? 'Failed to load searches');
      } finally {
        restHydrated = true;
        if (
          mounted &&
          mountedRef.current &&
          requestId === loadRequestIdRef.current
        ) {
          setInitialSearchesLoaded(true);
        }
      }
    };

    const refreshSearch = async (eventOrSearch) => {
      const id = searchEventIdentifier(eventOrSearch);
      if (eventOrSearch?.searchText || eventOrSearch?.query) {
        if (id === undefined || id === null) {
          await loadSearches();
          return;
        }
        onUpdate((old) => ({
          ...old,
          [id]: { ...mergeSearchRecords(old[id], eventOrSearch), id },
        }));
        return;
      }

      if (id === undefined || id === null) {
        await loadSearches();
        return;
      }

      const requestId = (refreshRequestIdsRef.current.get(id) ?? 0) + 1;
      refreshRequestIdsRef.current.set(id, requestId);

      try {
        const search = await library.getStatus({ id });
        if (
          !mounted ||
          !mountedRef.current ||
          refreshRequestIdsRef.current.get(id) !== requestId
        ) {
          return;
        }
        const searchId = searchEventIdentifier(search) ?? id;
        onUpdate((old) => ({
          ...old,
          [searchId]: {
            ...mergeSearchRecords(old[searchId], search),
            id: searchId,
          },
        }));
      } catch (refreshError) {
        console.debug('failed to refresh search event payload', refreshError);
        if (
          mounted &&
          mountedRef.current &&
          refreshRequestIdsRef.current.get(id) === requestId
        ) {
          await loadSearches();
        }
      }
    };

    onConnecting();

    const searchHub = createSearchHubConnection();

    searchHub.on('list', (searchesEvent) => {
      if (!mountedRef.current || restHydrated) {
        return;
      }
      if (!Array.isArray(searchesEvent)) {
        void loadSearches();
        return;
      }
      onUpdate(
        searchesEvent.reduce((accumulator, search) => {
          const id = searchEventIdentifier(search);
          if (id !== undefined && id !== null) {
            accumulator[id] = { ...search, id };
          }
          return accumulator;
        }, {}),
      );
      onConnected();
    });

    searchHub.on('update', (search) => {
      void refreshSearch(search);
    });

    searchHub.on('delete', (search) => {
      const id = searchEventIdentifier(search);
      if (id === undefined || id === null) {
        return;
      }
      onUpdate((old) => {
        const next = { ...old };
        delete next[id];
        return next;
      });
    });

    searchHub.on('create', (search) => {
      void refreshSearch(search);
    });

    searchHub.onreconnecting((connectionError) =>
      onConnectionError(connectionError?.message ?? 'Disconnected'),
    );
    searchHub.onreconnected(() => onConnected());
    searchHub.onclose((connectionError) =>
      onConnectionError(connectionError?.message ?? 'Disconnected'),
    );

    const connect = async () => {
      try {
        onConnecting();
        await searchHub.start();
        if (mountedRef.current) {
          await loadSearches();
        }
      } catch (connectionError) {
        if (!mountedRef.current) {
          return;
        }
        toast.error(toDisplayError(connectionError, 'Failed to connect to search updates'));
        await loadSearches();
      }
    };

    void connect();

    // Scene ↔ Pod Bridging is opt-in. Do not infer it from generic capabilities,
    // otherwise ordinary searches silently leave the proven Soulseek path.
    const checkFeatureFlag = async () => {
      if (runtimeProfile === 'legacy') {
        return;
      }

      try {
        const capabilities = await getCapabilities();
        const enabled =
          capabilities?.feature?.scenePodBridge === true ||
          capabilities?.features?.includes('scene_pod_bridge') === true;
        if (mountedRef.current) {
          setScenePodBridgeEnabled(enabled);
        }
      } catch (error_) {
        // Feature flag check failed - assume disabled
        console.debug(
          'Scene ↔ Pod Bridging feature flag check failed:',
          error_,
        );
      }
    };

    checkFeatureFlag();

    return () => {
      mounted = false;
      loadRequestIdRef.current += 1;
      refreshRequestIdsRef.current.clear();
      searchHub.stop();
    };
  }, [mountedRef, runtimeProfile]); // eslint-disable-line react-hooks/exhaustive-deps

  // create a new search, and optionally navigate to it to display the details
  // we do this if the user clicks the search icon, or repeats an existing search
  const create = async ({ navigate = false, search } = {}) => {
    if (!mountedRef.current) {
      return;
    }

    const ref = inputRef?.current?.inputRef?.current;
    const searchText = (search ?? ref?.value ?? '').trim();
    const id = uuidv4();
    const requestId = ++createRequestIdRef.current;
    const isCurrentRequest = () =>
      mountedRef.current && createRequestIdRef.current === requestId;

    if (!searchText) {
      toast.error('Please enter a search phrase');
      return;
    }

    try {
      setCreating(true);

      // Include provider selection if Scene ↔ Pod Bridging is enabled
      const providers = scenePodBridgeEnabled
        ? [providerPod && 'pod', providerScene && 'scene'].filter(Boolean)
        : null;

      await library.create({
        acquisitionProfile: acquisitionProfile.id,
        id,
        providers,
        searchText,
      });

      if (!isCurrentRequest()) {
        return;
      }

      const initialSearch = {
        endedAt: null,
        fileCount: 0,
        id,
        isComplete: false,
        lockedFileCount: 0,
        query: searchText,
        responseCount: 0,
        responsesAvailable: false,
        searchText,
        startedAt: new Date().toISOString(),
        state: 'InProgress',
        status: 'active',
      };

      // The create response is acknowledged only after the backend has
      // persisted the search, but the hub CREATE event can race this render.
      // Hydrate once so local-share matches and the full status projection are
      // visible even when that event is missed during connection startup.
      let hydratedSearch = {};
      try {
        let hydrationTimeout;
        const status = await Promise.race([
          library.getStatus({ id }),
          new Promise((resolve) => {
            hydrationTimeout = setTimeout(
              () => resolve({}),
              SEARCH_CREATE_HYDRATION_TIMEOUT_MS,
            );
          }),
        ]).finally(() => clearTimeout(hydrationTimeout));
        if (status && typeof status === 'object' && !Array.isArray(status)) {
          hydratedSearch = status;
        }
      } catch (hydrateError) {
        // The create succeeded; retain the local projection and let the hub or
        // the normal refresh path populate the rest when the status endpoint
        // is temporarily unavailable.
        console.debug('failed to hydrate newly created search', hydrateError);
      }

      if (!isCurrentRequest()) {
        return;
      }

      setSearches((old) => ({
        ...old,
        [id]: {
          ...mergeSearchRecords(
            mergeSearchRecords(initialSearch, old[id] ?? {}),
            hydratedSearch,
          ),
          id,
        },
      }));

      if (ref) {
        ref.value = '';
      }

      if (navigate) {
        routerNavigate(`/searches/${id}`);
      }

      return id;
    } catch (createError) {
      if (!isCurrentRequest()) {
        return;
      }
      console.error(createError);
      toast.error(toDisplayError(createError, 'Failed to create search'));
    } finally {
      if (isCurrentRequest()) {
        setCreating(false);
      }
    }
  };

  // delete a search
  const remove = async (search) => {
    if (!mountedRef.current) {
      return;
    }
    const requestId = ++removeRequestIdRef.current;

    try {
      setRemoving(true);

      await library.remove({ id: search.id });
      if (
        !mountedRef.current ||
        removeRequestIdRef.current !== requestId
      ) {
        return;
      }
      setSearches((old) => {
        const next = { ...old };
        delete next[search.id];
        return next;
      });
    } catch (error_) {
      if (
        !mountedRef.current ||
        removeRequestIdRef.current !== requestId
      ) {
        return;
      }
      console.error(error_);
      toast.error(toDisplayError(error_, 'Failed to remove search'));
    } finally {
      if (
        mountedRef.current &&
        removeRequestIdRef.current === requestId
      ) {
        setRemoving(false);
      }
    }
  };

  // delete all searches
  const removeAll = async () => {
    if (!mountedRef.current) {
      return;
    }
    const requestId = ++removeAllRequestIdRef.current;

    try {
      setRemovingAll(true);
      const result = await library.removeAll();
      if (
        !mountedRef.current ||
        removeAllRequestIdRef.current !== requestId
      ) {
        return;
      }
      setSearches({});
      toast.success(`Cleared ${result?.data?.deleted ?? 'all'} searches`);
    } catch (removeAllError) {
      if (
        !mountedRef.current ||
        removeAllRequestIdRef.current !== requestId
      ) {
        return;
      }
      console.error(removeAllError);
      toast.error(toDisplayError(removeAllError, 'Failed to clear searches'));
    } finally {
      if (
        mountedRef.current &&
        removeAllRequestIdRef.current === requestId
      ) {
        setRemovingAll(false);
      }
    }
  };

  // stop an in-progress search
  const stop = async (search) => {
    if (!mountedRef.current) {
      return;
    }
    const requestId = ++stopRequestIdRef.current;

    try {
      setStopping(true);
      await library.stop({ id: search.id });
      if (!mountedRef.current || stopRequestIdRef.current !== requestId) {
        return;
      }
      setSearches((old) => ({
        ...old,
        [search.id]: {
          ...search,
          // The row callback can hold an older projection than the hub/REST
          // state. Preserve the freshest fields while forcing cancellation.
          ...old[search.id],
          endedAt: new Date().toISOString(),
          isComplete: true,
          state: 'Cancelled',
          status: 'cancelled',
        },
      }));
    } catch (stoppingError) {
      if (!mountedRef.current || stopRequestIdRef.current !== requestId) {
        return;
      }
      console.error(stoppingError);
      toast.error(toDisplayError(stoppingError, 'Failed to stop search'));
    } finally {
      if (
        mountedRef.current &&
        stopRequestIdRef.current === requestId
      ) {
        setStopping(false);
      }
    }
  };

  useEffect(() => {
    if (searchId || connecting || error || creating) {
      return;
    }

    inputRef?.current?.inputRef?.current?.focus();
  }, [connecting, creating, error, runtimeProfile, searchId]);

  useEffect(() => {
    if (
      !searchId ||
      connecting ||
      error ||
      creating ||
      !initialSearchesLoaded ||
      searches[searchId]
    ) {
      return;
    }

    let cancelled = false;

    const loadSearch = async () => {
      try {
        const search = await library.getStatus({ id: searchId });
        const resolvedId = searchEventIdentifier(search);
        if (
          cancelled ||
          !mountedRef.current ||
          resolvedId === undefined ||
          resolvedId === null
        ) {
          return;
        }
        onUpdate((old) => ({
          ...old,
          [searchId]: { ...search, id: searchId },
        }));
      } catch (loadError) {
        if (!cancelled && mountedRef.current) {
          console.debug('failed to load search details', loadError);
          routerNavigate('/searches', { replace: true });
        }
      }
    };

    void loadSearch();

    return () => {
      cancelled = true;
    };
  }, [
    connecting,
    creating,
    error,
    initialSearchesLoaded,
    mountedRef,
    routerNavigate,
    searchId,
    searches,
  ]);

  if (connecting) {
    return <LoaderSegment />;
  }

  if (error) {
    return <ErrorSegment caption={error?.message ?? error} />;
  }

  // if searchId is not null, there's an id in the route.
  // display the details for the search, if there is one
  if (searchId) {
    if (searches[searchId]) {
      return (
        <SearchDetail
          creating={creating}
          disabled={!normalizedServer.isConnected}
          onCreate={create}
          onRemove={remove}
          onStop={stop}
          removing={removing}
          search={searches[searchId]}
          stopping={stopping}
        />
      );
    }

    // if the searchId doesn't match a search we know about, chop
    // the id off of the url and force navigation back to the list
    if (creating) {
      return <LoaderSegment />;
    }

    return <LoaderSegment />;
  }

  if (runtimeProfile === 'legacy') {
    return (
      <>
        <Segment className="search-segment">
          <div className="search-segment-icon">
            <Icon
              name="search"
              size="big"
            />
          </div>
          <Input
            action={
              <>
                <Button
                  disabled={creating || !normalizedServer.isConnected}
                  icon="plus"
                  onClick={create}
                />
                <Button
                  disabled={creating || !normalizedServer.isConnected}
                  icon="search"
                  onClick={() => create({ navigate: true })}
                />
              </>
            }
            className="search-input"
            disabled={creating || !normalizedServer.isConnected}
            input={
              <input
                data-lpignore="true"
                placeholder="Connect to server to perform a search"
                type="search"
              />
            }
            loading={creating}
            onKeyUp={(keyUpEvent) =>
              keyUpEvent.key === 'Enter' ? create() : ''
            }
            placeholder="Search phrase"
            ref={inputRef}
            size="big"
          />
        </Segment>
        {Object.keys(searches).length === 0 ? (
          <PlaceholderSegment
            caption="No searches to display"
            icon="search"
          />
        ) : (
          <SearchList
            connecting={connecting}
            error={error}
            onRemove={remove}
            onStop={stop}
            searches={searches}
          />
        )}
      </>
    );
  }

  return (
    <>
      <CollapsibleSection
        storageKey="slskr.search.section.search"
        title="Search"
      >
        <Segment className="search-segment">
          <div className="search-segment-icon">
            <Icon
              name="search"
              size="big"
            />
          </div>
          <Input
            action={
              <>
                <Popup
                  content="Queue this search without leaving the search page."
                  position="top center"
                  trigger={
                    <Button
                      aria-label="Queue search"
                      disabled={creating || !normalizedServer.isConnected}
                      icon="plus"
                      onClick={create}
                    />
                  }
                />
                <Popup
                  content="Start this search and open its detailed results immediately."
                  position="top center"
                  trigger={
                    <Button
                      aria-label="Search and open results"
                      disabled={creating || !normalizedServer.isConnected}
                      icon="search"
                      onClick={() => create({ navigate: true })}
                    />
                  }
                />
              </>
            }
            className="search-input"
            disabled={creating || !normalizedServer.isConnected}
            input={
              <input
                data-lpignore="true"
                data-testid="search-input"
                placeholder={
                  normalizedServer.isConnected
                    ? 'Search phrase'
                    : 'Connect to server to perform a search'
                }
                type="search"
              />
            }
            loading={creating}
            onKeyUp={(keyUpEvent) => (keyUpEvent.key === 'Enter' ? create() : '')}
            placeholder="Search phrase"
            ref={inputRef}
            size="big"
          />
          {scenePodBridgeEnabled && (
            <div
              style={{
                background: 'rgba(0,0,0,0.05)',
                borderRadius: '4px',
                marginTop: '0.75em',
                padding: '0.75em',
              }}
            >
              <div
                style={{
                  alignItems: 'center',
                  display: 'flex',
                  flexWrap: 'wrap',
                  gap: '1em',
                }}
              >
                <span style={{ fontSize: '0.95em', fontWeight: 'bold' }}>
                  Search Sources:
                </span>
                <Checkbox
                  checked={providerPod}
                  label={
                    <label>
                      <Icon
                        name="sitemap"
                        style={{ marginRight: '0.25em' }}
                      />
                      Pod/Mesh
                    </label>
                  }
                  onChange={(e, { checked }) => setProviderPod(checked)}
                  toggle
                />
                <Checkbox
                  checked={providerScene}
                  label={
                    <label>
                      <Icon
                        name="globe"
                        style={{ marginRight: '0.25em' }}
                      />
                      Soulseek Scene
                    </label>
                  }
                  onChange={(e, { checked }) => setProviderScene(checked)}
                  toggle
                />
                {!providerPod && !providerScene && (
                  <span
                    style={{
                      color: 'orange',
                      fontSize: '0.9em',
                      fontStyle: 'italic',
                    }}
                  >
                    <Icon name="warning" /> At least one source must be selected
                  </span>
                )}
              </div>
            </div>
          )}
          <div className="search-acquisition-profile-strip">
            <div className="search-acquisition-profile-label">
              <Icon name={acquisitionProfile.icon} />
              Acquisition Profile
            </div>
            {runtimeProfile !== 'native' && (
              <Popup
                content={`${acquisitionProfile.label}: ${acquisitionProfile.description}`}
                position="top center"
                trigger={
                  <Dropdown
                    aria-label="Acquisition profile"
                    className="search-acquisition-profile-dropdown"
                    data-testid="acquisition-profile-select"
                    onChange={updateAcquisitionProfile}
                    options={acquisitionProfileOptions}
                    selection
                    value={acquisitionProfile.id}
                  />
                }
              />
            )}
            <span className="search-acquisition-profile-summary">
              {acquisitionProfile.summary}
            </span>
          </div>
        </Segment>
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen={false}
        storageKey="slskr.search.section.songid"
        title="SongID"
      >
        <SongIDPanel disabled={!normalizedServer.isConnected} />
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen={false}
        storageKey="slskr.search.section.musicbrainz"
        title="MusicBrainz Lookup"
      >
        <MusicBrainzLookup disabled={!normalizedServer.isConnected} />
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen={false}
        storageKey="slskr.search.section.discographyCoverage"
        title="Discography Concierge"
      >
        <DiscographyCoveragePanel disabled={!normalizedServer.isConnected} />
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen={false}
        storageKey="slskr.search.section.artistReleaseRadar"
        title="Artist Release Radar"
      >
        <ArtistReleaseRadarPanel disabled={!normalizedServer.isConnected} />
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen={false}
        storageKey="slskr.search.section.soulseekDiscovery"
        title="Soulseek Discovery"
      >
        <SoulseekDiscoveryPanel
          disabled={!normalizedServer.isConnected}
          onSearch={(search) => create({ navigate: true, search })}
        />
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen={false}
        storageKey="slskr.search.section.federatedTaste"
        title="Federated Taste"
      >
        <FederatedTasteRecommendationsPanel disabled={!normalizedServer.isConnected} />
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen={false}
        storageKey="slskr.search.section.discoveryGraphAtlas"
        title="Discovery Graph Atlas"
      >
        <DiscoveryGraphAtlasPanel disabled={!normalizedServer.isConnected} />
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen={false}
        storageKey="slskr.search.section.albumCompletion"
        title="Album Completion"
      >
        <AlbumCompletionPanel disabled={!normalizedServer.isConnected} />
      </CollapsibleSection>
      <CollapsibleSection
        defaultOpen
        storageKey="slskr.search.section.searchResults"
        title="Search Results"
      >
        {Object.keys(searches).length === 0 ? (
          <PlaceholderSegment
            caption="No searches to display"
            icon="search"
          />
        ) : (
          <SearchList
            connecting={connecting}
            error={error}
            onRemove={remove}
            onRemoveAll={removeAll}
            onStop={stop}
            removingAll={removingAll}
            searches={searches}
          />
        )}
      </CollapsibleSection>
    </>
  );
};

export default Searches;
