import * as nowPlaying from '../../lib/nowPlaying';
import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react';

export const PlayerContext = createContext({
  clearQueue: () => {},
  clear: () => {},
  current: null,
  followParty: () => {},
  followingParty: null,
  history: [],
  next: () => {},
  pause: () => {},
  playItem: () => {},
  previous: () => {},
  queue: [],
  queueItems: () => {},
  removeFromQueue: () => {},
  seekRelative: () => {},
  setAudioElement: () => {},
});

export const PlayerProvider = ({ children }) => {
  const [audioElement, setAudioElement] = useState(null);
  const [current, setCurrent] = useState(null);
  const [history, setHistory] = useState([]);
  const [queue, setQueue] = useState([]);
  const [followingParty, setFollowingParty] = useState(null);
  const playTimeoutRef = useRef(null);

  useEffect(() => () => {
    if (playTimeoutRef.current !== null) {
      window.clearTimeout(playTimeoutRef.current);
      playTimeoutRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (!current?.artist || !current?.title) return;
    void nowPlaying
      .setNowPlaying({
        album: current.album,
        artist: current.artist,
        title: current.title,
      })
      .catch((error) => {
        console.debug('[Player] Unable to update now-playing state:', error);
      });
  }, [current]);

  const playItem = useCallback(
    (item, options = {}) => {
      if (!item?.contentId) return;

      const playable = {
        album: item.album || item.collectionTitle || '',
        artist: item.artist || item.username || 'slskr',
        artworkUrl: item.artworkUrl || item.coverUrl || item.imageUrl || '',
        confidence: item.confidence || item.matchConfidence || item.score || 0,
        contentId: item.contentId,
        fileName: item.fileName || item.title || item.contentId,
        genre: item.genre || '',
        positionSeconds: options.positionSeconds || 0,
        sourceProviders: item.sourceProviders || item.providers || [],
        streamUrl: item.streamUrl || options.streamUrl || '',
        tags: item.tags || item.genres || [],
        title: item.title || item.fileName || item.contentId,
        verified: Boolean(
          item.verified ||
          item.verifiedAt ||
          item.fingerprint?.verifiedAt ||
          item.verification?.verified,
        ),
      };

      setHistory((existing) => (current ? [current, ...existing].slice(0, 25) : existing));
      setCurrent(playable);
      setQueue((existing) =>
        options.replaceQueue ? [playable] : [playable, ...existing],
      );

      if (playTimeoutRef.current !== null) {
        window.clearTimeout(playTimeoutRef.current);
      }
      const scheduledAudioElement = audioElement;
      playTimeoutRef.current = window.setTimeout(() => {
        playTimeoutRef.current = null;
        scheduledAudioElement?.play().catch(() => {});
      }, 0);
    },
    [audioElement, current],
  );

  const pause = useCallback(() => {
    if (audioElement) {
      audioElement.pause();
    }
  }, [audioElement]);

  const clear = useCallback(async () => {
    if (playTimeoutRef.current !== null) {
      window.clearTimeout(playTimeoutRef.current);
      playTimeoutRef.current = null;
    }
    if (audioElement) {
      audioElement.pause();
      audioElement.removeAttribute('src');
      audioElement.load();
    }

    setCurrent(null);
    setHistory([]);
    setQueue([]);
    try {
      await nowPlaying.clearNowPlaying();
    } catch (error) {
      console.debug('[Player] Unable to clear now-playing state:', error);
    }
  }, [audioElement]);

  const clearQueue = useCallback(() => {
    setQueue((existing) => (current ? [current] : existing.slice(0, 1)));
  }, [current]);

  const queueItems = useCallback((items = []) => {
    const nextItems = Array.isArray(items) ? items : [];
    setQueue((existing) => {
      const queuedIds = new Set(existing.map((item) => item.contentId));
      const additions = nextItems.filter((item) => {
        if (!item?.contentId || queuedIds.has(item.contentId)) return false;
        queuedIds.add(item.contentId);
        return true;
      });

      return additions.length > 0 ? [...existing, ...additions] : existing;
    });
  }, []);

  const removeFromQueue = useCallback((contentId) => {
    setQueue((existing) =>
      existing.filter((item, index) => index === 0 || item.contentId !== contentId),
    );
  }, []);

  const followParty = useCallback((partyState) => {
    setFollowingParty(partyState);
  }, []);

  const seekRelative = useCallback(
    (seconds) => {
      if (!audioElement) return;
      const duration = Number.isFinite(audioElement.duration)
        ? audioElement.duration
        : Number.MAX_SAFE_INTEGER;
      audioElement.currentTime = Math.max(
        0,
        Math.min(duration, audioElement.currentTime + seconds),
      );
    },
    [audioElement],
  );

  const next = useCallback(() => {
    if (queue.length < 2) return;
    const [, nextItem, ...remaining] = queue;
    setHistory((previousHistory) =>
      current ? [current, ...previousHistory].slice(0, 25) : previousHistory,
    );
    setCurrent(nextItem);
    setQueue([nextItem, ...remaining]);
  }, [current, queue]);

  const previous = useCallback(() => {
    if (history.length === 0) {
      if (audioElement) {
        audioElement.currentTime = 0;
      }
      return;
    }

    const [previousItem, ...remainingHistory] = history;
    setHistory(remainingHistory);
    setCurrent(previousItem);
    setQueue((existing) => [previousItem, ...existing]);
  }, [audioElement, history]);

  return (
    <PlayerContext.Provider
      value={{
        clearQueue,
        clear,
        current,
        followParty,
        followingParty,
        history,
        next,
        pause,
        playItem,
        previous,
        queue,
        queueItems,
        removeFromQueue,
        seekRelative,
        setAudioElement,
      }}
    >
      {children}
    </PlayerContext.Provider>
  );
};

export const usePlayer = () => useContext(PlayerContext);
