import {
  getLocalStorageItem,
  setLocalStorageItem,
} from './storage';
import {
  maxPersistedJsonCharacters,
  readBoundedJson,
  writeBoundedObject,
} from './persistedJson';

export const playerRatingsStorageKey = 'slskr.player.ratings';

const maxPlayerRatings = 2_000;
const maxRatingKeyCharacters = 2_048;

const normalizeText = (value = '') =>
  String(value).trim().slice(0, maxRatingKeyCharacters).toLowerCase();

const readRatings = () => {
  const parsed = readBoundedJson(
    getLocalStorageItem,
    playerRatingsStorageKey,
    {},
    maxPersistedJsonCharacters,
  );
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {};

  return Object.fromEntries(
    Object.entries(parsed)
      .filter(
        ([key, value]) =>
          typeof key === 'string' &&
          key.length <= maxRatingKeyCharacters &&
          Number.isInteger(Number(value)) &&
          Number(value) >= 1 &&
          Number(value) <= 5,
      )
      .slice(-maxPlayerRatings),
  );
};

const writeRatings = (ratings) => {
  return writeBoundedObject(
    setLocalStorageItem,
    playerRatingsStorageKey,
    ratings,
    {
      maxCharacters: maxPersistedJsonCharacters,
      maxEntries: maxPlayerRatings,
    },
  );
};

export const getPlayerRatingKey = (track = {}) => {
  if (!track) return '';
  if (track.contentId) return `content:${normalizeText(track.contentId)}`;
  if (track.streamUrl) return `stream:${normalizeText(track.streamUrl)}`;

  const parts = [
    normalizeText(track.artist),
    normalizeText(track.album),
    normalizeText(track.title || track.fileName),
  ].filter(Boolean);

  return parts.length > 0 ? `meta:${parts.join('|')}` : '';
};

export const getPlayerRating = (track) => {
  const key = getPlayerRatingKey(track);
  if (!key) return 0;

  const rating = Number(readRatings()[key] || 0);
  return Number.isInteger(rating) && rating >= 1 && rating <= 5 ? rating : 0;
};

export const setPlayerRating = (track, rating) => {
  const key = getPlayerRatingKey(track);
  if (!key) return getPlayerRating(track);

  const nextRating = Number(rating);
  const ratings = readRatings();

  if (!Number.isInteger(nextRating) || nextRating < 1 || nextRating > 5) {
    delete ratings[key];
  } else {
    ratings[key] = nextRating;
  }

  writeRatings(ratings);
  return getPlayerRating(track);
};

export const getPlayerRatingSummary = (track) => {
  const rating = getPlayerRating(track);

  if (rating >= 4) {
    return {
      label: 'Discovery boost',
      rating,
      tone: 'positive',
    };
  }

  if (rating > 0 && rating <= 2) {
    return {
      label: 'Discovery caution',
      rating,
      tone: 'negative',
    };
  }

  if (rating === 3) {
    return {
      label: 'Neutral rating',
      rating,
      tone: 'neutral',
    };
  }

  return {
    label: 'Not rated',
    rating: 0,
    tone: 'unrated',
  };
};
