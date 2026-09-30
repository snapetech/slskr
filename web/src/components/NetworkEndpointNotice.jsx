import React from 'react';
import { getLocalStorageItem, setLocalStorageItem } from '../lib/storage';
import { readBoundedJson } from '../lib/persistedJson';
import { Button, Icon, Popup, Segment } from 'semantic-ui-react';

const MAX_NETWORK_ENDPOINT_FORWARDS = 32;
const MAX_NETWORK_ENDPOINT_TEXT_CHARACTERS = 256;
const MAX_NETWORK_ENDPOINT_SIGNATURE_CHARACTERS = 64 * 1024;
const MAX_NETWORK_ENDPOINT_STORAGE_CHARACTERS = 64 * 1024;

const NETWORK_ENDPOINT_NOTICE_STORAGE_KEY =
  'slskr.networkEndpoints.v2.dismissedSignature';
const NETWORK_ENDPOINT_SNAPSHOT_STORAGE_KEY =
  'slskr.networkEndpoints.v2.lastDismissedSnapshot';
const LEGACY_NETWORK_ENDPOINT_SNAPSHOT_STORAGE_KEY =
  'slskr.networkEndpoints.lastDismissedSnapshot';
const LEGACY_VPN_PORT_NOTICE_STORAGE_KEY =
  'slskr.vpnForwardedPorts.dismissedSignature';
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


export {
  getVpnPortForwards,
  getVpnPortSignature,
  hasDismissedVpnPortNotice,
  storeDismissedVpnPortNotice,
  VpnPortChangeNotice,
};
