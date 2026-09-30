import { useCallback, useEffect, useMemo, useRef } from 'react';
import { useMountedRef } from '../../../lib/useMountedRef';
import { Icon, Label } from 'semantic-ui-react';

export const getOption = (source, ...keys) => {
  for (const key of keys) {
    if (source && Object.prototype.hasOwnProperty.call(source, key)) {
      return source[key];
    }
  }

  return undefined;
};

export const getIntegrationsOptions = (options = {}) =>
  getOption(options, 'integration', 'Integration', 'integrations', 'Integrations') ||
  {};

export const getVpnOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'vpn', 'Vpn', 'VPN') || {};

export const getLidarrOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'lidarr', 'Lidarr') || {};

export const getSpotifyOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'spotify', 'Spotify') || {};

export const getYouTubeOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'youtube', 'YouTube') || {};

export const getLastFmOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'lastfm', 'lastFm', 'LastFm') || {};

export const getPushbulletOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'pushbullet', 'Pushbullet') || {};

export const getNtfyOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'ntfy', 'Ntfy') || {};

export const getPushoverOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'pushover', 'Pushover') || {};

export const getFtpOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'ftp', 'Ftp', 'FTP') || {};

export const getChromaprintOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'chromaprint', 'Chromaprint') || {};

export const getAcoustIdOptions = (options = {}) =>
  getOption(getIntegrationsOptions(options), 'acoustId', 'acoustid', 'AcoustId') ||
  {};

export const getMusicBrainzOptions = (options = {}) =>
  getOption(
    getIntegrationsOptions(options),
    'musicBrainz',
    'musicbrainz',
    'MusicBrainz',
  ) || {};

export const getVpnState = (state = {}) => getOption(state, 'vpn', 'Vpn', 'VPN') || {};

export const boolLabel = (value, trueText = 'Enabled', falseText = 'Disabled') => (
  <Label color={value ? 'green' : 'grey'}>
    <Icon name={value ? 'check circle' : 'minus circle'} />
    {value ? trueText : falseText}
  </Label>
);

export const valueOrDash = (value) =>
  value === undefined || value === null || value === '' ? '-' : value;

export const formatBytes = (value) => {
  if (!Number.isFinite(value)) return '-';
  if (value < 1024) return `${value} B`;
  const units = ['KiB', 'MiB', 'GiB', 'TiB'];
  let size = value / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && size >= 1024; index += 1) {
    size /= 1024;
    unit = units[index];
  }
  return `${size.toFixed(1)} ${unit}`;
};

export const isConfigured = (value) =>
  value !== undefined && value !== null && value !== '';

export const toNumber = (value, fallback) => {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
};

export const useAsyncGuard = () => {
  const mountedRef = useMountedRef();
  const requestIdRef = useRef(0);
  const inFlightRef = useRef(false);

  useEffect(() => () => {
    requestIdRef.current += 1;
    inFlightRef.current = false;
  }, []);

  const begin = useCallback(() => {
    if (!mountedRef.current || inFlightRef.current) return null;
    inFlightRef.current = true;
    return ++requestIdRef.current;
  }, [mountedRef]);

  const isCurrent = useCallback(
    (requestId) =>
      mountedRef.current && requestId === requestIdRef.current,
    [mountedRef],
  );

  const finish = useCallback((requestId) => {
    if (requestId === requestIdRef.current) inFlightRef.current = false;
  }, []);

  return useMemo(() => ({ begin, finish, isCurrent }), [
    begin,
    finish,
    isCurrent,
  ]);
};

export const portForwards = (vpn = {}) =>
  getOption(vpn, 'portForwards', 'PortForwards') || [];

