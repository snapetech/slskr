import 'react-toastify/dist/ReactToastify.css';
import './App.css';
import * as chat from '../lib/chat';
import * as collectionsAPI from '../lib/collections';
import { createApplicationHubConnection } from '../lib/hubFactory';
import { getState as getApplicationState } from '../lib/application';
import { getCurrent as getApplicationOptions } from '../lib/options';
import * as relayAPI from '../lib/relay';
import * as rooms from '../lib/rooms';
import { connect, disconnect } from '../lib/server';
import * as session from '../lib/session';
import { getLocalStorageItem, setLocalStorageItem } from '../lib/storage';
import {
  readBoundedJson,
  writeBoundedObject,
} from '../lib/persistedJson';
import { isPassthroughEnabled } from '../lib/token';
import AppContext from './AppContext';
import LoginForm from './LoginForm';
import PlayerBar from './Player/PlayerBar';
import { PlayerProvider } from './Player/PlayerContext';
import ErrorSegment from './Shared/ErrorSegment';
import Footer from './Shared/Footer';
import React, { Component, lazy, Suspense, useEffect } from 'react';
import { NavLink, Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { ToastContainer } from 'react-toastify';
import {
  Button,
  Dropdown,
  Form,
  Header,
  Icon,
  Loader,
  Menu,
  Modal,
  Popup,
  Segment,
  Sidebar,
} from 'semantic-ui-react';

const SLSKR_RELEASES_URL = 'https://github.com/snapetech/slskr/releases';
const NETWORK_ENDPOINT_NOTICE_STORAGE_KEY =
  'slskr.networkEndpoints.v2.dismissedSignature';
const NETWORK_ENDPOINT_SNAPSHOT_STORAGE_KEY =
  'slskr.networkEndpoints.v2.lastDismissedSnapshot';
const LEGACY_NETWORK_ENDPOINT_SNAPSHOT_STORAGE_KEY =
  'slskr.networkEndpoints.lastDismissedSnapshot';
const LEGACY_VPN_PORT_NOTICE_STORAGE_KEY =
  'slskr.vpnForwardedPorts.dismissedSignature';
const ROOM_ACTIVITY_SEEN_STORAGE_KEY = 'slskr.rooms.lastSeenActivity';
const NAV_ACTIVITY_POLL_INTERVAL_MS = 10_000;
const MAX_ROOM_ACTIVITY_ROOMS = 500;
const MAX_ROOM_ACTIVITY_NAME_CHARACTERS = 2_048;
const MAX_ROOM_ACTIVITY_STORAGE_CHARACTERS = 64 * 1024;
const MAX_NETWORK_ENDPOINT_FORWARDS = 32;
const MAX_NETWORK_ENDPOINT_TEXT_CHARACTERS = 256;
const MAX_NETWORK_ENDPOINT_SIGNATURE_CHARACTERS = 64 * 1024;
const MAX_NETWORK_ENDPOINT_STORAGE_CHARACTERS = 64 * 1024;

const Browse = lazy(() => import('./Browse/Browse'));
const Chat = lazy(() => import('./Chat/Chat'));
const Collections = lazy(() => import('./Collections/Collections'));
const Contacts = lazy(() => import('./Contacts/Contacts'));
const CompatibilityDashboard = lazy(() => import('./CompatibilityDashboard'));
const DiscoveryGraphAtlasPage = lazy(() =>
  import('./Search/DiscoveryGraphAtlasPage'));
const Messaging = lazy(() => import('./Messaging/Messaging'));
const PlaylistIntake = lazy(() => import('./PlaylistIntake/PlaylistIntake'));
const Rooms = lazy(() => import('./Rooms/Rooms'));
const Searches = lazy(() => import('./Search/Searches'));
const ShareGroups = lazy(() => import('./ShareGroups/ShareGroups'));
const SharedWithMe = lazy(() => import('./Shares/SharedWithMe'));
const SolidSettings = lazy(() => import('./Solid/SolidSettings'));
const System = lazy(() => import('./System/System'));
const Transfers = lazy(() => import('./Transfers/Transfers'));
const Users = lazy(() => import('./Users/Users'));
const Wishlist = lazy(() => import('./Wishlist/Wishlist'));

const THEME_OPTIONS = [
  { key: 'slskr', text: 'slskr', value: 'slskr' },
  { key: 'classic-dark', text: 'Classic Dark', value: 'classic-dark' },
  { key: 'light', text: 'Light', value: 'light' },
];

const THEME_LABELS = THEME_OPTIONS.reduce(
  (labels, option) => ({ ...labels, [option.value]: option.text }),
  {},
);

const SOULSEEK_CREDENTIAL_STORE_OPTIONS = [
  { key: 'memory', text: 'This session only', value: 'memory' },
  { key: 'os', text: 'OS credential store', value: 'os' },
  { key: 'file', text: 'Local credential file', value: 'file' },
];

const normalizeTheme = (theme) => {
  if (theme === 'light' || theme === 'classic-dark') {
    return theme;
  }

  return 'slskr';
};

const getSemanticTheme = (theme) => (theme === 'light' ? 'light' : 'dark');

const toDisplayError = (error, fallback = 'Request failed') => {
  const value = error?.response?.data ?? error?.message ?? error;
  if (typeof value === 'string' || typeof value === 'number') return String(value);
  if (value && typeof value === 'object') {
    for (const key of ['message', 'error', 'detail', 'title']) {
      if (typeof value[key] === 'string' && value[key].trim()) return value[key];
    }
  }
  return fallback;
};

const normalizePortForwardProtocol = (proto) =>
  (typeof proto === 'string' ? proto : '').trim().toUpperCase().slice(0, MAX_NETWORK_ENDPOINT_TEXT_CHARACTERS);

const normalizeNetworkEndpointText = (value) =>
  (typeof value === 'string' || typeof value === 'number')
    ? String(value).trim().slice(0, MAX_NETWORK_ENDPOINT_TEXT_CHARACTERS)
    : undefined;

const normalizeNetworkEndpointSignature = (value) =>
  typeof value === 'string'
    ? value.trim().slice(0, MAX_NETWORK_ENDPOINT_SIGNATURE_CHARACTERS)
    : undefined;

const normalizeNetworkEndpointPort = (value) => {
  const port = Number(value);
  return Number.isInteger(port) && port > 0 && port <= 65_535 ? port : undefined;
};

const normalizeNetworkEndpointForward = (forward) => {
  if (!forward || typeof forward !== 'object' || Array.isArray(forward)) return null;
  const publicPort = normalizeNetworkEndpointPort(forward.publicPort);
  if (!publicPort) return null;

  const slot = Number(forward.slot);
  return {
    localPort: normalizeNetworkEndpointPort(forward.localPort),
    namespace: normalizeNetworkEndpointText(forward.namespace),
    proto: normalizePortForwardProtocol(forward.proto),
    publicIp: normalizeNetworkEndpointText(forward.publicIPAddress || forward.publicIp),
    publicPort,
    slot: Number.isSafeInteger(slot) ? slot : undefined,
    targetPort: normalizeNetworkEndpointPort(forward.targetPort),
  };
};

const normalizeNetworkEndpointForwards = (forwards) =>
  (Array.isArray(forwards) ? forwards : [])
    .slice(0, MAX_NETWORK_ENDPOINT_FORWARDS)
    .map(normalizeNetworkEndpointForward)
    .filter(Boolean)
    .sort((left, right) => (left.slot ?? 0) - (right.slot ?? 0));

const normalizeNetworkEndpointSnapshot = (snapshot) => {
  if (!snapshot || typeof snapshot !== 'object' || Array.isArray(snapshot)) return null;
  const signature = normalizeNetworkEndpointSignature(snapshot.signature);
  const portForwards = normalizeNetworkEndpointForwards(snapshot.portForwards);
  return signature && portForwards.length > 0 ? { portForwards, signature } : null;
};

const getOption = (source, ...keys) => {
  for (const key of keys) {
    if (source && Object.prototype.hasOwnProperty.call(source, key)) {
      return source[key];
    }
  }

  return undefined;
};

const toConfiguredPort = (value, fallback) => {
  const port = Number(value);
  return Number.isInteger(port) && port > 0 ? port : fallback;
};

const getVpnPortForwards = (vpn = {}) => {
  if (Array.isArray(vpn.portForwards) && vpn.portForwards.length > 0) {
    return normalizeNetworkEndpointForwards(vpn.portForwards);
  }

  const forwardedPort = normalizeNetworkEndpointPort(vpn.forwardedPort);
  if (forwardedPort) {
    return [
      {
        proto: 'TCP',
        publicIp: normalizeNetworkEndpointText(vpn.publicIPAddress),
        publicPort: forwardedPort,
        slot: 0,
      },
    ];
  }

  return [];
};

const getVpnPortSignature = (forwards) =>
  forwards
    .map((forward) =>
      [
        forward.slot ?? '',
        forward.proto ?? '',
        forward.publicIp ?? '',
        forward.publicPort ?? '',
        forward.localPort ?? '',
        forward.targetPort ?? '',
      ].join(':'),
    )
    .join('|');

const parseLegacyVpnPortSignature = (signature) => {
  if (
    typeof signature !== 'string'
    || !signature
    || signature.length > MAX_NETWORK_ENDPOINT_SIGNATURE_CHARACTERS
  ) return null;

  const portForwards = signature
    .split('|', MAX_NETWORK_ENDPOINT_FORWARDS)
    .map((entry) => {
      const [slot, proto, publicIp, publicPort, localPort, targetPort] = entry.split(':', 6);
      const slotNumber = Number.parseInt(slot, 10);
      const normalizedProto = normalizePortForwardProtocol(proto);

      return {
        label:
          slotNumber === 0
            ? 'Soulseek'
            : normalizedProto || 'Forward',
        localPort: normalizeNetworkEndpointPort(localPort),
        proto: normalizedProto,
        publicIp: normalizeNetworkEndpointText(publicIp),
        publicPort: normalizeNetworkEndpointPort(publicPort),
        slot: Number.isFinite(slotNumber) ? slotNumber : undefined,
        targetPort: normalizeNetworkEndpointPort(targetPort),
      };
    })
    .filter((forward) => forward.publicPort > 0);

  return portForwards.length ? { portForwards, signature } : null;
};

const hasDismissedVpnPortNotice = (signature) => {
  return getLocalStorageItem(NETWORK_ENDPOINT_NOTICE_STORAGE_KEY) === signature;
};

export const getStoredNetworkEndpointSnapshot = () => {
  for (const storageKey of [
    NETWORK_ENDPOINT_SNAPSHOT_STORAGE_KEY,
    LEGACY_NETWORK_ENDPOINT_SNAPSHOT_STORAGE_KEY,
  ]) {
    const snapshot = normalizeNetworkEndpointSnapshot(
      readBoundedJson(
        getLocalStorageItem,
        storageKey,
        null,
        MAX_NETWORK_ENDPOINT_STORAGE_CHARACTERS,
      ),
    );
    if (snapshot) return snapshot;
  }

  return parseLegacyVpnPortSignature(
    getLocalStorageItem(LEGACY_VPN_PORT_NOTICE_STORAGE_KEY, ''),
  );
};

const storeDismissedVpnPortNotice = (signature, portForwards) => {
  const normalizedSignature = normalizeNetworkEndpointSignature(signature);
  const snapshot = normalizeNetworkEndpointSnapshot({
    portForwards,
    signature: normalizedSignature,
  });
  if (!snapshot) return;

  setLocalStorageItem(NETWORK_ENDPOINT_NOTICE_STORAGE_KEY, snapshot.signature);
  const serialized = JSON.stringify(snapshot);
  if (serialized.length <= MAX_NETWORK_ENDPOINT_STORAGE_CHARACTERS) {
    setLocalStorageItem(NETWORK_ENDPOINT_SNAPSHOT_STORAGE_KEY, serialized);
  }
};

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

const getMessageTimestamp = (message) => {
  const rawTimestamp = message?.timestamp;
  const numericTimestamp = Number(rawTimestamp);
  if (Number.isFinite(numericTimestamp) && numericTimestamp > 0) {
    return numericTimestamp < 10_000_000_000
      ? numericTimestamp * 1_000
      : numericTimestamp;
  }

  const timestamp = Date.parse(rawTimestamp);
  return Number.isFinite(timestamp) ? timestamp : 0;
};

const isIncomingRoomMessage = (message) =>
  message?.self !== true && message?.direction !== 'Out';

const setNavigationHeightVariable = (element) => {
  if (!element || typeof document === 'undefined') return;

  const rect = element.getBoundingClientRect();
  const height = Math.ceil(rect.height || element.offsetHeight || 0);
  if (height > 0) {
    document.documentElement.style.setProperty(
      '--slskr-nav-height',
      `${height}px`,
    );
  }
};

const NavigationIcon = ({ alert, alertTestId, name }) => (
  <span className="navigation-alert-icon">
    <Icon name={name} />
    {alert && (
      <span
        aria-label="New activity"
        className="navigation-alert-dot"
        data-testid={alertTestId}
        role="status"
      />
    )}
  </span>
);

const LEGACY_INGRESS_PORTS = [
  {
    config: 'soulseek.listen_port',
    label: 'Soulseek peer/file transfers',
    port: 50300,
    proto: 'TCP',
  },
  {
    config: 'dht.overlay_port + dht.dht_port + overlay.quic_listen_port',
    label: 'slskr mesh, DHT rendezvous, and QUIC overlay',
    port: 50305,
    proto: 'TCP/UDP',
  },
  {
    config: 'mesh.overlay.listen_port',
    label: 'legacy mesh UDP overlay',
    port: 50400,
    proto: 'UDP',
  },
  {
    config: 'mesh.data.listen_port',
    label: 'legacy mesh data overlay',
    port: 50401,
    proto: 'UDP',
  },
  {
    config: 'mesh.overlay.quic_listen_port',
    label: 'legacy mesh QUIC overlay',
    port: 50402,
    proto: 'UDP',
  },
];

const buildCurrentIngressPorts = (options = {}) => {
  const soulseek = getOption(options, 'soulseek', 'Soulseek') || {};
  const dht = getOption(options, 'dht', 'dhtRendezvous', 'DhtRendezvous') || {};
  const soulseekListenPort = toConfiguredPort(
    getOption(soulseek, 'listenPort', 'listen_port', 'ListenPort'),
    50300,
  );
  const dhtOverlayPort = toConfiguredPort(
    getOption(dht, 'overlayPort', 'overlay_port', 'OverlayPort'),
    50300,
  );
  const dhtPort = toConfiguredPort(
    getOption(dht, 'dhtPort', 'dht_port', 'DhtPort'),
    50300,
  );
  if (soulseekListenPort === dhtOverlayPort && soulseekListenPort === dhtPort) {
    return [{
      config: 'soulseek.listen_port + dht.overlay_port + dht.dht_port + overlay.quic_listen_port',
      label: 'Soulseek peer/file transfers, slskr mesh overlay, DHT rendezvous, and QUIC overlay',
      port: soulseekListenPort,
      proto: 'TCP/UDP',
    }];
  }

  const ports = [{
    config: 'soulseek.listen_port',
    label: 'Soulseek peer/file transfers',
    port: soulseekListenPort,
    proto: 'TCP',
  }];

  if (dhtOverlayPort === dhtPort) {
    ports.push({
      config: 'dht.overlay_port + dht.dht_port',
      label: 'slskr mesh overlay and DHT rendezvous',
      port: dhtOverlayPort,
      proto: 'TCP/UDP',
    });
  } else {
    ports.push(
      {
        config: 'dht.overlay_port',
        label: 'slskr mesh overlay',
        port: dhtOverlayPort,
        proto: 'TCP',
      },
      {
        config: 'dht.dht_port',
        label: 'DHT rendezvous',
        port: dhtPort,
        proto: 'UDP',
      },
    );
  }

  return ports;
};

const IngressPortList = ({ expectedPorts, title }) => {
  if (!expectedPorts?.length) {
    return null;
  }

  return (
    <div className="network-endpoint-change-group">
      {title ? <span className="network-endpoint-change-title">{title}</span> : null}
      <div className="network-endpoint-change-list">
        {expectedPorts.map((expected) => (
          <div
            className="network-endpoint-change-item"
            key={`${expected.proto}-${expected.port}-${expected.config}`}
          >
            <span className="network-endpoint-change-service">
              {expected.label}
            </span>
            <code>{`${expected.proto} ${expected.port}`}</code>
            <span className="network-endpoint-change-config">
              {expected.config}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
};

const VpnPortChangeNotice = ({ onDismiss, options, portForwards }) => {
  if (!portForwards.length) {
    return null;
  }

  return (
    <Segment
      className="network-endpoint-change-notice"
      data-testid="vpn-port-change-notice"
    >
      <div className="network-endpoint-change-notice-body">
        <Icon name="exchange" />
        <div className="network-endpoint-change-notice-copy">
          <strong>slskr ingress ports were reduced.</strong>
          <span>
            Older builds needed five public forwards. Current defaults need one
            public port number on both TCP and UDP: Soulseek peer/file
            transfers and the slskr mesh/DHT/QUIC overlay.
          </span>
          <IngressPortList
            expectedPorts={LEGACY_INGRESS_PORTS}
            title="Used to need"
          />
          <IngressPortList
            expectedPorts={buildCurrentIngressPorts(options)}
            title="Need now"
          />
        </div>
      </div>
      <Popup
        content="Dismiss this port migration reminder until the forwarded ports change again."
        trigger={
          <Button
            basic
            compact
            icon="close"
            onClick={onDismiss}
            title="Dismiss port migration reminder"
          />
        }
      />
    </Segment>
  );
};

const initialState = {
  applicationOptions: {},
  applicationState: {},
  error: false,
  initialized: false,
  login: {
    error: undefined,
    pending: false,
  },
  navActivity: {
    chat: false,
    rooms: false,
  },
  retriesExhausted: false,
  soulseekCredentials: {
    credentialStore: 'memory',
    error: undefined,
    open: false,
    password: '',
    pending: false,
    username: '',
  },
  themeMenuOpen: false,
};

const getRuntimeProfileHint = () => {
  if (typeof document === 'undefined') {
    return undefined;
  }

  const target = document
    .querySelector('meta[name="slskr-runtime-profile"]')
    ?.getAttribute('content');
  return ['legacy', 'native'].includes(target) ? target : undefined;
};

const ModeSpecificConnectButton = ({
  runtimeProfile,
  connectionWatchdog,
  controller = {},
  mode,
  pendingReconnect,
  server,
  onConnect,
  user,
}) => {
  const compatibilityRole = runtimeProfile ? 'presentation' : undefined;

  if (mode === 'Agent') {
    const isConnected = controller?.state === 'Connected';
    const isTransitioning = ['Connecting', 'Reconnecting'].includes(
      controller?.state,
    );

    return (
      <Menu.Item
        onClick={() =>
          isConnected ? relayAPI.disconnect() : relayAPI.connect()
        }
        role={compatibilityRole}
      >
        <Icon.Group className="menu-icon-group">
          <Icon
            color={
              controller?.state === 'Connected'
                ? 'green'
                : isTransitioning
                  ? 'yellow'
                  : 'grey'
            }
            name="plug"
          />
          {!isConnected && (
            <Icon
              className="menu-icon-no-shadow"
              color="red"
              corner="bottom right"
              name="close"
            />
          )}
        </Icon.Group>
        Controller {controller?.state}
      </Menu.Item>
    );
  } else {
    if (server?.isConnected) {
      return (
        <Menu.Item
          disabled={server?.isDisconnecting}
          onClick={() => {
            if (!server?.isDisconnecting) {
              disconnect().catch((error) => {
                console.error('Failed to disconnect from Soulseek:', error);
              });
            }
          }}
          role={compatibilityRole}
        >
          <Icon.Group className="menu-icon-group">
            <Icon
              color={pendingReconnect ? 'yellow' : 'green'}
              name="plug"
            />
            {user?.privileges?.isPrivileged && (
              <Icon
                className="menu-icon-no-shadow"
                color="yellow"
                corner
                name="star"
              />
            )}
          </Icon.Group>
          Connected
        </Menu.Item>
      );
    }

    // the server is disconnected, and we need to give the user some information about what the client is doing
    // options are:
    // - nothing. the client was manually disconnected, kicked off by another login, etc., and we're not trying to connect
    // - actively trying to make a connection to the server
    // - still trying to connect, but waiting for the next connection attempt
    let icon = 'close';
    let color = 'red';

    if (connectionWatchdog?.isAttemptingConnection) {
      icon = 'clock';
      color = 'yellow';
    }

    const isSessionTransitioning =
      server?.isConnecting ||
      server?.IsConnecting ||
      server?.isLoggingIn ||
      server?.IsLoggingIn ||
      connectionWatchdog?.isAttemptingConnection;

    if (isSessionTransitioning) {
      icon = 'sync alternate loading';
      color = 'green';
    }

    const label = isSessionTransitioning
      ? 'Connecting'
      : server?.lastError
        ? 'Connection Failed'
        : 'Disconnected';

    return (
      <Menu.Item
        disabled={isSessionTransitioning}
        onClick={() => {
          if (!isSessionTransitioning) {
            onConnect?.(server) ?? connect();
          }
        }}
        role={compatibilityRole}
        title={server?.lastError || undefined}
      >
        <Icon.Group className="menu-icon-group">
          <Icon
            color="grey"
            name="plug"
          />
          <Icon
            className="menu-icon-no-shadow"
            color={color}
            corner="bottom right"
            name={icon}
          />
        </Icon.Group>
        {label}
      </Menu.Item>
    );
  }
};

const RouteMissRedirect = () => {
  const location = useLocation();

  if (typeof window !== 'undefined') {
    window.routeMissPath = location.pathname;
  }

  useEffect(() => {
    if (typeof window === 'undefined') return undefined;

    const timeout = window.setTimeout(() => {
      const element = document.querySelector('[data-testid="route-miss"]');
      if (element) {
        window.routeMissElement = element.textContent;
      }
    }, 100);

    return () => window.clearTimeout(timeout);
  }, [location.pathname]);

  console.error('[Router] Route miss for:', location.pathname);

  return (
    <>
      <div
        data-testid="route-miss"
        style={{
          background: 'red',
          color: 'white',
          left: 0,
          padding: '20px',
          position: 'fixed',
          top: 0,
          zIndex: 9_999,
        }}
      >
        Route miss: {location.pathname}
      </div>
      <Navigate replace to="/searches" />
    </>
  );
};

class App extends Component {
  constructor(props) {
    super(props);

    this.runtimeProfileHint = getRuntimeProfileHint();
    this.state = {
      ...initialState,
      applicationState: this.runtimeProfileHint
        ? { runtimeProfile: this.runtimeProfileHint }
        : initialState.applicationState,
    };
    this.applicationHub = undefined;
    this.navigationActivityInterval = undefined;
    this.navigationResizeObserver = undefined;
    this.roomActivityBaselined = false;
    this.isMountedFlag = false;
    this.navigationActivityRunning = false;
    this.navigationActivityRequestId = 0;
    this.connectionInFlight = false;
    this.loginInFlight = false;
  }

  componentDidMount() {
    this.isMountedFlag = true;
    this.init();
    this.startNavigationActivityPolling();
    this.startChromeMeasurement();
  }

  componentDidUpdate(previousProps) {
    if (previousProps.location?.pathname !== this.props.location?.pathname) {
      this.refreshNavigationActivity();
    }
    this.updateNavigationHeight();
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    this.navigationActivityRequestId += 1;
    if (this.applicationHub) {
      this.applicationHub.stop().catch(() => {});
      this.applicationHub = undefined;
    }

    if (this.navigationActivityInterval) {
      window.clearInterval(this.navigationActivityInterval);
    }

    if (this.navigationResizeObserver) {
      this.navigationResizeObserver.disconnect();
      this.navigationResizeObserver = undefined;
    }
  }

  startChromeMeasurement = () => {
    this.updateNavigationHeight();
    if (typeof window.ResizeObserver !== 'function') {
      return;
    }

    const navigation = document.querySelector('.navigation');
    if (!navigation) {
      return;
    }

    this.navigationResizeObserver = new window.ResizeObserver(
      this.updateNavigationHeight,
    );
    this.navigationResizeObserver.observe(navigation);
  };

  updateNavigationHeight = () => {
    setNavigationHeightVariable(document.querySelector('.navigation'));
  };

  startNavigationActivityPolling = () => {
    this.refreshNavigationActivity();
    this.navigationActivityInterval = window.setInterval(
      this.refreshNavigationActivity,
      NAV_ACTIVITY_POLL_INTERVAL_MS,
    );
  };

  getCurrentPath = () =>
    this.props.location?.pathname || window.location?.pathname || '';

  isAuthenticated = () => session.isLoggedIn() || isPassthroughEnabled();

  getChatActivity = async () => {
    if (
      this.getCurrentPath().startsWith('/chat') ||
      this.getCurrentPath().startsWith('/messages')
    ) {
      return false;
    }

    const conversations = await chat.getAll({ unAcknowledgedOnly: true });
    return Array.isArray(conversations) && conversations.length > 0;
  };

  getRoomsActivity = async () => {
    const joinedRoomsResponse = await rooms.getJoined();
    const joinedRooms = Array.isArray(joinedRoomsResponse)
      ? Array.from(new Set(
          joinedRoomsResponse.filter(
            (roomName) => typeof roomName === 'string' && roomName,
          ),
        )).slice(0, MAX_ROOM_ACTIVITY_ROOMS)
      : [];
    const roomMessages = await Promise.all(
      joinedRooms.map(async (roomName) => {
        const messages = await rooms.getMessages({ roomName });
        return {
          messages: Array.isArray(messages) ? messages : [],
          roomName,
        };
      }),
    );
    const latestByRoom = roomMessages.reduce((activity, room) => {
      const latest = room.messages
        .filter(isIncomingRoomMessage)
        .reduce(
          (latestTimestamp, message) =>
            Math.max(latestTimestamp, getMessageTimestamp(message)),
          0,
        );

      return latest > 0
        ? { ...activity, [room.roomName]: latest }
        : activity;
    }, {});

    if (
      this.getCurrentPath().startsWith('/rooms') ||
      this.getCurrentPath().startsWith('/messages')
    ) {
      storeRoomActivity(latestByRoom);
      this.roomActivityBaselined = true;
      return false;
    }

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

  refreshNavigationActivity = async () => {
    if (!this.isMountedFlag || this.navigationActivityRunning) return;
    this.navigationActivityRunning = true;
    const requestId = ++this.navigationActivityRequestId;

    if (['legacy', 'native'].includes(this.runtimeProfileHint)) {
      if (this.isMountedFlag && requestId === this.navigationActivityRequestId) {
        this.setState({
          navActivity: {
            chat: false,
            rooms: false,
          },
        });
      }
      this.navigationActivityRunning = false;
      return;
    }

    if (!this.isAuthenticated()) {
      if (this.isMountedFlag && requestId === this.navigationActivityRequestId) {
        this.setState({
          navActivity: {
            chat: false,
            rooms: false,
          },
        });
      }
      this.navigationActivityRunning = false;
      return;
    }

    try {
      const [chatActivity, roomsActivity] = await Promise.all([
        this.getChatActivity(),
        this.getRoomsActivity(),
      ]);

      if (
        this.isMountedFlag &&
        requestId === this.navigationActivityRequestId &&
        this.isAuthenticated()
      ) {
        this.setState({
          navActivity: {
            chat: chatActivity,
            rooms: roomsActivity,
          },
        });
      }
    } catch (error) {
      console.error('Failed to refresh navigation activity:', error);
    } finally {
      this.navigationActivityRunning = false;
    }
  };

  startApplicationHub = () => {
    if (!this.isMountedFlag) return;
    if (this.applicationHub) {
      this.applicationHub.stop().catch(() => {});
    }

    const HUB_START_TIMEOUT_MS = 30000;
    const appHub = createApplicationHubConnection();
    this.applicationHub = appHub;

    appHub.on('state', (state) => {
      if (!this.isMountedFlag || this.applicationHub !== appHub) return;
      this.setState({
        applicationState: this.runtimeProfileHint
          ? { ...state, runtimeProfile: this.runtimeProfileHint }
          : state,
      });
    });

    appHub.on('options', (options) => {
      if (!this.isMountedFlag || this.applicationHub !== appHub) return;
      this.setState({ applicationOptions: options });
    });

    appHub.onreconnecting(() =>
      this.isMountedFlag &&
      this.applicationHub === appHub &&
      this.setState({ error: true, retriesExhausted: false }),
    );
    appHub.onclose(() =>
      this.isMountedFlag &&
      this.applicationHub === appHub &&
      this.setState({ error: true, retriesExhausted: true }),
    );
    appHub.onreconnected(() =>
      this.isMountedFlag &&
      this.applicationHub === appHub &&
      this.setState({ error: false, retriesExhausted: false }),
    );

    const hubStart = appHub.start();
    let hubTimeoutId;
    const hubTimeout = new Promise((_, reject) => {
      hubTimeoutId = setTimeout(
        () => reject(new Error('HubConnectionTimeout')),
        HUB_START_TIMEOUT_MS,
      );
    });

    Promise.race([hubStart, hubTimeout])
      .catch((error) => {
        if (!this.isMountedFlag || this.applicationHub !== appHub) {
          return;
        }

        if (error?.message === 'HubConnectionTimeout') {
          console.warn(
            'Event feed connection timed out during background startup; allowing the UI to continue while WebSocket reconnects.',
          );
          return;
        }

        console.error(error);
        this.setState({ error: true, retriesExhausted: false });
      })
      .finally(() => {
        if (hubTimeoutId) {
          clearTimeout(hubTimeoutId);
        }

        // Prevent unhandled rejections if the timeout wins and the start later faults.
        hubStart.catch(() => {});
      });
  };

  init = async () => {
    this.setState({ initialized: false }, async () => {
      if (!this.isMountedFlag) return;
      const INIT_TOTAL_TIMEOUT_MS = 30000;

      let initTimedOut = false;
      let initTimeoutId;
      try {
        const initTask = (async () => {
          const securityEnabled = await session.getSecurityEnabled();
          if (!this.isMountedFlag) return;

          if (!securityEnabled) {
            console.debug('application security is not enabled, per api call');
            session.enablePassthrough();
          }

          const sessionValid =
            !securityEnabled && this.runtimeProfileHint === 'native'
              ? true
              : await session.check();
          if (!this.isMountedFlag) return;

          if (sessionValid) {
            if (this.runtimeProfileHint === 'native') {
              // The frozen native UI bootstraps from the build and collection
              // contracts.  Its system/application state arrives through the
              // native hubs; requesting the legacy application/options pair
              // here changes the observable API surface and makes a native
              // profile look like slskd to target clients.
              await collectionsAPI.getCollections();
              if (!this.isMountedFlag) return;
              this.setState({
                applicationOptions: {},
                applicationState: { runtimeProfile: 'native' },
              });
            } else if (this.runtimeProfileHint === 'legacy') {
              // The frozen slskd UI receives its application state and
              // options from the application hub.  Keep those legacy REST
              // bootstrap requests out of the compatibility profile so the
              // replacement has the same startup API inventory.
              this.setState({
                applicationOptions: {},
                applicationState: { runtimeProfile: 'legacy' },
              });
            } else {
              const [initialApplicationState, initialOptions] =
                await Promise.all([
                  getApplicationState(),
                  getApplicationOptions(),
                ]);
              if (!this.isMountedFlag) return;
              this.setState({
                applicationOptions: initialOptions || {},
                applicationState: {
                  ...(initialApplicationState || {}),
                },
              });
            }
            this.startApplicationHub();
          }

          const savedTheme = this.getSavedTheme();
          if (!this.isMountedFlag) return;
          if (savedTheme != null) {
            this.setState({ theme: savedTheme });
          }

          this.setState({
            error: false,
          });
        })();

        // Safety timeout so a stalled init doesn't keep the UI on the big loader forever.
        const initTimeout = new Promise((resolve) => {
          initTimeoutId = setTimeout(() => {
            initTimedOut = true;
            resolve();
          }, INIT_TOTAL_TIMEOUT_MS);
        });

        await Promise.race([initTask, initTimeout]);

        // Prevent unhandled rejections if the timeout wins.
        initTask.catch((error) => {
          if (initTimedOut) {
            console.warn('Init completed after timeout.', error);
          }
        });

        if (initTimedOut) {
          console.warn('Init timed out; showing UI (hub/state may reconnect later).');
        }
      } catch (error) {
        if (!initTimedOut && this.isMountedFlag) {
          console.error(error);
          this.setState({ error: true, retriesExhausted: true });
        }
      } finally {
        if (initTimeoutId) {
          clearTimeout(initTimeoutId);
        }
        if (this.isMountedFlag) this.setState({ initialized: true });
      }
    });
  };

  getSavedTheme = () => {
    const savedTheme = getLocalStorageItem('slskr-theme');
    return savedTheme == null ? null : normalizeTheme(savedTheme);
  };

  setTheme = (theme) => {
    const nextTheme = normalizeTheme(theme);
    setLocalStorageItem('slskr-theme', nextTheme);
    this.setState({ theme: nextTheme, themeMenuOpen: false });
  };

  openThemeMenu = () => {
    this.setState({ themeMenuOpen: true });
  };

  closeThemeMenu = () => {
    this.setState({ themeMenuOpen: false });
  };

  openSoulseekCredentials = (server) => {
    const modes = Array.isArray(server?.writableCredentialStoreModes)
      ? server.writableCredentialStoreModes
      : [];
    const defaultCredentialStore = modes.includes(server?.credentialStore)
      ? server.credentialStore
      : 'memory';
    this.setState((previousState) => ({
      soulseekCredentials: {
        ...previousState.soulseekCredentials,
        credentialStore: defaultCredentialStore,
        error: undefined,
        open: true,
      },
    }));
  };

  closeSoulseekCredentials = () => {
    this.setState((previousState) => ({
      soulseekCredentials: {
        ...previousState.soulseekCredentials,
        error: undefined,
        open: false,
        password: '',
        pending: false,
      },
    }));
  };

  updateSoulseekCredential = (field, value) => {
    this.setState((previousState) => ({
      soulseekCredentials: {
        ...previousState.soulseekCredentials,
        [field]: value,
      },
    }));
  };

  updateServerState = (server) => {
    if (!server) return;

    this.setState((previousState) => ({
      applicationState: {
        ...previousState.applicationState,
        server,
      },
    }));
  };

  setServerConnectError = (error) => {
    this.setState((previousState) => ({
      applicationState: {
        ...previousState.applicationState,
        server: {
          ...previousState.applicationState?.server,
          isConnecting: false,
          isLoggingIn: false,
          lastError: toDisplayError(error, 'Unable to connect to Soulseek.'),
        },
      },
    }));
  };

  handleSoulseekConnect = async (server) => {
    if (this.connectionInFlight || !this.isMountedFlag) return;
    const credentialsConfigured =
      server?.credentialsConfigured ||
      server?.runtimeCredentialsConfigured ||
      server?.credentialSource === 'config' ||
      server?.credentialSource === 'runtime';

    if (credentialsConfigured) {
      this.connectionInFlight = true;
      try {
        const response = await connect();
        this.updateServerState(response?.data);
      } catch (error) {
        console.error('Failed to connect to Soulseek:', error);
        this.setServerConnectError(error);
      } finally {
        this.connectionInFlight = false;
      }
      return;
    }

    this.openSoulseekCredentials(server);
  };

  submitSoulseekCredentials = async () => {
    if (this.connectionInFlight || !this.isMountedFlag) return;
    const { soulseekCredentials = {} } = this.state;
    const username = (soulseekCredentials.username || '').trim();
    const password = soulseekCredentials.password || '';

    if (!username || !password) {
      this.setState((previousState) => ({
        soulseekCredentials: {
          ...previousState.soulseekCredentials,
          error: 'Username and password are required.',
        },
      }));
      return;
    }

    this.connectionInFlight = true;
    this.setState((previousState) => ({
      soulseekCredentials: {
        ...previousState.soulseekCredentials,
        error: undefined,
        pending: true,
      },
    }));

    try {
      const response = await connect({
        credentialStore: soulseekCredentials.credentialStore || 'memory',
        password,
        username,
      });
      this.updateServerState(response?.data);
      this.closeSoulseekCredentials();
    } catch (error) {
      this.setState((previousState) => ({
        soulseekCredentials: {
          ...previousState.soulseekCredentials,
          error: toDisplayError(
            error,
            'Unable to connect with those credentials.',
          ),
          password: '',
          pending: false,
        },
      }));
    } finally {
      this.connectionInFlight = false;
    }
  };

  dismissVpnPortNotice = (signature, portForwards) => {
    storeDismissedVpnPortNotice(signature, portForwards);
    this.forceUpdate();
  };

  handleLogin = (username, password) => {
    if (this.loginInFlight || !this.isMountedFlag) return;
    this.loginInFlight = true;
    this.setState(
      (previousState) => ({
        login: { ...previousState.login, error: undefined, pending: true },
      }),
      async () => {
        try {
          await session.login({ password, username });
          this.setState(
            (previousState) => ({
              login: { ...previousState.login, error: false, pending: false },
            }),
            () => this.init(),
          );
        } catch (error) {
          this.setState((previousState) => ({
            login: { ...previousState.login, error, pending: false },
          }));
        } finally {
          this.loginInFlight = false;
        }
      },
    );
  };

  logout = () => {
    session.logout();
    this.setState({ login: { ...initialState.login } });
  };

  withTokenCheck = (component) => {
    return component;
  };

  // eslint-disable-next-line complexity
  render() {
    const {
      applicationOptions = {},
      applicationState = {},
      error,
      initialized,
      login,
      navActivity,
      retriesExhausted,
      soulseekCredentials = {},
      theme = normalizeTheme(this.getSavedTheme() || 'slskr'),
      themeMenuOpen,
    } = this.state;
    const semanticTheme = getSemanticTheme(theme);
    const {
      connectionWatchdog = {},
      pendingReconnect,
      pendingRestart,
      relay = {},
      server,
      shares = {},
      user,
      version = {},
    } = applicationState;
    const { current, isUpdateAvailable, latest } = version;
    const { scanPending: pendingShareRescan } = shares;
    const vpnPortForwards = getVpnPortForwards(applicationState.vpn);
    const vpnPortSignature = getVpnPortSignature(vpnPortForwards);
    const previousNetworkEndpointSnapshot = getStoredNetworkEndpointSnapshot();
    const showVpnPortNotice =
      vpnPortSignature &&
      applicationState.vpn?.isReady &&
      !hasDismissedVpnPortNotice(vpnPortSignature) &&
      previousNetworkEndpointSnapshot?.signature !== vpnPortSignature;

    const { controller, mode } = relay;
    const runtimeProfile = ['legacy', 'native'].includes(
      applicationState.runtimeProfile,
    )
      ? applicationState.runtimeProfile
      : undefined;
    const isLegacyProfile = runtimeProfile === 'legacy';
    const isNativeProfile = runtimeProfile === 'native';

    if (!initialized) {
      return (
        <Loader
          active
          size="big"
        />
      );
    }

    if (!session.isLoggedIn() && !isPassthroughEnabled()) {
      if (error) {
        return (
          <ErrorSegment
            caption={
              <>
                <span>Lost connection to slskr</span>
                <br />
                <span>
                  {retriesExhausted ? 'Refresh to reconnect' : 'Retrying...'}
                </span>
              </>
            }
            icon="attention"
            suppressPrefix
          />
        );
      }

      return (
        <LoginForm
          error={login.error}
          initialized={login.initialized}
          loading={login.pending}
          onLoginAttempt={this.handleLogin}
        />
      );
    }

    const isAgent = mode === 'Agent';
    document.title = 'slskR';

    document.documentElement.classList.remove(
      'classic-dark',
      'dark',
      'light',
      'slskr',
    );
    document.documentElement.classList.add(theme);
    if (semanticTheme === 'dark') {
      document.documentElement.classList.add('dark');
    }

    return (
      <>
        {error && (
          <Segment
            color="red"
            inverted
            style={{
              borderRadius: 0,
              margin: 0,
              padding: '0.75rem 1rem',
            }}
          >
            <Icon name="attention" />
            Lost connection to slskr. {retriesExhausted ? 'Refresh to reconnect.' : 'Retrying...'}
          </Segment>
        )}
        <Modal
          centered
          closeIcon={!soulseekCredentials.pending}
          onClose={this.closeSoulseekCredentials}
          open={soulseekCredentials.open}
          size="mini"
        >
          <Modal.Header>Soulseek Authentication</Modal.Header>
          <Modal.Content>
            <Form
              error={Boolean(soulseekCredentials.error)}
              onSubmit={this.submitSoulseekCredentials}
            >
              <Form.Input
                autoComplete="username"
                disabled={soulseekCredentials.pending}
                label="Username"
                onChange={(_, data) =>
                  this.updateSoulseekCredential('username', data.value)
                }
                value={soulseekCredentials.username}
              />
              <Form.Input
                autoComplete="current-password"
                disabled={soulseekCredentials.pending}
                label="Password"
                onChange={(_, data) =>
                  this.updateSoulseekCredential('password', data.value)
                }
                type="password"
                value={soulseekCredentials.password}
              />
              <Form.Select
                disabled={soulseekCredentials.pending}
                label="Credential storage"
                onChange={(_, data) =>
                  this.updateSoulseekCredential('credentialStore', data.value)
                }
                options={SOULSEEK_CREDENTIAL_STORE_OPTIONS.filter((option) =>
                  (server?.writableCredentialStoreModes || ['memory']).includes(
                    option.value,
                  ),
                )}
                value={soulseekCredentials.credentialStore}
              />
              {soulseekCredentials.error && (
                <Segment
                  color="red"
                  inverted
                >
                  {soulseekCredentials.error}
                </Segment>
              )}
            </Form>
          </Modal.Content>
          <Modal.Actions>
            <Button
              disabled={soulseekCredentials.pending}
              onClick={this.closeSoulseekCredentials}
            >
              Cancel
            </Button>
            <Button
              loading={soulseekCredentials.pending}
              onClick={this.submitSoulseekCredentials}
              primary
            >
              Connect
            </Button>
          </Modal.Actions>
        </Modal>
        <PlayerProvider>
          <Sidebar.Pushable
            as={Segment}
            className="app"
            data-runtime-profile={runtimeProfile || 'unknown'}
          >
            <Sidebar
              animation="overlay"
              as={Menu}
              className="navigation"
              direction="top"
              horizontal="true"
              icon="labeled"
              inverted
              visible
              width="thin"
            >
              <div className="navigation-primary">
                {version.isCanary && (
                  <Menu.Item>
                    <Icon
                      color="yellow"
                      name="flask"
                    />
                    Canary
                  </Menu.Item>
                )}
              {isAgent ? (
                <Menu.Item>
                  <Icon name="detective" />
                  Agent Mode
                </Menu.Item>
              ) : (
                isLegacyProfile ? (
                <>
                  <NavLink to="/dashboard">
                    <Menu.Item data-testid="nav-dashboard">
                      <Icon name="chart bar" />
                      Dashboard
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/searches">
                    <Menu.Item data-testid="nav-search">
                      <Icon name="search" />
                      Search
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/downloads">
                    <Menu.Item data-testid="nav-downloads">
                      <Icon name="download" />
                      Downloads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/uploads">
                    <Menu.Item data-testid="nav-uploads">
                      <Icon name="upload" />
                      Uploads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/rooms">
                    <Menu.Item data-testid="nav-rooms">
                      <NavigationIcon
                        alert={navActivity.rooms}
                        alertTestId="nav-rooms-alert"
                        name="comments"
                      />
                      Rooms
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/chat">
                    <Menu.Item data-testid="nav-chat">
                      <NavigationIcon
                        alert={navActivity.chat}
                        alertTestId="nav-chat-alert"
                        name="comment"
                      />
                      Chat
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/users">
                    <Menu.Item data-testid="nav-users">
                      <Icon name="users" />
                      Users
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/browse">
                    <Menu.Item data-testid="nav-browse">
                      <Icon name="folder open" />
                      Browse
                    </Menu.Item>
                  </NavLink>
                </>
                ) : isNativeProfile ? (
                <>
                  <NavLink to="/searches">
                    <Menu.Item data-testid="nav-search">
                      <Icon name="search" />
                      Search
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/discovery-graph">
                    <Menu.Item data-testid="nav-discovery-graph">
                      <Icon name="crosshairs" />
                      Discovery Graph
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/playlist-intake">
                    <Menu.Item data-testid="nav-playlist-intake">
                      <Icon name="list alternate outline" />
                      Playlist Intake
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/wishlist">
                    <Menu.Item data-testid="nav-wishlist">
                      <Icon name="star" />
                      Wishlist
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/lidarr">
                    <Menu.Item data-testid="nav-lidarr">
                      <Icon name="music" />
                      Lidarr
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/downloads">
                    <Menu.Item data-testid="nav-downloads">
                      <Icon name="download" />
                      Downloads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/uploads">
                    <Menu.Item data-testid="nav-uploads">
                      <Icon name="upload" />
                      Uploads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/messages">
                    <Menu.Item data-testid="nav-messages">
                      <NavigationIcon
                        alert={navActivity.rooms || navActivity.chat}
                        alertTestId={
                          navActivity.chat ? 'nav-chat-alert' : 'nav-rooms-alert'
                        }
                        name="comments"
                      />
                      Messages
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/users">
                    <Menu.Item data-testid="nav-users">
                      <Icon name="users" />
                      Users
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/contacts">
                    <Menu.Item data-testid="nav-contacts">
                      <Icon name="address book" />
                      Contacts
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/solid">
                    <Menu.Item data-testid="nav-solid">
                      <Icon name="key" />
                      Solid
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/collections">
                    <Menu.Item data-testid="nav-collections">
                      <Icon name="list" />
                      Collections
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/sharegroups">
                    <Menu.Item data-testid="nav-groups">
                      <Icon name="users" />
                      Share Groups
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/shared">
                    <Menu.Item data-testid="nav-shared-with-me">
                      <Icon name="share" />
                      Shared with Me
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/browse">
                    <Menu.Item data-testid="nav-browse">
                      <Icon name="folder open" />
                      Browse
                    </Menu.Item>
                  </NavLink>
                </>
                ) : (
                <>
                  <NavLink to="/searches">
                    <Menu.Item data-testid="nav-search">
                      <Icon name="search" />
                      Search
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/wishlist">
                    <Menu.Item data-testid="nav-wishlist">
                      <Icon name="star" />
                      Wishlist
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/downloads">
                    <Menu.Item data-testid="nav-downloads">
                      <Icon name="download" />
                      Downloads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/uploads">
                    <Menu.Item data-testid="nav-uploads">
                      <Icon name="upload" />
                      Uploads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/messages">
                    <Menu.Item data-testid="nav-messages">
                      <NavigationIcon
                        alert={navActivity.rooms || navActivity.chat}
                        alertTestId={
                          navActivity.chat ? 'nav-chat-alert' : 'nav-rooms-alert'
                        }
                        name="comments"
                      />
                      Messages
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/users">
                    <Menu.Item data-testid="nav-users">
                      <Icon name="users" />
                      Users
                    </Menu.Item>
                  </NavLink>
                  <Dropdown
                    className="navigation-more"
                    data-testid="nav-more"
                    icon={null}
                    item
                    trigger={(
                      <span className="navigation-more-trigger">
                        <Icon name="ellipsis horizontal" />
                        More
                      </span>
                    )}
                  >
                    <Dropdown.Menu>
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-discovery-graph"
                        icon="crosshairs"
                        text="Discovery Graph"
                        to="/discovery-graph"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-playlist-intake"
                        icon="list alternate outline"
                        text="Playlist Intake"
                        to="/playlist-intake"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-contacts"
                        icon="address book"
                        text="Contacts"
                        to="/contacts"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-solid"
                        icon="key"
                        text="Solid"
                        to="/solid"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-collections"
                        icon="list"
                        text="Collections"
                        to="/collections"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-groups"
                        icon="users"
                        text="Share Groups"
                        to="/sharegroups"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-shared-with-me"
                        icon="share"
                        text="Shared with Me"
                        to="/shared"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-browse"
                        icon="folder open"
                        text="Browse"
                        to="/browse"
                      />
                    </Dropdown.Menu>
                  </Dropdown>
                </>
                )
              )}
            </div>
            <Menu
              className="right"
              inverted
            >
              <ModeSpecificConnectButton
                runtimeProfile={runtimeProfile}
                connectionWatchdog={connectionWatchdog}
                controller={controller}
                mode={mode}
                onConnect={this.handleSoulseekConnect}
                pendingReconnect={pendingReconnect}
                server={server}
                user={user}
              />
              <Popup
                basic
                className="theme-picker-popup"
                on="click"
                onClose={this.closeThemeMenu}
                onOpen={this.openThemeMenu}
                open={themeMenuOpen}
                pinned
                position="bottom right"
                trigger={(
                  <Menu.Item
                    className={`theme-menu ${themeMenuOpen ? 'visible' : ''}`}
                    data-testid="theme-menu"
                    role={runtimeProfile ? 'presentation' : undefined}
                    title="Choose the web UI color theme"
                  >
                    <Icon name="paint brush" />
                    <span className="theme-menu-label">Theme</span>
                  </Menu.Item>
                )}
              >
                <Menu
                  className="theme-picker-menu"
                  vertical
                >
                  {THEME_OPTIONS.map((option) => (
                    <Menu.Item
                      active={theme === option.value}
                      data-testid={`theme-option-${option.value}`}
                      key={option.value}
                      onClick={() => this.setTheme(option.value)}
                    >
                      <Icon name="theme" />
                      {option.text}
                    </Menu.Item>
                  ))}
                </Menu>
              </Popup>
              {(pendingReconnect || pendingRestart || pendingShareRescan) && (
                <Menu.Item position="right">
                  <Icon.Group className="menu-icon-group">
                    <NavLink to="/system/info">
                      <Icon
                        color="yellow"
                        name="exclamation circle"
                      />
                    </NavLink>
                  </Icon.Group>
                  Pending Action
                </Menu.Item>
              )}
              {isUpdateAvailable && (
                <Modal
                  centered
                  closeIcon
                  size="mini"
                  trigger={
                    <Menu.Item position="right">
                      <Icon.Group className="menu-icon-group">
                        <Icon
                          color="yellow"
                          name="bullhorn"
                        />
                      </Icon.Group>
                      New Version!
                    </Menu.Item>
                  }
                >
                  <Modal.Header>New Version!</Modal.Header>
                  <Modal.Content>
                    <p>
                      You are currently running version{' '}
                      <strong>{current}</strong>
                      while version <strong>{latest}</strong> is available.
                    </p>
                  </Modal.Content>
                  <Modal.Actions>
                    <Button
                      fluid
                      href={SLSKR_RELEASES_URL}
                      primary
                      style={{ marginLeft: 0 }}
                    >
                      See Release Notes
                    </Button>
                  </Modal.Actions>
                </Modal>
              )}
              <NavLink to="/system">
                <Menu.Item data-testid="nav-system">
                  <Icon name="cogs" />
                  System
                </Menu.Item>
              </NavLink>
              {session.isLoggedIn() && (
                <Modal
                  actions={[
                    'Cancel',
                    {
                      content: 'Log Out',
                      key: 'done',
                      negative: true,
                      onClick: this.logout,
                    },
                  ]}
                  centered
                  content="Are you sure you want to log out?"
                  header={
                    <Header
                      content="Confirm Log Out"
                      icon="sign-out"
                    />
                  }
                  size="mini"
                  trigger={
                    <Menu.Item data-testid="logout">
                      <Icon name="sign-out" />
                      Log Out
                    </Menu.Item>
                  }
                />
              )}
            </Menu>
            </Sidebar>
            <Sidebar.Pusher className="app-content">
              {showVpnPortNotice && (
                <VpnPortChangeNotice
                  onDismiss={() =>
                    this.dismissVpnPortNotice(vpnPortSignature, vpnPortForwards)
                  }
                  options={applicationOptions}
                  portForwards={vpnPortForwards}
                />
              )}
              <AppContext.Provider
                // Note: Context value object recreated on each render (class component limitation)
                // Deferred: Optimize with useMemo when converting to functional component
                // Deferred until this class component is converted to hooks.
                // eslint-disable-next-line react/jsx-no-constructed-context-values
                value={{ options: applicationOptions, state: applicationState }}
              >
                <Suspense
                  fallback={
                    <Segment
                      basic
                      className="view"
                    >
                      <Loader active />
                    </Segment>
                  }
                >
                  {isAgent ? (
                  <Routes>
                  <Route
                    path="/system"
                    element={
                      this.withTokenCheck(
                        <System
                          options={applicationOptions}
                          state={applicationState}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/system/:tab"
                    element={
                      this.withTokenCheck(
                        <System
                          options={applicationOptions}
                          state={applicationState}
                        />,
                      )
                    }
                  />
                  <Route
                    path="*"
                    element={<Navigate replace to="/system" />}
                  />
                  </Routes>
                  ) : (
                  <Routes>
                  <Route
                    path="/"
                    element={
                      <Navigate
                        replace
                        to={isLegacyProfile ? '/dashboard' : '/searches'}
                      />
                    }
                  />
                  <Route
                    path="/dashboard"
                    element={
                      isLegacyProfile ? (
                        this.withTokenCheck(
                          <CompatibilityDashboard
                            runtimeProfile={runtimeProfile}
                            server={applicationState.server}
                          />,
                        )
                      ) : (
                        <Navigate replace to="/searches" />
                      )
                    }
                  />
                  <Route
                    path="/lidarr"
                    element={<Navigate replace to="/system/integrations" />}
                  />
                  <Route
                    path="/collections"
                    element={(() => {
                      // This should log if route matches
                      if (typeof window !== 'undefined') {
                        window.routeMatchedCollections = true;
                        console.log(
                          '[Router] /collections route matched!',
                          '/collections',
                        );
                      }

                      try {
                        const result = this.withTokenCheck(
                          <div className="view">
                            <Collections />
                          </div>,
                        );
                        console.log(
                          '[Router] Collections rendered successfully',
                        );
                        return result;
                      } catch (renderError) {
                        console.error(
                          '[Router] Error rendering Collections:',
                          renderError,
                        );
                        // Return error UI instead of crashing
                        return (
                          <div className="view">
                            <ErrorSegment
                              caption={`Error loading Collections: ${renderError.message}`}
                            />
                          </div>
                        );
                      }
                    })()}
                  />
                  <Route
                    path="/solid"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <SolidSettings />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/discovery-graph"
                    element={
                      this.withTokenCheck(
                        <DiscoveryGraphAtlasPage
                          server={applicationState.server}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/playlist-intake"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <PlaylistIntake />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/searches"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <Searches
                            runtimeProfile={runtimeProfile}
                            server={applicationState.server}
                          />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/searches/:id"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <Searches
                            runtimeProfile={runtimeProfile}
                            server={applicationState.server}
                          />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/wishlist"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <Wishlist />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/browse"
                    element={this.withTokenCheck(
                      <Browse runtimeProfile={runtimeProfile} />,
                    )}
                  />
                  <Route
                    path="/users"
                    element={this.withTokenCheck(<Users />)}
                  />
                  <Route
                    path="/contacts"
                    element={this.withTokenCheck(<Contacts />)}
                  />
                  <Route
                    path="/sharegroups"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <ShareGroups />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/shared"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <SharedWithMe />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/chat"
                    element={
                      this.withTokenCheck(
                        isLegacyProfile ? (
                          <Chat
                            runtimeProfile={runtimeProfile}
                            state={applicationState}
                          />
                        ) : (
                          <Messaging
                            initialKind="chat"
                            state={applicationState}
                          />
                        ),
                      )
                    }
                  />
                  <Route
                    path="/pods"
                    element={
                      this.withTokenCheck(
                          <Messaging
                            runtimeProfile={runtimeProfile}
                            initialKind="pod"
                          state={applicationState}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/pods/:podId"
                    element={<Navigate replace to="/messages" />}
                  />
                  <Route
                    path="/pods/:podId/channels/:channelId"
                    element={<Navigate replace to="/messages" />}
                  />
                  <Route
                    path="/rooms"
                    element={
                      this.withTokenCheck(
                        isLegacyProfile ? (
                          <Rooms runtimeProfile={runtimeProfile} />
                        ) : (
                          <Messaging
                            runtimeProfile={runtimeProfile}
                            initialKind="room"
                            state={applicationState}
                          />
                        ),
                      )
                    }
                  />
                  <Route
                    path="/messages"
                    element={
                      isLegacyProfile ? (
                        <Navigate replace to="/chat" />
                      ) : this.withTokenCheck(
                        <Messaging
                          runtimeProfile={runtimeProfile}
                          initialKind="mixed"
                          state={applicationState}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/uploads"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <Transfers
                            runtimeProfile={runtimeProfile}
                            direction="upload"
                          />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/downloads"
                    element={
                      this.withTokenCheck(
                        <div className="view">
                          <Transfers
                            runtimeProfile={runtimeProfile}
                            direction="download"
                            server={applicationState.server}
                          />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/system"
                    element={
                      this.withTokenCheck(
                        <System
                          runtimeProfile={runtimeProfile}
                          options={applicationOptions}
                          state={applicationState}
                          theme={semanticTheme}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/system/:tab"
                    element={
                      this.withTokenCheck(
                        <System
                          runtimeProfile={runtimeProfile}
                          options={applicationOptions}
                          state={applicationState}
                          theme={semanticTheme}
                        />,
                      )
                    }
                  />
                  <Route
                    path="*"
                    element={<RouteMissRedirect />}
                  />
                  </Routes>
                  )}
                </Suspense>
              </AppContext.Provider>
            </Sidebar.Pusher>
          </Sidebar.Pushable>
          {!isLegacyProfile && (
            <PlayerBar runtimeProfile={runtimeProfile} />
          )}
        </PlayerProvider>
        <ToastContainer
          autoClose={5_000}
          closeOnClick
          draggable={false}
          hideProgressBar={false}
          newestOnTop
          pauseOnFocusLoss
          pauseOnHover
          position="bottom-center"
          rtl={false}
        />
        <Footer runtimeProfile={runtimeProfile} />
      </>
    );
  }
}

const AppWithLocation = (props) => {
  const location = useLocation();
  return (
    <App
      {...props}
      location={location}
    />
  );
};

export { App };
export default AppWithLocation;
