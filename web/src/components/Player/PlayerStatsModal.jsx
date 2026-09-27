import * as listenBrainz from '../../lib/listenBrainz';
import {
  clearListeningHistory,
  exportListeningHistoryCsv,
  exportListeningHistoryJson,
  getListeningRecommendationQueries,
  getListeningRecommendationSeeds,
  getListeningStats,
  importListeningHistory,
} from '../../lib/listeningHistory';
import * as searches from '../../lib/searches';
import * as wishlistAPI from '../../lib/wishlist';
import { copyToClipboard } from '../../lib/clipboard';
import { toDisplayError } from '../../lib/errors';
import { readFileTextBounded } from '../../lib/fileReaders';
import { useMountedRef } from '../../lib/useMountedRef';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Button,
  Icon,
  Message,
  Modal,
  Popup,
  TextArea,
} from 'semantic-ui-react';

const PlayerStatsModal = ({ onClose, onOpenSearch, open }) => {
  const fileInputRef = useRef(null);
  const [rangeDays, setRangeDays] = useState(30);
  const [importText, setImportText] = useState('');
  const [importStatus, setImportStatus] = useState(null);
  const [runningSeedSearches, setRunningSeedSearches] = useState(false);
  const [scrobblingRecent, setScrobblingRecent] = useState(false);
  const [savingSeedWishlist, setSavingSeedWishlist] = useState(false);
  const [stats, setStats] = useState(() =>
    getListeningStats({ rangeDays: 30 }),
  );
  const mountedRef = useMountedRef();
  const openRef = useRef(open);
  const actionRequestIdRef = useRef(0);
  const actionInFlightRef = useRef(false);

  useEffect(() => {
    openRef.current = open;
    if (!open) {
      actionRequestIdRef.current += 1;
      setRunningSeedSearches(false);
      setScrobblingRecent(false);
      setSavingSeedWishlist(false);
    }
  }, [open]);

  const isActive = () => mountedRef.current && openRef.current;
  const recommendationSeeds = getListeningRecommendationSeeds(stats);
  const refreshStats = useCallback((nextRangeDays = rangeDays) => {
    setStats(getListeningStats({ rangeDays: nextRangeDays }));
  }, [rangeDays]);

  useEffect(() => {
    if (open) refreshStats();
  }, [open, refreshStats]);

  const clearStats = () => {
    clearListeningHistory();
    setImportStatus(null);
    refreshStats();
  };

  const updateRange = (nextRangeDays) => {
    setRangeDays(nextRangeDays);
    refreshStats(nextRangeDays);
  };

  const importHistory = () => {
    const result = importListeningHistory(importText);
    setImportStatus(
      `${result.imported} imported, ${result.skipped} skipped as duplicates or incomplete rows.`,
    );
    setImportText('');
    refreshStats();
  };

  const copyHistory = async (format) => {
    const content = format === 'csv'
      ? exportListeningHistoryCsv()
      : exportListeningHistoryJson();

    try {
      const copied = await copyToClipboard(content);
      if (!mountedRef.current) return;
      setImportStatus(
        copied
          ? `${format.toUpperCase()} export copied for ${stats.history.length} plays.`
          : `Prepared ${format.toUpperCase()} export for ${stats.history.length} plays; clipboard is unavailable.`,
      );
    } catch (error) {
      if (mountedRef.current) {
        setImportStatus(toDisplayError(error, `Unable to copy ${format.toUpperCase()} export.`));
      }
    }
  };

  const startSeedSearches = async () => {
    if (!isActive() || actionInFlightRef.current) return;
    const requestId = ++actionRequestIdRef.current;
    const isCurrentRequest = () =>
      isActive() && actionRequestIdRef.current === requestId;
    const queries = getListeningRecommendationQueries(stats, { limit: 3 });
    if (queries.length === 0) {
      setImportStatus('No listening seeds are ready to search.');
      return;
    }

    actionInFlightRef.current = true;
    try {
      setRunningSeedSearches(true);
      const count = await searches.createBatch({ queries });
      if (!isCurrentRequest()) return;
      setImportStatus(`Started ${count} bounded listening seed search${count === 1 ? '' : 'es'}.`);
    } catch {
      if (!isCurrentRequest()) return;
      setImportStatus('Unable to start listening seed searches.');
    } finally {
      actionInFlightRef.current = false;
      if (isCurrentRequest()) setRunningSeedSearches(false);
    }
  };

  const addSeedsToWishlist = async () => {
    if (!isActive() || actionInFlightRef.current) return;
    const requestId = ++actionRequestIdRef.current;
    const isCurrentRequest = () =>
      isActive() && actionRequestIdRef.current === requestId;
    const queries = getListeningRecommendationQueries(stats, { limit: 5 });
    if (queries.length === 0) {
      setImportStatus('No listening seeds are ready for Wishlist.');
      return;
    }

    actionInFlightRef.current = true;
    try {
      setSavingSeedWishlist(true);
      await queries.reduce(
        (chain, searchText) =>
          chain.then(() =>
            wishlistAPI.create({
              autoDownload: false,
              enabled: true,
              filter: '',
              maxResults: 50,
              searchText,
            }),
          ),
        Promise.resolve(),
      );
      if (!isCurrentRequest()) return;
      setImportStatus(`Added ${queries.length} listening seed${queries.length === 1 ? '' : 's'} to Wishlist for manual acquisition.`);
    } catch {
      if (!isCurrentRequest()) return;
      setImportStatus('Unable to add listening seeds to Wishlist.');
    } finally {
      actionInFlightRef.current = false;
      if (isCurrentRequest()) setSavingSeedWishlist(false);
    }
  };

  const scrobbleRecentHistory = async () => {
    if (!isActive() || actionInFlightRef.current) return;
    const requestId = ++actionRequestIdRef.current;
    const isCurrentRequest = () =>
      isActive() && actionRequestIdRef.current === requestId;
    actionInFlightRef.current = true;
    try {
      setScrobblingRecent(true);
      const result = await listenBrainz.submitListeningHistory(stats.history, {
        limit: 10,
      });
      if (!isCurrentRequest()) return;
      setImportStatus(
        result.submitted > 0
          ? `Submitted ${result.submitted} recent listen${result.submitted === 1 ? '' : 's'} to ListenBrainz.`
          : 'No ListenBrainz token or eligible recent listens are available.',
      );
    } catch {
      if (!isCurrentRequest()) return;
      setImportStatus('Unable to submit recent listens to ListenBrainz.');
    } finally {
      actionInFlightRef.current = false;
      if (isCurrentRequest()) setScrobblingRecent(false);
    }
  };

  const readImportFile = (event) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const requestId = ++actionRequestIdRef.current;
    const isCurrentRequest = () =>
      isActive() && actionRequestIdRef.current === requestId;

    readFileTextBounded(file).then((content) => {
      if (!isCurrentRequest()) return;
      setImportText(content);
      setImportStatus(`Loaded ${file.name} for review.`);
    }).catch((error) => {
      if (!isCurrentRequest()) return;
      setImportStatus(`Could not read ${file.name}: ${toDisplayError(error)}`);
    });
    event.target.value = '';
  };

  const renderList = (items, emptyText) => (
    items.length > 0 ? (
      <div className="player-stats-list">
        {items.map((item, index) => (
          <div className="player-stats-row" key={`${item.label || item.title}-${index}`}>
            <span>{index + 1}</span>
            <strong>{item.label || item.title}</strong>
            <em>{item.plays ? `${item.plays} plays` : item.artist || item.album || ''}</em>
          </div>
        ))}
      </div>
    ) : (
      <div className="player-queue-manager-empty">{emptyText}</div>
    )
  );

  return (
    <Modal
      className="player-browser-modal player-stats-modal"
      onClose={onClose}
      open={open}
      size="small"
    >
      <Modal.Header>Listening Stats</Modal.Header>
      <Modal.Content>
        <div className="player-stats-summary" data-testid="player-stats-summary">
          <Icon name="bar chart" />
          <div>
            <strong>{stats.totalPlays}</strong>
            <span>
              local plays recorded in this browser
              {rangeDays ? ` over ${rangeDays} days` : ' overall'}
            </span>
          </div>
        </div>
        <div className="player-stats-ranges" role="group" aria-label="Listening stats range">
          {[
            { label: '7D', value: 7 },
            { label: '30D', value: 30 },
            { label: '90D', value: 90 },
            { label: 'All', value: null },
          ].map((range) => (
            <Button
              active={rangeDays === range.value}
              data-testid={`player-stats-range-${range.label}`}
              key={range.label}
              onClick={() => updateRange(range.value)}
              size="mini"
              type="button"
            >
              {range.label}
            </Button>
          ))}
        </div>
        <div className="player-stats-grid">
          <section>
            <div className="player-panel-title">Top Artists</div>
            {renderList(stats.topArtists, 'No artist plays recorded yet.')}
          </section>
          <section>
            <div className="player-panel-title">Top Tracks</div>
            {renderList(stats.topTracks, 'No track plays recorded yet.')}
          </section>
          <section>
            <div className="player-panel-title">Top Genres</div>
            {renderList(stats.topGenres, 'No genre metadata recorded yet.')}
          </section>
          <section>
            <div className="player-panel-title">Recent</div>
            {renderList(stats.recent, 'No recent plays recorded yet.')}
          </section>
          <section>
            <div className="player-panel-title">Forgotten Favorites</div>
            {renderList(
              stats.forgottenFavorites,
              'No older repeat plays outside this range yet.',
            )}
          </section>
        </div>
        <section className="player-stats-recommendations">
          <div className="player-panel-title">Recommendation Seeds</div>
          {recommendationSeeds.length > 0 ? (
            <>
              <div className="player-stats-seed-list">
                {recommendationSeeds.map((seed) => (
                  <div className="player-stats-seed-row" key={`${seed.type}-${seed.query}`}>
                    <div>
                      <strong>{seed.label}</strong>
                      <span>{seed.type} - {seed.basis}</span>
                    </div>
                    <Popup
                      content="Open this local listening seed as a normal Search page query. Network search starts only after you choose to search."
                      trigger={
                        <Button
                          aria-label={`Search ${seed.label}`}
                          data-testid={`player-stats-search-seed-${seed.query}`}
                          icon
                          onClick={() => onOpenSearch(seed.query)}
                          size="mini"
                          type="button"
                        >
                          <Icon name="search" />
                        </Button>
                      }
                    />
                  </div>
                ))}
              </div>
              <Popup
                content="Start up to three live searches from the strongest listening seeds. This only starts searches; it does not browse peers, queue downloads, or mutate files."
                trigger={
                  <Button
                    data-testid="player-stats-start-seed-searches"
                    disabled={
                      recommendationSeeds.length === 0 ||
                      runningSeedSearches ||
                      savingSeedWishlist ||
                      scrobblingRecent
                    }
                    loading={runningSeedSearches}
                    onClick={startSeedSearches}
                    size="mini"
                    type="button"
                  >
                    <Icon name="search" />
                    Start Searches
                  </Button>
                }
              />
              <Popup
                content="Add up to five listening seeds to Wishlist as enabled manual-acquisition requests. Auto-download stays off."
                trigger={
                  <Button
                    data-testid="player-stats-add-seeds-to-wishlist"
                    disabled={
                      recommendationSeeds.length === 0 ||
                      runningSeedSearches ||
                      savingSeedWishlist ||
                      scrobblingRecent
                    }
                    loading={savingSeedWishlist}
                    onClick={addSeedsToWishlist}
                    size="mini"
                    type="button"
                  >
                    <Icon name="heart" />
                    Add Wishlist
                  </Button>
                }
              />
            </>
          ) : (
            <div className="player-queue-manager-empty">
              Play more tracks locally to build recommendation seeds.
            </div>
          )}
        </section>
        <section className="player-stats-import">
          <div className="player-panel-title">Media Server Import</div>
          <TextArea
            aria-label="Paste exported media server play history"
            data-testid="player-listening-history-import-text"
            onChange={(event) => setImportText(event.target.value)}
            placeholder="Paste Plex, Jellyfin, Navidrome, or generic CSV/JSON play history here for local import."
            rows={4}
            value={importText}
          />
          {importStatus ? (
            <Message compact size="mini">
              {importStatus}
            </Message>
          ) : null}
          <div className="player-stats-import-actions">
            <input
              accept=".csv,.json,.txt"
              aria-label="Choose media server history file"
              data-testid="player-listening-history-file"
              onChange={readImportFile}
              ref={fileInputRef}
              type="file"
            />
            <Popup
              content="Choose a local CSV or JSON export from Plex, Jellyfin, Navidrome, or another media server. The file is read in this browser only."
              trigger={
                <Button
                  data-testid="player-listening-history-choose-file"
                  onClick={() => fileInputRef.current?.click()}
                  size="mini"
                  type="button"
                >
                  <Icon name="folder open" />
                  Choose File
                </Button>
              }
            />
            <Popup
              content="Import the pasted or chosen play history into browser-local listening stats with duplicate suppression."
              trigger={
                <Button
                  data-testid="player-listening-history-import"
                  disabled={!importText.trim()}
                  onClick={importHistory}
                  primary
                  size="mini"
                  type="button"
                >
                  <Icon name="upload" />
                  Import
                </Button>
              }
            />
            <Popup
              content="Copy the browser-local listening history as JSON for backup or review."
              trigger={
                <Button
                  data-testid="player-listening-history-export-json"
                  disabled={stats.history.length === 0}
                  onClick={() => copyHistory('json')}
                  size="mini"
                  type="button"
                >
                  <Icon name="copy" />
                  JSON
                </Button>
              }
            />
            <Popup
              content="Copy the browser-local listening history as CSV for media-server or spreadsheet review."
              trigger={
                <Button
                  data-testid="player-listening-history-export-csv"
                  disabled={stats.history.length === 0}
                  onClick={() => copyHistory('csv')}
                  size="mini"
                  type="button"
                >
                  <Icon name="table" />
                  CSV
                </Button>
              }
            />
            <Popup
              content="Submit up to ten recent browser-local plays to ListenBrainz using the saved token. This does not search, browse peers, download, or mutate files."
              trigger={
                <Button
                  data-testid="player-listening-history-scrobble-recent"
                  disabled={
                    stats.history.length === 0 ||
                    runningSeedSearches ||
                    savingSeedWishlist ||
                    scrobblingRecent
                  }
                  loading={scrobblingRecent}
                  onClick={scrobbleRecentHistory}
                  size="mini"
                  type="button"
                >
                  <Icon name="send" />
                  Scrobble Recent
                </Button>
              }
            />
          </div>
        </section>
      </Modal.Content>
      <Modal.Actions>
        <Popup
          content="Clear only the browser-local listening history used for this stats view."
          trigger={
            <Button
              data-testid="player-clear-listening-history"
              disabled={stats.totalPlays === 0}
              onClick={clearStats}
              type="button"
            >
              <Icon name="trash alternate outline" />
              Clear Local History
            </Button>
          }
        />
        <Popup
          content="Close listening stats."
          trigger={
            <Button
              data-testid="player-close-listening-stats"
              onClick={onClose}
              primary
              type="button"
            >
              <Icon name="check" />
              Done
            </Button>
          }
        />
      </Modal.Actions>
    </Modal>
  );
};

export default PlayerStatsModal;