export const buildSourceFeedForm = (options = {}) => {
  const spotify = getSpotifyOptions(options);
  const youtube = getYouTubeOptions(options);
  const lastfm = getLastFmOptions(options);

  return {
    lastFmApiKey: '',
    lastFmConfigured: isConfigured(getOption(lastfm, 'apiKey', 'ApiKey')),
    lastFmEnabled: Boolean(getOption(lastfm, 'enabled', 'Enabled')),
    spotifyClientId: '',
    spotifyClientSecret: '',
    spotifyConfigured: isConfigured(getOption(spotify, 'clientId', 'ClientId')),
    spotifyEnabled: Boolean(getOption(spotify, 'enabled', 'Enabled')),
    spotifyMaxItems: String(
      getOption(spotify, 'maxItemsPerImport', 'MaxItemsPerImport') ?? 500,
    ),
    spotifyMarket: getOption(spotify, 'market', 'Market') || 'US',
    spotifyRedirectUri: getOption(spotify, 'redirectUri', 'RedirectUri') || '',
    spotifySecretConfigured: isConfigured(
      getOption(spotify, 'clientSecret', 'ClientSecret'),
    ),
    spotifyTimeout: String(getOption(spotify, 'timeoutSeconds', 'TimeoutSeconds') ?? 20),
    youTubeApiKey: '',
    youTubeConfigured: isConfigured(getOption(youtube, 'apiKey', 'ApiKey')),
    youTubeEnabled: Boolean(getOption(youtube, 'enabled', 'Enabled')),
  };
};

export const buildNotificationForm = (options = {}) => {
  const pushbullet = getPushbulletOptions(options);
  const ntfy = getNtfyOptions(options);
  const pushover = getPushoverOptions(options);

  return {
    ntfyAccessToken: '',
    ntfyAccessTokenConfigured: isConfigured(
      getOption(ntfy, 'accessToken', 'AccessToken'),
    ),
    ntfyEnabled: Boolean(getOption(ntfy, 'enabled', 'Enabled')),
    ntfyNotifyOnPrivateMessage:
      getOption(ntfy, 'notifyOnPrivateMessage', 'NotifyOnPrivateMessage') ?? true,
    ntfyNotifyOnRoomMention:
      getOption(ntfy, 'notifyOnRoomMention', 'NotifyOnRoomMention') ?? true,
    ntfyPrefix: getOption(ntfy, 'notificationPrefix', 'NotificationPrefix') || 'slskr',
    ntfyUrl: getOption(ntfy, 'url', 'Url') || '',
    pushbulletAccessToken: '',
    pushbulletAccessTokenConfigured: isConfigured(
      getOption(pushbullet, 'accessToken', 'AccessToken'),
    ),
    pushbulletCooldownTime: String(
      getOption(pushbullet, 'cooldownTime', 'CooldownTime') ?? 900000,
    ),
    pushbulletEnabled: Boolean(getOption(pushbullet, 'enabled', 'Enabled')),
    pushbulletNotifyOnPrivateMessage:
      getOption(
        pushbullet,
        'notifyOnPrivateMessage',
        'NotifyOnPrivateMessage',
      ) ?? true,
    pushbulletNotifyOnRoomMention:
      getOption(pushbullet, 'notifyOnRoomMention', 'NotifyOnRoomMention') ?? true,
    pushbulletPrefix:
      getOption(pushbullet, 'notificationPrefix', 'NotificationPrefix') ||
      'From slskr:',
    pushbulletRetryAttempts: String(
      getOption(pushbullet, 'retryAttempts', 'RetryAttempts') ?? 3,
    ),
    pushoverEnabled: Boolean(getOption(pushover, 'enabled', 'Enabled')),
    pushoverNotifyOnPrivateMessage:
      getOption(
        pushover,
        'notifyOnPrivateMessage',
        'NotifyOnPrivateMessage',
      ) ?? true,
    pushoverNotifyOnRoomMention:
      getOption(pushover, 'notifyOnRoomMention', 'NotifyOnRoomMention') ?? true,
    pushoverPrefix:
      getOption(pushover, 'notificationPrefix', 'NotificationPrefix') || 'slskr',
    pushoverToken: '',
    pushoverTokenConfigured: isConfigured(getOption(pushover, 'token', 'Token')),
    pushoverUserKey: '',
    pushoverUserKeyConfigured: isConfigured(
      getOption(pushover, 'userKey', 'UserKey'),
    ),
  };
};

