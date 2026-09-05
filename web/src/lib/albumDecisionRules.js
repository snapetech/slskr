// <copyright file="albumDecisionRules.js" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import { getLocalStorageItem, setLocalStorageItem } from './storage';
import {
  maxPersistedJsonCharacters,
  readBoundedJson,
  writeBoundedList,
} from './persistedJson';

const STORAGE_KEY = 'slskr.albumDecisionRules';
const MAX_RULES = 50;
const MAX_RULE_TEXT_CHARACTERS = 2_048;
const MAX_RULE_COMPONENTS = 64;

const limitText = (value = '') => String(value).trim().slice(0, MAX_RULE_TEXT_CHARACTERS);

const normalizeText = (value = '') =>
  limitText(value)
    .toLowerCase()
    .replace(/[^\d a-z]+/gu, ' ')
    .replace(/\s+/gu, ' ')
    .trim();

const asArray = (value) => (Array.isArray(value) ? value : []);
const isPlainObject = (value) =>
  value && typeof value === 'object' && !Array.isArray(value);

const normalizeRule = (rule = {}) => {
  if (!isPlainObject(rule)) return null;

  const expectedTrackCount = Number(rule.expectedTrackCount);
  const minCompleteness = Number(rule.minCompleteness);
  const sourceCount = Number(rule.sourceCount);
  const warningCount = Number(rule.warningCount);

  return {
    albumKey: normalizeText(rule.albumKey),
    albumTitle: limitText(rule.albumTitle),
    createdAt: limitText(rule.createdAt),
    expectedTrackCount: Number.isFinite(expectedTrackCount)
      ? Math.max(0, Math.floor(expectedTrackCount))
      : 0,
    formatPolicy: limitText(rule.formatPolicy),
    id: limitText(rule.id),
    minCompleteness: Number.isFinite(minCompleteness)
      ? Math.min(Math.max(minCompleteness, 0), 1)
      : 0,
    notes: asArray(rule.notes)
      .filter((note) => typeof note === 'string' || typeof note === 'number')
      .map(limitText)
      .slice(0, MAX_RULE_COMPONENTS),
    searchKey: normalizeText(rule.searchKey),
    sourceCount: Number.isFinite(sourceCount)
      ? Math.max(0, Math.floor(sourceCount))
      : 0,
    substitutionTracks: asArray(rule.substitutionTracks)
      .filter((trackNumber) => Number.isFinite(Number(trackNumber)))
      .map((trackNumber) => Math.max(0, Math.floor(Number(trackNumber))))
      .slice(0, MAX_RULE_COMPONENTS),
    warningCount: Number.isFinite(warningCount)
      ? Math.max(0, Math.floor(warningCount))
      : 0,
  };
};

const parseRules = () => {
  const parsed = readBoundedJson(
    getLocalStorageItem,
    STORAGE_KEY,
    [],
    maxPersistedJsonCharacters,
  );

  return Array.isArray(parsed)
    ? parsed
        .slice(0, MAX_RULES)
        .map(normalizeRule)
        .filter(Boolean)
    : [];
};

export const getAlbumDecisionRules = () => parseRules();

export const buildAlbumDecisionRule = ({
  candidate,
  createdAt = new Date().toISOString(),
  searchText = '',
} = {}) => {
  const albumKey = normalizeText(candidate?.albumTitle || searchText);
  const searchKey = normalizeText(searchText);
  const formatMix = asArray(candidate?.formatMix)
    .filter(isPlainObject)
    .slice(0, MAX_RULE_COMPONENTS);
  const substitutionOptions = asArray(candidate?.substitutionOptions).filter(
    isPlainObject,
  ).slice(0, MAX_RULE_COMPONENTS);
  const warnings = asArray(candidate?.warnings).slice(0, MAX_RULE_COMPONENTS);
  const formatPolicy = formatMix
    .map((item) => `${limitText(item.format)}:${limitText(item.count)}`)
    .join(',');
  const expectedTrackCount = Number(candidate?.expectedTrackCount);

  return {
    albumKey,
    albumTitle: limitText(candidate?.albumTitle || searchText),
    createdAt: limitText(createdAt),
    expectedTrackCount: Number.isFinite(expectedTrackCount)
      ? Math.max(0, Math.floor(expectedTrackCount))
      : 0,
    formatPolicy: limitText(formatPolicy),
    id: limitText(
      `${albumKey || searchKey}:${Number.isFinite(expectedTrackCount)
        ? Math.max(0, Math.floor(expectedTrackCount))
        : 0}:${formatPolicy}`,
    ),
    minCompleteness: Number.isFinite(Number(candidate?.completenessRatio))
      ? Math.min(Math.max(Number(candidate.completenessRatio), 0), 1)
      : 0,
    notes: [
      ...warnings.map((warning) => `warn:${limitText(warning)}`),
      ...substitutionOptions.map(
        (option) =>
          `substitute:track-${limitText(option.trackNumber)}:${limitText(option.optionCount)}-options`,
      ),
    ].map(limitText),
    searchKey,
    sourceCount: Number.isFinite(Number(candidate?.sourceCount))
      ? Math.max(0, Math.floor(Number(candidate.sourceCount)))
      : 0,
    substitutionTracks: substitutionOptions.map(
      (option) => Math.max(0, Math.floor(Number(option.trackNumber) || 0)),
    ).slice(0, MAX_RULE_COMPONENTS),
    warningCount: warnings.length,
  };
};

export const saveAlbumDecisionRule = ({ candidate, searchText } = {}) => {
  const rule = buildAlbumDecisionRule({ candidate, searchText });
  const existing = parseRules().filter((item) => item.id !== rule.id);
  const rules = [rule, ...existing].slice(0, MAX_RULES);

  writeBoundedList(setLocalStorageItem, STORAGE_KEY, rules, {
    maxCharacters: maxPersistedJsonCharacters,
    maxItems: MAX_RULES,
  });

  return {
    rule,
    rules,
  };
};
