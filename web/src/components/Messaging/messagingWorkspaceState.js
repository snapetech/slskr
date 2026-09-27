import { readBoundedJson } from '../../lib/persistedJson';
import { getLocalStorageItem, setLocalStorageItem } from '../../lib/storage';

const STORAGE_KEY = 'slskr-messaging-workspace';
const GOLD_STAR_CLUB_POD_ID = 'pod:901d57a2c1bb4e5d90d57a2c1bb4e5d0';
const MAX_PANELS = 64;
const MAX_PANEL_TEXT_CHARACTERS = 2_048;
const MAX_PANEL_STORAGE_CHARACTERS = 128 * 1024;
const MAX_PANEL_COUNTER = 1_000_000_000;
const MAX_POD_MESSAGES = 100;

let panelCounter = 0;

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (record) => record && typeof record === 'object' && !Array.isArray(record),
  );

const normalizePanel = (panel) => {
  if (!panel || typeof panel !== 'object' || Array.isArray(panel)) return null;
  const type = ['chat', 'room', 'pod'].includes(panel.type) ? panel.type : null;
  const target = `${panel.target ?? ''}`.trim().slice(0, MAX_PANEL_TEXT_CHARACTERS);
  const label = `${panel.label ?? ''}`.trim().slice(0, MAX_PANEL_TEXT_CHARACTERS);
  const id = `${panel.id || `${type || 'panel'}-${target}`}`
    .trim()
    .slice(0, MAX_PANEL_TEXT_CHARACTERS);
  if (!type || !target) return null;
  return {
    collapsed: Boolean(panel.collapsed),
    ...(label ? { label } : {}),
    id: id || `${type}-${target}`,
    target,
    type,
  };
};

const loadPanels = () => {
  const parsed = readBoundedJson(
    getLocalStorageItem,
    STORAGE_KEY,
    {},
    MAX_PANEL_STORAGE_CHARACTERS,
  );
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    panelCounter = 0;
    return [];
  }

  panelCounter = Number.isSafeInteger(parsed.panelCounter) && parsed.panelCounter >= 0
    ? Math.min(parsed.panelCounter, MAX_PANEL_COUNTER)
    : 0;
  return Array.isArray(parsed.panels)
    ? parsed.panels
        .slice(-MAX_PANELS)
        .map(normalizePanel)
        .filter(Boolean)
    : [];
};

const savePanels = (panels) => {
  const normalized = (Array.isArray(panels) ? panels : [])
    .map(normalizePanel)
    .filter(Boolean)
    .slice(-MAX_PANELS);
  let serialized = JSON.stringify({
    panelCounter: Math.min(Math.max(panelCounter, 0), MAX_PANEL_COUNTER),
    panels: normalized,
  });

  while (serialized.length > MAX_PANEL_STORAGE_CHARACTERS && normalized.length > 0) {
    normalized.shift();
    serialized = JSON.stringify({
      panelCounter: Math.min(Math.max(panelCounter, 0), MAX_PANEL_COUNTER),
      panels: normalized,
    });
  }

  setLocalStorageItem(STORAGE_KEY, serialized);
};

const makePanel = (type, target, collapsed = false) => {
  panelCounter = panelCounter >= MAX_PANEL_COUNTER ? 1 : panelCounter + 1;
  return {
    collapsed,
    id: `${type}-${panelCounter}`,
    target,
    type,
  };
};

const encodePodTarget = (podId, channelId) => `${podId}\u001f${channelId}`;

const decodePodTarget = (target) => {
  const [podId, channelId] = `${target || ''}`.split('\u001f');
  return { channelId, podId };
};

const channelLabel = (channel) =>
  [channel.podName, channel.channelName || channel.channelId]
    .filter(Boolean)
    .join(' / ');

const normalizeConversationName = (value) => `${value || ''}`.trim().toLowerCase();

const isPodDirectChannel = (channel) => {
  const channelKind = normalizeConversationName(channel.channelKind);
  const channelName = normalizeConversationName(
    channel.channelName || channel.channelId,
  );

  return (
    channelKind === 'direct' ||
    channelName === 'dm' ||
    channelName === 'direct' ||
    channelName === 'direct message'
  );
};

const panelLabel = (panel) => {
  if (panel.type === 'room') return `#${panel.target}`;
  if (panel.type === 'pod') return panel.label || 'Pod channel';

  return panel.target;
};


export {
  GOLD_STAR_CLUB_POD_ID,
  MAX_PANELS,
  asRecords,
  normalizePanel,
  loadPanels,
  savePanels,
  makePanel,
  encodePodTarget,
  decodePodTarget,
  channelLabel,
  normalizeConversationName,
  isPodDirectChannel,
  panelLabel,
  MAX_POD_MESSAGES,
};
