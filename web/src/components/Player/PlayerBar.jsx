import './Player.css';
import { upsertDiscoveryShelfItem } from '../../lib/discoveryShelf';
import * as listenBrainz from '../../lib/listenBrainz';
import { recordLocalPlay } from '../../lib/listeningHistory';
import {
  getPlayerRating,
  setPlayerRating,
} from '../../lib/playerRatings';
import { buildPlayerRadioSearchPath } from '../../lib/playerRadio';
import { getPlayerShortcutAction } from '../../lib/playerShortcuts';
import { getLocalStorageItem, setLocalStorageItem } from '../../lib/storage';
import { useMountedRef } from '../../lib/useMountedRef';
import Equalizer from './Equalizer';
import LyricsPane from './LyricsPane';
import { PlayerAnalyzerTile, PlayerVisualTile, readStoredVisualizerEngineTileMode } from './PlayerVisualTiles';
import PlayerIntegrationsModal from './PlayerIntegrationsModal';
import PlayerRatingControls from './PlayerRatingControls';
import PlayerToolButton from './PlayerToolButton';
import usePlayerStreamSource from './usePlayerStreamSource';
import useExternalVisualizerControl from './useExternalVisualizerControl';
import usePlayerPictureInPicture from './usePlayerPictureInPicture';
import PlayerLauncher from './PlayerLauncher';
import PlayerStatsModal from './PlayerStatsModal';
import PlayerRadioModal from './PlayerRadioModal';
import PlayerQueueModal from './PlayerQueueModal';
import PlayerDiscoveryShelfModal from './PlayerDiscoveryShelfModal';
import { fadeOutputGain, resumeAudioGraph, setKaraokeEnabled, setOutputGain } from './audioGraph';
import { usePlayer } from './PlayerContext';
import { useNavigate } from 'react-router-dom';
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import {
  Button,
  Checkbox,
  Icon,
  Label,
  Modal,
  Popup,
} from 'semantic-ui-react';

const localMuteStorageKey = 'slskr.player.localMuted';
const collapsedStorageKey = 'slskr.player.collapsed';
const visualizerStorageKey = 'slskr.player.visualizerEnabled';
const eqPanelStorageKey = 'slskr.player.eqPanelOpen';
const karaokeStorageKey = 'slskr.player.karaokeEnabled';
const crossfadeStorageKey = 'slskr.player.crossfadeEnabled';
const visualTileStorageKey = 'slskr.player.visualTileMode';
const analyzerModeStorageKey = 'slskr.player.analyzerMode';

const readStoredBoolean = (key) => {
  return getLocalStorageItem(key) === 'true';
};

const readStoredTileMode = () => {
  const mode = getLocalStorageItem(visualTileStorageKey);
  if (['art', 'spectrum', 'scope', 'rustymilk-webgl2', 'rustymilk-webgpu'].includes(mode)) {
    return mode;
  }
  if (mode === 'rustymilk') {
    return readStoredVisualizerEngineTileMode();
  }
  return 'art';
};

const readStoredAnalyzerMode = () => {
  const mode = getLocalStorageItem(analyzerModeStorageKey);
  return mode === 'scope' ? 'scope' : 'spectrum';
};

const setPlayerHeightVariable = (element) => {
  if (!element || typeof document === 'undefined') return;

  const height = Math.ceil(element.getBoundingClientRect().height);
  if (height > 0) {
    document.documentElement.style.setProperty(
      '--slskr-player-reserved-height',
      `${height}px`,
    );
  }
};



const formatPlayerProvider = (provider = '') => {
  const normalized = String(provider).trim();
  if (!normalized) return '';
  if (normalized.toLowerCase() === 'soulseek') return 'Soulseek';
  if (normalized.toLowerCase() === 'mesh') return 'Mesh';
  if (normalized.toLowerCase() === 'pod') return 'Pod';
  return normalized;
};

