import React, { useState } from 'react';
import { getLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import PlaceholderSegment from '../Shared/PlaceholderSegment';
import AlbumCompletionPanel from './AlbumCompletionPanel';
import ArtistReleaseRadarPanel from './ArtistReleaseRadarPanel';
import DiscographyCoveragePanel from './DiscographyCoveragePanel';
import DiscoveryGraphAtlasPanel from './DiscoveryGraphAtlasPanel';
import FederatedTasteRecommendationsPanel from './FederatedTasteRecommendationsPanel';
import SearchList from './List/SearchList';
import MusicBrainzLookup from './MusicBrainzLookup';
import SongIDPanel from './SongIDPanel';
import SoulseekDiscoveryPanel from './SoulseekDiscoveryPanel';
import {
  Button,
  Checkbox,
  Dropdown,
  Header,
  Icon,
  Input,
  Message,
  Popup,
  Segment,
} from 'semantic-ui-react';

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

const SearchesListView = ({ actions, inputRef, state }) => {
  const {
    acquisitionProfile,
    acquisitionProfileOptions,
    connecting,
    creating,
    error,
    hasSearchData,
    initialSearchesLoaded,
    normalizedServer,
    providerPod,
    providerScene,
    removingAll,
    runtimeProfile,
    scenePodBridgeEnabled,
    searches,
  } = state;
  const {
    create,
    remove,
    removeAll,
    setProviderPod,
    setProviderScene,
    stop,
    updateAcquisitionProfile,
  } = actions;
  const connectionNotice = connecting && hasSearchData ? (
    <Message info size="small">
      <Icon loading name="circle notched" />
      Connecting to live search updates. Your loaded searches are available below.
    </Message>
  ) : null;

  if (runtimeProfile === 'legacy') {
    return (
      <>
        {connectionNotice}
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
            connecting={connecting && !initialSearchesLoaded}
            error={hasSearchData ? undefined : error}
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
      {connectionNotice}
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
            connecting={connecting && !initialSearchesLoaded}
            error={hasSearchData ? undefined : error}
            onRemove={remove}
            onRemoveAll={removeAll}
            onStop={stop}
            removingAll={removingAll}
            searches={searches}
          />
        )}
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
    </>
  );
};

export default SearchesListView;
