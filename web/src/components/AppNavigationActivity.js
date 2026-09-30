import * as chat from '../lib/chat';
import * as rooms from '../lib/rooms';
import { getLocalStorageItem, setLocalStorageItem } from '../lib/storage';
import { readBoundedJson, writeBoundedObject } from '../lib/persistedJson';

const ROOM_ACTIVITY_SEEN_STORAGE_KEY = 'slskr.rooms.lastSeenActivity';
const NAV_ACTIVITY_POLL_INTERVAL_MS = 10_000;
const MAX_ROOM_ACTIVITY_ROOMS = 500;
const MAX_ROOM_ACTIVITY_NAME_CHARACTERS = 2_048;
const MAX_ROOM_ACTIVITY_STORAGE_CHARACTERS = 64 * 1024;

const getStoredRoomActivity = () => {
  const stored = readBoundedJson(
    getLocalStorageItem,
    ROOM_ACTIVITY_SEEN_STORAGE_KEY,
    {},
    MAX_ROOM_ACTIVITY_STORAGE_CHARACTERS,
  );
  if (!stored || typeof stored !== 'object' || Array.isArray(stored)) return {};

  return Object.fromEntries(
    Object.entries(stored)
      .map(([roomName, timestamp]) => [
        typeof roomName === 'string'
          ? roomName.trim().slice(0, MAX_ROOM_ACTIVITY_NAME_CHARACTERS)
          : '',
        Number(timestamp),
      ])
      .filter(([roomName, timestamp]) => roomName && Number.isFinite(timestamp) && timestamp > 0)
      .slice(-MAX_ROOM_ACTIVITY_ROOMS),
  );
};

const storeRoomActivity = (activity) => {
  const normalized = Object.fromEntries(
    Object.entries(activity && typeof activity === 'object' ? activity : {})
      .map(([roomName, timestamp]) => [
        typeof roomName === 'string'
          ? roomName.trim().slice(0, MAX_ROOM_ACTIVITY_NAME_CHARACTERS)
          : '',
        Number(timestamp),
      ])
      .filter(([roomName, timestamp]) => roomName && Number.isFinite(timestamp) && timestamp > 0)
      .slice(-MAX_ROOM_ACTIVITY_ROOMS),
  );

  writeBoundedObject(
    setLocalStorageItem,
    ROOM_ACTIVITY_SEEN_STORAGE_KEY,
    normalized,
    {
      maxCharacters: MAX_ROOM_ACTIVITY_STORAGE_CHARACTERS,
      maxEntries: MAX_ROOM_ACTIVITY_ROOMS,
    },
  );
};

export default class AppNavigationActivity {
  constructor({ getCurrentPath, isAuthenticated, onActivityChange, runtimeProfileHint }) {
    this.getCurrentPath = getCurrentPath;
    this.isAuthenticated = isAuthenticated;
    this.onActivityChange = onActivityChange;
    this.runtimeProfileHint = runtimeProfileHint;
    this.roomActivityBaselined = false;
    this.started = false;
    this.running = false;
    this.requestId = 0;
    this.timer = undefined;
  }

  start = () => {
    if (this.started) return;
    this.started = true;
    if (document.visibilityState !== 'hidden') {
      void this.refresh();
    }
    this.schedule();
  };

  stop = () => {
    this.started = false;
    this.requestId += 1;
    if (this.timer !== undefined) {
      window.clearTimeout(this.timer);
      this.timer = undefined;
    }
  };

  schedule = () => {
    if (
      !this.started ||
      document.visibilityState === 'hidden' ||
      this.timer !== undefined
    ) {
      return;
    }
    this.timer = window.setTimeout(async () => {
      this.timer = undefined;
      await this.refresh();
      this.schedule();
    }, NAV_ACTIVITY_POLL_INTERVAL_MS);
  };

  handleVisibilityChange = () => {
    if (document.visibilityState === 'hidden') {
      if (this.timer !== undefined) {
        window.clearTimeout(this.timer);
        this.timer = undefined;
      }
      return;
    }

    void this.refresh();
    this.schedule();
  };

  getChatActivity = async () => {
    const currentPath = this.getCurrentPath();
    if (currentPath.startsWith('/chat') || currentPath.startsWith('/messages')) {
      return false;
    }

    return chat.hasUnAcknowledgedMessages();
  };

  getRoomsActivity = async () => {
    const currentPath = this.getCurrentPath();
    if (currentPath.startsWith('/rooms') || currentPath.startsWith('/messages')) {
      this.roomActivityBaselined = true;
      return false;
    }

    const latestByRoom = await rooms.getActivity();
    const seenActivity = getStoredRoomActivity();
    if (!this.roomActivityBaselined && Object.keys(seenActivity).length === 0) {
      storeRoomActivity(latestByRoom);
      this.roomActivityBaselined = true;
      return false;
    }

    this.roomActivityBaselined = true;
    return Object.entries(latestByRoom).some(
      ([roomName, latest]) => latest > (seenActivity[roomName] || 0),
    );
  };

  refresh = async () => {
    if (!this.started || this.running) return;
    this.running = true;
    const requestId = ++this.requestId;
    const isCurrent = () => this.started && requestId === this.requestId;

    if (['legacy', 'native'].includes(this.runtimeProfileHint)) {
      if (isCurrent()) {
        this.onActivityChange({ chat: false, rooms: false });
      }
      this.running = false;
      return;
    }

    if (!this.isAuthenticated()) {
      if (isCurrent()) {
        this.onActivityChange({ chat: false, rooms: false });
      }
      this.running = false;
      return;
    }

    try {
      const [chatActivity, roomsActivity] = await Promise.all([
        this.getChatActivity(),
        this.getRoomsActivity(),
      ]);

      if (isCurrent() && this.isAuthenticated()) {
        this.onActivityChange({ chat: chatActivity, rooms: roomsActivity });
      }
    } catch (error) {
      console.error('Failed to refresh navigation activity:', error);
    } finally {
      this.running = false;
    }
  };
}