const getPlayerBadges = (current) => {
  if (!current) return [];

  const providers = (Array.isArray(current.sourceProviders)
    ? current.sourceProviders
    : [])
    .map(formatPlayerProvider)
    .filter(Boolean);
  const badges = providers.slice(0, 2).map((provider) => ({
    color: provider === 'Mesh' || provider === 'Pod' ? 'violet' : 'grey',
    icon: provider === 'Mesh' ? 'share alternate' : 'music',
    key: `source-${provider}`,
    text: provider,
    title: `Playback source: ${provider}`,
  }));

  const confidence = Number(current.confidence || 0);
  if (confidence > 0) {
    badges.push({
      color: confidence >= 0.75 ? 'green' : 'yellow',
      icon: 'crosshairs',
      key: 'confidence',
      text: `${Math.round(confidence * 100)}% match`,
      title: 'Local match confidence for this now-playing item.',
    });
  }

  if (current.verified) {
    badges.push({
      color: 'teal',
      icon: 'check circle',
      key: 'verified',
      text: 'Verified',
      title: 'This item has local verification evidence.',
    });
  }

  return badges;
};



const PlayerBar = ({ runtimeProfile } = {}) => {
  const navigate = useNavigate();
  const mountedRef = useMountedRef();
  const audioRef = useRef(null);
  const fadeAudioRef = useRef(null);
  const fadePauseTimeoutRef = useRef(null);
  const fadeGenerationRef = useRef(0);
  const lastSourceRef = useRef('');
  const playerBarRef = useRef(null);
  const scrobbledRef = useRef('');
  const {
    clearQueue,
    clear,
    current,
    followingParty,
    history,
    next,
    pause,
    queue,
    previous,
    queueItems,
    removeFromQueue,
    seekRelative,
    setAudioElement,
    playItem,
  } = usePlayer();
  const [localMuted, setLocalMuted] = useState(() =>
    readStoredBoolean(localMuteStorageKey),
  );
  const [collapsed, setCollapsed] = useState(() => {
    const stored = getLocalStorageItem(collapsedStorageKey);
    // The frozen native profile opens its player drawer by default.  Keep the
    // compact default for the general slskR surface, while preserving an
    // explicit preference for either profile.
    return stored === null ? runtimeProfile !== 'native' : stored === 'true';
  });
  const [playing, setPlaying] = useState(false);
  const [visualizerMode, setVisualizerMode] = useState(() =>
    readStoredBoolean(visualizerStorageKey) ? 'inline' : 'off',
  );
  const [visualTileMode, setVisualTileMode] = useState(readStoredTileMode);
  const [analyzerMode, setAnalyzerMode] = useState(readStoredAnalyzerMode);
  const [eqPanelOpen, setEqPanelOpen] = useState(() =>
    readStoredBoolean(eqPanelStorageKey),
  );
  const [lyricsOpen, setLyricsOpen] = useState(false);
  const [karaokeEnabled, setKaraokeEnabledState] = useState(() =>
    readStoredBoolean(karaokeStorageKey),
  );
  const [crossfadeEnabled, setCrossfadeEnabled] = useState(() =>
    readStoredBoolean(crossfadeStorageKey),
  );
  const [listenBrainzToken, setListenBrainzTokenState] = useState(() =>
    listenBrainz.getListenBrainzToken(),
  );
  const [integrationsOpen, setIntegrationsOpen] = useState(false);
  const [queueOpen, setQueueOpen] = useState(false);
  const [radioOpen, setRadioOpen] = useState(false);
  const [shelfOpen, setShelfOpen] = useState(false);
  const [statsOpen, setStatsOpen] = useState(false);
  const [playerAudioElement, setPlayerAudioElement] = useState(null);
  const [playerRating, setPlayerRatingState] = useState(0);

  const {
    externalVisualizerLaunching,
    externalVisualizerLoading,
    externalVisualizerMessage,
    externalVisualizerStatus,
    launchExternalVisualizer,
    refreshExternalVisualizerStatus,
  } = useExternalVisualizerControl({ integrationsOpen, mountedRef });

  const bindAudioElement = useCallback((element) => {
    audioRef.current = element;
    if (element !== null || mountedRef.current) {
      setPlayerAudioElement(element);
      setAudioElement(element);
    }
  }, [mountedRef, setAudioElement]);

  useLayoutEffect(() => {
    const element = playerBarRef.current;
    if (!element) return undefined;

    setPlayerHeightVariable(element);
    if (typeof window.ResizeObserver !== 'function') {
      return undefined;
    }

    const resizeObserver = new window.ResizeObserver(() =>
      setPlayerHeightVariable(element));
    resizeObserver.observe(element);

    return () => resizeObserver.disconnect();
  }, [collapsed, current, eqPanelOpen, lyricsOpen]);

  const playAudio = useCallback(async () => {
    if (!mountedRef.current || !audioRef.current) return;
    const element = audioRef.current;
    await resumeAudioGraph(element);
    if (!mountedRef.current || audioRef.current !== element) return;
    await element.play();
  }, [mountedRef]);

  useEffect(() => {
    if (!playerAudioElement) return;
    playerAudioElement.muted = localMuted;
    setLocalStorageItem(localMuteStorageKey, localMuted ? 'true' : 'false');
  }, [localMuted, playerAudioElement]);

  useEffect(() => {
    setLocalStorageItem(collapsedStorageKey, collapsed ? 'true' : 'false');
  }, [collapsed]);

  useEffect(() => {
    document.documentElement.classList.toggle('player-collapsed', collapsed);
    return () => {
      document.documentElement.classList.remove('player-collapsed');
    };
  }, [collapsed]);

  useEffect(() => {
    setLocalStorageItem(
      visualizerStorageKey,
      visualizerMode !== 'off' ? 'true' : 'false',
    );
  }, [visualizerMode]);

  useEffect(() => {
    setLocalStorageItem(visualTileStorageKey, visualTileMode);
  }, [visualTileMode]);

  useEffect(() => {
    setLocalStorageItem(analyzerModeStorageKey, analyzerMode);
  }, [analyzerMode]);

  useEffect(() => {
    setLocalStorageItem(eqPanelStorageKey, eqPanelOpen ? 'true' : 'false');
  }, [eqPanelOpen]);

  useEffect(() => {
    setLocalStorageItem(
      karaokeStorageKey,
      karaokeEnabled ? 'true' : 'false',
    );
    if (playerAudioElement) {
      setKaraokeEnabled(playerAudioElement, karaokeEnabled);
    }
  }, [karaokeEnabled, playerAudioElement]);

  useEffect(() => {
    setLocalStorageItem(
      crossfadeStorageKey,
      crossfadeEnabled ? 'true' : 'false',
    );
  }, [crossfadeEnabled]);

  const toggleVisualizer = () => {
    setVisualizerMode((mode) => {
      if (mode === 'off') {
        setVisualTileMode(readStoredVisualizerEngineTileMode());
        return 'inline';
      }
      return 'off';
    });
  };

  useEffect(() => {
    setPlayerRatingState(getPlayerRating(current));
  }, [current]);

  const updatePlayerRating = (rating) => {
    const nextRating = setPlayerRating(current, rating);
    setPlayerRatingState(nextRating);
    upsertDiscoveryShelfItem(current, nextRating);
  };

  const openRadioSearch = (query) => {
    setRadioOpen(false);
    navigate(buildPlayerRadioSearchPath(query));
  };

  const togglePlayback = useCallback(() => {
    if (!audioRef.current || !current) return;
    if (playing) {
      pause();
    } else {
      playAudio().catch(() => {});
    }
  }, [current, pause, playAudio, playing]);

  const source = usePlayerStreamSource({ current, mountedRef });

  useEffect(() => {
    const handleKeyDown = (event) => {
      const action = getPlayerShortcutAction(event);
      if (!action || !current) return;

      event.preventDefault();

      if (action === 'togglePlayback') {
        togglePlayback();
      } else if (action === 'seekBackward') {
        seekRelative(-15);
      } else if (action === 'seekForward') {
        seekRelative(30);
      } else if (action === 'previous') {
        previous();
      } else if (action === 'next') {
        next();
      } else if (action === 'toggleMute') {
        setLocalMuted((muted) => !muted);
      } else if (action === 'toggleEqualizer') {
        setEqPanelOpen((open) => !open);
      } else if (action === 'toggleLyrics') {
        setLyricsOpen((open) => !open);
      } else if (action === 'toggleVisualizer') {
        toggleVisualizer();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    current,
    next,
    previous,
    seekRelative,
    togglePlayback,
    toggleVisualizer,
  ]);

  const openPictureInPicture = usePlayerPictureInPicture({ audioRef, mountedRef });

  useEffect(() => () => {
    fadeGenerationRef.current += 1;
    if (fadePauseTimeoutRef.current !== null) {
      window.clearTimeout(fadePauseTimeoutRef.current);
      fadePauseTimeoutRef.current = null;
    }
  }, []);

  useEffect(() => {
    const fadeGeneration = ++fadeGenerationRef.current;
    if (fadePauseTimeoutRef.current !== null) {
      window.clearTimeout(fadePauseTimeoutRef.current);
      fadePauseTimeoutRef.current = null;
    }
    if (!audioRef.current || !source) return;
    const previousSource = lastSourceRef.current;
    const fadeAudio = fadeAudioRef.current;
    if (crossfadeEnabled && previousSource && previousSource !== source && fadeAudio) {
      fadeAudio.src = previousSource;
      fadeAudio.currentTime = audioRef.current.currentTime || 0;
      fadeAudio.play().then(() => {
        if (fadeGenerationRef.current !== fadeGeneration) return;
        fadeOutputGain(fadeAudio, 1, 0, 5);
        fadePauseTimeoutRef.current = window.setTimeout(() => {
          if (fadeGenerationRef.current !== fadeGeneration) return;
          fadePauseTimeoutRef.current = null;
          fadeAudio.pause();
        }, 5200);
      }).catch(() => {});
      setOutputGain(audioRef.current, 0);
      fadeOutputGain(fadeAudio, 0, 1, 5);
    } else {
      setOutputGain(audioRef.current, 1);
    }
    lastSourceRef.current = source;
    audioRef.current.load();
    playAudio().catch(() => {});
  }, [crossfadeEnabled, playAudio, source]);

  useEffect(() => {
    if (!current?.artist || !current?.title) return;
    listenBrainz.submitListen('playing_now', current).catch(() => {});
    scrobbledRef.current = '';
  }, [current]);

  useEffect(() => {
    const audioElement = playerAudioElement;
    if (!audioElement || !current) return undefined;

    const handleTimeUpdate = () => {
      const duration = Number.isFinite(audioElement.duration)
        ? audioElement.duration
        : 0;
      const threshold = duration > 0
        ? Math.min(duration / 2, 240)
        : 240;
      const scrobbleKey = `${current.contentId}:${current.title}`;

        if (audioElement.currentTime >= threshold && scrobbledRef.current !== scrobbleKey) {
          scrobbledRef.current = scrobbleKey;
          recordLocalPlay(current);
          listenBrainz.submitListen('single', current).catch(() => {});
        }
    };

    audioElement.addEventListener('timeupdate', handleTimeUpdate);
    return () => audioElement.removeEventListener('timeupdate', handleTimeUpdate);
  }, [current, playerAudioElement]);

  useEffect(() => {
    if (!('mediaSession' in navigator) || !window.MediaMetadata) {
      return undefined;
    }
    if (!current) {
      navigator.mediaSession.metadata = null;
      return undefined;
    }

    navigator.mediaSession.metadata = new window.MediaMetadata({
      album: current.album || '',
      artist: current.artist || '',
      title: current.title || current.fileName || current.contentId,
    });

    const handlers = {
      nexttrack: next,
      pause,
      play: () => playAudio().catch(() => {}),
      previoustrack: previous,
      seekbackward: () => seekRelative(-15),
      seekforward: () => seekRelative(30),
    };

    Object.entries(handlers).forEach(([action, handler]) => {
      try {
        navigator.mediaSession.setActionHandler(action, handler);
      } catch {
        // Some browsers expose a partial Media Session implementation.
      }
    });

    return () => {
      Object.keys(handlers).forEach((action) => {
        try {
          navigator.mediaSession.setActionHandler(action, null);
        } catch {
          // Some browsers expose a partial Media Session implementation.
        }
      });
    };
  }, [current, next, pause, previous, seekRelative]);

  const audio = (
    <>
      <audio
        onLoadedMetadata={() => {
          if (audioRef.current && current?.positionSeconds > 0) {
            audioRef.current.currentTime = current.positionSeconds;
          }
        }}
        onEnded={next}
        onPause={() => setPlaying(false)}
        onPlay={() => setPlaying(true)}
        playsInline
        preload="metadata"
        ref={bindAudioElement}
        src={source || undefined}
      />
      <audio preload="metadata" ref={fadeAudioRef} />
    </>
  );
  const playerBadges = getPlayerBadges(current);

  if (collapsed) {
    return (
      <div
        aria-label="Audio player"
        className="player-bar player-bar-collapsed player-bar-modern"
        role="region"
        ref={playerBarRef}
      >
        {audio}
        <div className="player-track player-track-lcd">
          <Icon name="music" />
          <div>
            <div className="player-title">
              {current?.title || 'Player'}
            </div>
            <div className="player-subtitle">
              {current?.artist || 'Ready'}
            </div>
          </div>
        </div>
        <div className="player-controls player-control-cluster">
          <PlayerToolButton
            content="Expand the player drawer."
            aria-label="Expand player"
            data-testid="player-expand"
            icon="angle up"
            onClick={() => setCollapsed(false)}
          />
          <PlayerToolButton
            content={playing ? 'Pause the current stream.' : 'Resume the current stream.'}
            aria-label={playing ? 'Pause local playback' : 'Resume local playback'}
            data-testid="player-collapsed-toggle-playback"
            disabled={!current}
            icon={playing ? 'pause' : 'play'}
            onClick={togglePlayback}
          />
          <PlayerToolButton
            content={
              localMuted
                ? 'Unmute playback on this device without changing the stream.'
                : 'Mute playback on this device without changing the stream.'
            }
            aria-label={localMuted ? 'Unmute local playback' : 'Mute local playback'}
            data-testid="player-collapsed-toggle-mute"
            disabled={!current}
            icon={localMuted ? 'volume off' : 'volume up'}
            onClick={() => setLocalMuted((muted) => !muted)}
          />
        </div>
      </div>
    );
  }

  return (
    <div
      aria-label="Audio player"
      className="player-bar player-bar-modern"
      role="region"
      ref={playerBarRef}
    >
      {audio}
      <div className="player-main-deck">
        <div className="player-display">
          <PlayerVisualTile
            audioElement={playerAudioElement}
            runtimeProfile={runtimeProfile}
            current={current}
            mode={visualizerMode}
            onModeChange={setVisualizerMode}
            onTileModeChange={setVisualTileMode}
            tileMode={visualTileMode}
          />
          <div className="player-now-playing">
            <div className="player-track">
              <div>
                <div className="player-eyebrow">
                  {playing ? 'Now playing' : current ? 'Paused' : 'Ready'}
                </div>
                <div className="player-title">
                  {current?.title || 'Nothing playing'}
                </div>
                <div className="player-subtitle">
                  {current?.artist || 'Pick a collection or local audio file'}
                  {current?.album ? ` | ${current.album}` : ''}
                  {followingParty ? ` | Following ${followingParty.hostPeerId}` : ''}
                </div>
                {current ? (
                  <div className="player-now-playing-meta">
                    <div className="player-now-playing-badges">
                      {playerBadges.map((badge) => (
                        <Label
                          className="player-now-playing-badge"
                          color={badge.color}
                          data-testid={`player-badge-${badge.key}`}
                          key={badge.key}
                          size="mini"
                          title={badge.title}
                        >
                          <Icon name={badge.icon} />
                          {badge.text}
                        </Label>
                      ))}
                    </div>
                    <PlayerRatingControls
                      current={current}
                      onChange={updatePlayerRating}
                      rating={playerRating}
                    />
                  </div>
                ) : null}
              </div>
            </div>
            <div className="player-display-analyzers">
              <PlayerAnalyzerTile
                audioElement={current ? playerAudioElement : null}
                mode={analyzerMode}
                onModeChange={setAnalyzerMode}
              />
            </div>
          </div>
        </div>

        <div className="player-control-pad">
          <div className="player-control-row player-control-row-transport">
            <PlayerToolButton
              content="Go to the previous queue item, or restart the current stream."
              aria-label="Previous local track"
              data-testid="player-previous"
              disabled={!current}
              icon="step backward"
              onClick={previous}
            />
            <PlayerToolButton
              content="Rewind local playback by 15 seconds."
              aria-label="Rewind local playback"
              data-testid="player-rewind"
              disabled={!current}
              icon="backward"
              onClick={() => seekRelative(-15)}
            />
            <PlayerToolButton
              content={playing ? 'Pause the current stream.' : 'Resume the current stream.'}
              aria-label={playing ? 'Pause local playback' : 'Resume local playback'}
              className="player-play-button"
              data-testid="player-toggle-playback"
              disabled={!current}
              icon={playing ? 'pause' : 'play'}
              onClick={togglePlayback}
            />
            <PlayerToolButton
              content="Fast-forward local playback by 30 seconds."
              aria-label="Fast-forward local playback"
              data-testid="player-fast-forward"
              disabled={!current}
              icon="forward"
              onClick={() => seekRelative(30)}
            />
            <PlayerToolButton
              content="Play the next queue item."
              aria-label="Next local track"
              data-testid="player-next"
              disabled={!current || queue.length < 2}
              icon="step forward"
              onClick={next}
            />
            <PlayerToolButton
              content="Stop playback and clear your now-playing profile status."
              aria-label="Stop local playback"
              data-testid="player-stop"
              disabled={!current}
              icon="stop"
              onClick={clear}
            />
          </div>
          <div className="player-control-row">
            <PlayerLauncher
              compact
              onPlayItem={(item) => playItem(item, { replaceQueue: true })}
            />
            <PlayerToolButton
              active={queueOpen}
              content="Open the playback queue manager with current, upcoming, and recent session tracks."
              aria-label="Open playback queue"
              data-testid="player-open-queue"
              disabled={!current}
              icon="list ol"
              onClick={() => setQueueOpen(true)}
            />
            <PlayerToolButton
              active={localMuted}
              content={
                localMuted
                  ? 'Unmute playback on this device without changing the stream.'
                  : 'Mute playback on this device without changing the stream.'
              }
              aria-label={localMuted ? 'Unmute local playback' : 'Mute local playback'}
              data-testid="player-toggle-mute"
              disabled={!current}
              icon={localMuted ? 'volume off' : 'volume up'}
              onClick={() => setLocalMuted((muted) => !muted)}
            />
            <PlayerToolButton
              active={visualizerMode !== 'off'}
              content={
                visualizerMode === 'off'
                  ? `Show the ${runtimeProfile === 'native' ? 'MilkDrop3' : 'RustyMilk'} visualizer.`
                  : `Hide the ${runtimeProfile === 'native' ? 'MilkDrop3' : 'RustyMilk'} visualizer.`
              }
              aria-label={
                visualizerMode === 'off'
                  ? `Show ${runtimeProfile === 'native' ? 'MilkDrop3' : 'RustyMilk'} visualizer`
                  : `Hide ${runtimeProfile === 'native' ? 'MilkDrop3' : 'RustyMilk'} visualizer`
              }
              data-testid="player-toggle-visualizer"
              icon="eye"
              onClick={toggleVisualizer}
            />
            <PlayerToolButton
              active={eqPanelOpen}
              content={
                eqPanelOpen
                  ? 'Hide the equalizer panel.'
                  : 'Show the equalizer sliders and presets.'
              }
              aria-label={eqPanelOpen ? 'Hide equalizer' : 'Show equalizer'}
              data-testid="player-toggle-eq"
              icon="sliders horizontal"
              onClick={() => setEqPanelOpen((open) => !open)}
            />
            <PlayerToolButton
              active={lyricsOpen}
              content={
                lyricsOpen
                  ? 'Hide synced lyrics for the current track.'
                  : 'Fetch synced lyrics for the current artist and title from LRCLIB.'
              }
              aria-label={lyricsOpen ? 'Hide lyrics' : 'Show lyrics'}
              data-testid="player-toggle-lyrics"
              disabled={!current}
              icon="align left"
              onClick={() => setLyricsOpen((open) => !open)}
            />
            <PlayerToolButton
              content="Build smart-radio search seeds from the current track without starting network work yet."
              aria-label="Open smart radio seeds"
              data-testid="player-open-radio"
              disabled={!current}
              icon="random"
              onClick={() => setRadioOpen(true)}
            />
            <PlayerToolButton
              content="Show local listening stats recorded in this browser."
              aria-label="Open listening stats"
              data-testid="player-open-listening-stats"
              icon="bar chart"
              onClick={() => setStatsOpen(true)}
            />
            <PlayerToolButton
              active={shelfOpen}
              content="Open the browser-local discovery shelf built from player ratings."
              aria-label="Open discovery shelf"
              data-testid="player-open-discovery-shelf"
              icon="bookmark"
              onClick={() => setShelfOpen(true)}
            />
          </div>
          <div className="player-control-row">
            <PlayerToolButton
              content="Collapse the player into a small drawer bar above the footer."
              aria-label="Collapse player"
              data-testid="player-collapse"
              icon="angle down"
              onClick={() => setCollapsed(true)}
            />
            <PlayerToolButton
              active={karaokeEnabled}
              content={
                karaokeEnabled
                  ? 'Turn off center-channel vocal reduction.'
                  : 'Try center-channel vocal reduction for karaoke-style playback.'
              }
              aria-label={karaokeEnabled ? 'Disable karaoke mode' : 'Enable karaoke mode'}
              data-testid="player-toggle-karaoke"
              disabled={!current}
              icon="microphone slash"
              onClick={() => setKaraokeEnabledState((enabled) => !enabled)}
            />
            <PlayerToolButton
              active={crossfadeEnabled}
              content={
                crossfadeEnabled
                  ? 'Disable the five-second fade between queue items.'
                  : 'Enable a five-second fade between queue items.'
              }
              aria-label={crossfadeEnabled ? 'Disable crossfade' : 'Enable crossfade'}
              data-testid="player-toggle-crossfade"
              icon="exchange"
              onClick={() => setCrossfadeEnabled((enabled) => !enabled)}
            />
            <PlayerToolButton
              content="Open a tiny always-on-top spectrum window when this browser supports Document Picture-in-Picture."
              aria-label="Open visualizer picture in picture"
              data-testid="player-document-pip"
              disabled={!current || !window.documentPictureInPicture}
              icon="window restore"
              onClick={openPictureInPicture}
            />
            <PlayerToolButton
              active={listenBrainzToken.length > 0}
              content="Configure ListenBrainz scrobbling for this browser."
              aria-label="Configure ListenBrainz scrobbling"
              data-testid="player-open-integrations"
              icon="cloud upload"
              onClick={() => setIntegrationsOpen(true)}
            />
          </div>
        </div>
      </div>

      <div className="player-expanded-panels">
        {eqPanelOpen ? (
          <div className="player-panel player-panel-eq">
            <Equalizer audioElement={playerAudioElement} />
          </div>
        ) : null}
        <LyricsPane
          audioElement={playerAudioElement}
          current={current}
          visible={lyricsOpen}
        />
      </div>

      <PlayerIntegrationsModal
        externalVisualizerLaunching={externalVisualizerLaunching}
        externalVisualizerLoading={externalVisualizerLoading}
        externalVisualizerMessage={externalVisualizerMessage}
        externalVisualizerStatus={externalVisualizerStatus}
        listenBrainzToken={listenBrainzToken}
        onClearListenBrainzToken={() => {
          setListenBrainzTokenState('');
          listenBrainz.setListenBrainzToken('');
        }}
        onClose={() => setIntegrationsOpen(false)}
        onLaunchExternalVisualizer={launchExternalVisualizer}
        onListenBrainzTokenChange={(event) => {
          setListenBrainzTokenState(event.target.value);
          listenBrainz.setListenBrainzToken(event.target.value);
        }}
        onRefreshExternalVisualizerStatus={refreshExternalVisualizerStatus}
        open={integrationsOpen}
      />
      <PlayerRadioModal
        current={current}
        onClose={() => setRadioOpen(false)}
        onOpenSearch={openRadioSearch}
        open={radioOpen}
      />
      <PlayerQueueModal
        current={current}
        history={history}
        onAutoQueueSimilar={queueItems}
        onClearQueue={clearQueue}
        onClose={() => setQueueOpen(false)}
        onNext={next}
        onPrevious={previous}
        onRemove={removeFromQueue}
        open={queueOpen}
        queue={queue}
      />
      <PlayerDiscoveryShelfModal
        onClose={() => setShelfOpen(false)}
        open={shelfOpen}
      />
      <PlayerStatsModal
        onClose={() => setStatsOpen(false)}
        onOpenSearch={(query) => {
          setStatsOpen(false);
          openRadioSearch(query);
        }}
        open={statsOpen}
      />
      {current && queue.length > 1 ? (
        <div className="player-queue">
          {queue.slice(1, 4).map((item) => (
            <button
              className="player-queue-item"
              key={item.contentId}
              onClick={() => removeFromQueue(item.contentId)}
              title="Remove this item from the visible queue."
              type="button"
            >
              {item.title || item.fileName || item.contentId}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
};

export default PlayerBar;
