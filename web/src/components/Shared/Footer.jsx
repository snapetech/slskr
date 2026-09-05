import './Footer.css';
import * as application from '../../lib/application';
import { toDisplayError } from '../../lib/errors';
import * as mesh from '../../lib/mesh';
import * as session from '../../lib/session';
import * as slskrAPI from '../../lib/slskr';
import { getLocalStorageItem } from '../../lib/storage';
import * as transfers from '../../lib/transfers';
import { createPollingController } from '../../lib/usePolling';
import { urlBase } from '../../config';
import React, { Component } from 'react';
import { Icon } from 'semantic-ui-react';

const GITHUB_BASE = 'https://github.com/snapetech/slskr';

const formatSpeed = (bytesPerSec) => {
  if (!bytesPerSec || bytesPerSec === 0) return { unit: 'B', value: '0' };

  const kb = bytesPerSec / 1_024;
  const mb = kb / 1_024;
  const gb = mb / 1_024;

  if (gb >= 1) {
    return { unit: 'G', value: gb.toFixed(gb >= 10 ? 1 : 2) };
  }

  if (mb >= 1) {
    return { unit: 'M', value: mb.toFixed(mb >= 10 ? 1 : 2) };
  }

  if (kb >= 1) {
    return { unit: 'K', value: kb.toFixed(kb >= 10 ? 1 : 2) };
  }

  return { unit: 'B', value: bytesPerSec.toFixed(0) };
};

const formatCount = (value) => {
  if (value === undefined || value === null) return '—';
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return value.toString();
};

const finiteNumberOrNull = (value) => {
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
};

const setFooterHeightVariable = (element) => {
  if (!element || typeof document === 'undefined') return;

  const height = Math.ceil(element.getBoundingClientRect().height);
  if (height > 0) {
    document.documentElement.style.setProperty(
      '--slskr-footer-height',
      `${height}px`,
    );
  }
};

class Footer extends Component {
  constructor(props) {
    super(props);
    this.state = {
      buildInfo: null,
      buildInfoError: null,
      slskrStats: null,
      statsError: null,
      speeds: null,
      speedsError: null,
      stats: null,
    };
    this.footerRef = React.createRef();
    this.footerResizeObserver = null;
    this.pollController = null;
    this.isMountedFlag = false;
    this.requestIds = {
      buildInfo: 0,
      speeds: 0,
      stats: 0,
    };
  }

  componentDidMount() {
    this.isMountedFlag = true;
    this.updateFooterHeight();
    if (
      typeof window !== 'undefined' &&
      typeof window.ResizeObserver === 'function' &&
      this.footerRef.current
    ) {
      this.footerResizeObserver = new window.ResizeObserver(
        this.updateFooterHeight,
      );
      this.footerResizeObserver.observe(this.footerRef.current);
    }

    void this.fetchBuildInfo();

    if (session.isLoggedIn()) {
      this.pollController = createPollingController(
        async () => {
          await Promise.all([this.fetchStats(), this.fetchSpeeds()]);
        },
        2_000,
      );
    }
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    Object.keys(this.requestIds).forEach((key) => {
      this.requestIds[key] += 1;
    });
    this.pollController?.stop();
    this.pollController = null;
    if (this.footerResizeObserver) {
      this.footerResizeObserver.disconnect();
      this.footerResizeObserver = null;
    }
  }

  componentDidUpdate() {
    this.updateFooterHeight();
  }

  updateFooterHeight = () => {
    setFooterHeightVariable(this.footerRef.current);
  };

  fetchStats = async () => {
    if (!this.isMountedFlag || !session.isLoggedIn()) {
      return;
    }

    const requestId = ++this.requestIds.stats;
    try {
      const [transportStats, slskrStats] = await Promise.allSettled([
        mesh.getStats(),
        slskrAPI.getSlskrStats(),
      ]);

      if (this.isMountedFlag && requestId === this.requestIds.stats) {
        const nextState = {};
        const errors = [];
        if (slskrStats.status === 'fulfilled') {
          nextState.slskrStats = slskrStats.value;
        } else {
          errors.push(toDisplayError(slskrStats.reason, 'slskr telemetry unavailable'));
        }
        if (transportStats.status === 'fulfilled') {
          nextState.stats = transportStats.value;
        } else {
          errors.push(toDisplayError(transportStats.reason, 'Network telemetry unavailable'));
        }
        nextState.statsError = errors.length > 0 ? errors.join(' ') : null;
        this.setState(nextState);
      }
    } catch (error) {
      console.debug('Failed to fetch mesh stats:', error);
      if (this.isMountedFlag && requestId === this.requestIds.stats) {
        this.setState({
          statsError: toDisplayError(error, 'Network telemetry unavailable'),
        });
      }
    }
  };

  fetchSpeeds = async () => {
    if (!this.isMountedFlag || !session.isLoggedIn()) {
      return;
    }

    const requestId = ++this.requestIds.speeds;
    try {
      const speeds = await transfers.getSpeeds();
      if (this.isMountedFlag && requestId === this.requestIds.speeds) {
        this.setState({ speeds, speedsError: null });
      }
    } catch (error) {
      console.debug('Failed to fetch transfer speeds:', error);
      if (this.isMountedFlag && requestId === this.requestIds.speeds) {
        this.setState({
          speedsError: toDisplayError(error, 'Transfer speeds unavailable'),
        });
      }
    }
  };

  fetchBuildInfo = async () => {
    if (!this.isMountedFlag || this.props.runtimeProfile === 'legacy') {
      return;
    }

    const requestId = ++this.requestIds.buildInfo;
    try {
      const buildInfo = await application.getBuild({ checkForUpdates: true });
      if (this.isMountedFlag && requestId === this.requestIds.buildInfo) {
        this.setState({ buildInfo, buildInfoError: null });
      }
    } catch (error) {
      console.debug('Failed to fetch build info:', error);
      if (this.isMountedFlag && requestId === this.requestIds.buildInfo) {
        this.setState({
          buildInfoError: toDisplayError(error, 'Build information unavailable'),
        });
      }
    }
  };

  render() {
    if (this.props.runtimeProfile === 'legacy') {
      return (
        <footer
          className="slskr-footer"
          ref={this.footerRef}
        >
          <div className="slskr-footer-content">
            <a
              href="https://github.com/slskd/slskd"
              rel="noopener noreferrer"
              target="_blank"
            >
              slskd 0.0.0 AGPLv3
            </a>
          </div>
        </footer>
      );
    }

    const year = new Date().getFullYear();
    const {
      buildInfo,
      buildInfoError,
      slskrStats,
      speeds,
      speedsError,
      stats,
      statsError,
    } = this.state;
    const isLoggedIn = session.isLoggedIn();

    if (!isLoggedIn && this.props.runtimeProfile !== 'native') {
      // Nothing here is real yet — no donation asks, build badge, or live
      // telemetry before someone has even signed in. Just attribution.
      return (
        <footer
          className="slskr-footer slskr-footer-minimal"
          ref={this.footerRef}
        >
          <span className="slskr-footer-copyright">
            © {year}{' '}
            <a
              href={GITHUB_BASE}
              rel="noopener noreferrer"
              target="_blank"
              title="slskr project"
            >
              slskr
            </a>
          </span>
        </footer>
      );
    }

    const currentBuild = buildInfo?.current || buildInfo?.full || 'unknown';
    const fullBuild = buildInfo?.full || currentBuild;
    const latestBuild = buildInfo?.latest || '';
    const latestTag = buildInfo?.latestTag || latestBuild;
    const latestUrl = buildInfo?.latestUrl || `${GITHUB_BASE}/releases`;
    const isUpdateAvailable = buildInfo?.isUpdateAvailable === true;
    const dht = slskrStats?.dht || {};
    const hashDb = slskrStats?.hashDb || {};
    const meshStats = slskrStats?.mesh || {};
    const swarmJobs = Array.isArray(slskrStats?.swarmJobs)
      ? slskrStats.swarmJobs
      : [];
    const dhtNodes =
      finiteNumberOrNull(dht.dhtNodeCount) ?? finiteNumberOrNull(stats?.dht);
    const discoveredPeers = finiteNumberOrNull(dht.discoveredPeerCount);
    const displayedDhtPeers = discoveredPeers ?? dhtNodes;
    const meshPeers = finiteNumberOrNull(meshStats.connectedPeerCount);
    const hashCount = finiteNumberOrNull(hashDb.totalEntries);
    const seqId =
      finiteNumberOrNull(hashDb.currentSeqId) ??
      finiteNumberOrNull(meshStats.localSeqId);
    const isSyncing = Boolean(meshStats.isSyncing);
    const backfillActive = Boolean(slskrStats?.backfill?.isActive);
    const activeSwarms = swarmJobs.length;
    const karma = Number.parseInt(getLocalStorageItem('slskr-karma', '0'), 10);
    const totalSpeed = isLoggedIn && speeds ? formatSpeed(speeds.total) : null;
    const soulseekSpeed =
      isLoggedIn && speeds ? formatSpeed(speeds.soulseek) : null;
    const meshSpeed = isLoggedIn && speeds ? formatSpeed(speeds.mesh) : null;

    // Determine if stats are connected
    const isDhtConnected = isLoggedIn && displayedDhtPeers > 0;
    const isOverlayConnected = isLoggedIn && stats && stats.overlay > 0;
    const isNatResolved =
      isLoggedIn && stats && stats.natType && stats.natType !== 'Unknown';

    // Format NAT type tooltip
    const natTooltip =
      isLoggedIn && stats
        ? `NAT Type: ${stats.natType || 'Unknown'}`
        : 'NAT: Login to see stats';
    const telemetryErrors = [statsError, speedsError, buildInfoError]
      .filter(Boolean)
      .join(' ');
    const networkTooltip = isLoggedIn
      ? `DHT peers: ${formatCount(displayedDhtPeers)}; DHT nodes: ${formatCount(dhtNodes)}; mesh peers: ${formatCount(meshPeers)}; hashes: ${formatCount(hashCount)}; seq: ${formatCount(seqId)}${telemetryErrors ? `; ${telemetryErrors}` : ''}`
      : 'Login to see slskr network stats';

    return (
      <footer
        className="slskr-footer"
        ref={this.footerRef}
      >
        <div className="slskr-footer-content">
          <div className="slskr-footer-left">
            <div className="slskr-footer-brand">
              <a
                className="slskr-footer-github"
                href={GITHUB_BASE}
                rel="noopener noreferrer"
                target="_blank"
                title="Open slskr on GitHub"
              >
                <img
                  alt=""
                  aria-hidden="true"
                  src={`${urlBase}/slskr-mark.png`}
                />
                <Icon name="github" />
                <span>GitHub</span>
              </a>

              <span
                aria-label="Support slskr development"
                className="slskr-footer-support"
              >
                <a
                  className="slskr-footer-sponsor paypal"
                  href="https://www.paypal.com/donate/?business=donations%40snape.tech"
                  rel="noopener noreferrer"
                  target="_blank"
                  title="Support slskr development with PayPal"
                >
                  <Icon name="paypal" /> PayPal
                </a>
                <a
                  className="slskr-footer-sponsor kofi"
                  href="https://ko-fi.com/snapetech"
                  rel="noopener noreferrer"
                  target="_blank"
                  title="Support slskr development on Ko-fi"
                >
                  <Icon name="coffee" /> Ko-fi
                </a>
              </span>

              <span className="slskr-footer-copyright">
                © {year}{' '}
                <a
                  href={GITHUB_BASE}
                  rel="noopener noreferrer"
                  target="_blank"
                  title="slskr project"
                >
                  slskr
                </a>
              </span>
              <a
                className={`slskr-footer-build ${isUpdateAvailable ? 'update-available' : ''}`}
                href={isUpdateAvailable ? latestUrl : `${GITHUB_BASE}/releases`}
                rel="noopener noreferrer"
                target="_blank"
                title={
                  isUpdateAvailable
                    ? `Running ${fullBuild}; GitHub has ${latestTag || latestBuild}`
                    : `Running ${fullBuild}`
                }
              >
                <Icon name={isUpdateAvailable ? 'bullhorn' : 'code branch'} />
                <span className="slskr-footer-build-label">Build</span>
                <code>{currentBuild}</code>
                {isUpdateAvailable && (
                  <span className="slskr-footer-update-label">
                    update {latestBuild}
                  </span>
                )}
              </a>
            </div>
          </div>

          <div className="slskr-footer-center">
            <div
              className={`slskr-footer-speeds ${isLoggedIn && speeds ? 'active' : ''}`}
              aria-label="Transfer speeds"
            >
              <span className="slskr-footer-group-label">Speed</span>
              <span
                className="slskr-footer-speed-item"
                title={
                  isLoggedIn
                    ? 'Total transfer speed (upload + download)'
                    : 'Login to see real-time speeds'
                }
              >
                <strong>T:</strong>{' '}
                <span className="speed-value">
                  {totalSpeed ? totalSpeed.value : '—'}
                </span>
                <span className="speed-unit">{totalSpeed ? totalSpeed.unit : ''}</span>
              </span>
              <span
                className="slskr-footer-speed-item"
                title={
                  isLoggedIn
                    ? 'Soulseek network speed'
                    : 'Login to see real-time speeds'
                }
              >
                <strong>S:</strong>{' '}
                <span className="speed-value">
                  {soulseekSpeed ? soulseekSpeed.value : '—'}
                </span>
                <span className="speed-unit">
                  {soulseekSpeed ? soulseekSpeed.unit : ''}
                </span>
              </span>
              <span
                className="slskr-footer-speed-item"
                title={
                  isLoggedIn
                    ? 'Mesh network speed'
                    : 'Login to see real-time speeds'
                }
              >
                <strong>M:</strong>{' '}
                <span className="speed-value">
                  {meshSpeed ? meshSpeed.value : '—'}
                </span>
                <span className="speed-unit">{meshSpeed ? meshSpeed.unit : ''}</span>
              </span>
            </div>
          </div>

          <div className="slskr-footer-right">
            <a
              className={`slskr-footer-network ${isLoggedIn && slskrStats ? 'active' : ''}`}
              href={`${urlBase}/system/network`}
              title={networkTooltip}
            >
              <span className="slskr-footer-group-label">Network</span>
              <span className="slskr-footer-network-item">
                <Icon
                  color={displayedDhtPeers > 0 ? 'green' : 'grey'}
                  name="rss"
                />
                {formatCount(displayedDhtPeers)} dht
              </span>
              <span className="slskr-footer-network-item">
                <Icon
                  color={meshPeers > 0 ? 'green' : 'grey'}
                  name="sitemap"
                />
                {formatCount(meshPeers)} mesh
              </span>
              <span className="slskr-footer-network-item">
                <Icon
                  color={hashCount > 0 ? 'blue' : 'grey'}
                  name="database"
                />
                {formatCount(hashCount)} hashes
              </span>
              <span
                className={`slskr-footer-network-item ${isSyncing ? 'syncing' : ''}`}
              >
                <Icon
                  color={isSyncing ? 'yellow' : 'grey'}
                  loading={isSyncing}
                  name="sync"
                />
                seq:{seqId}
              </span>
              {activeSwarms > 0 && (
                <span className="slskr-footer-network-item active">
                  <Icon name="bolt" />
                  {activeSwarms} swarm{activeSwarms === 1 ? '' : 's'}
                </span>
              )}
              {backfillActive && (
                <span className="slskr-footer-network-item active">
                  <Icon
                    loading
                    name="clock"
                  />
                  backfill
                </span>
              )}
              <span className="slskr-footer-network-item">
                <Icon name="trophy" />
                {karma > 0 ? '+' : ''}
                {karma}
              </span>
            </a>

            <div
              className="slskr-footer-stats"
              aria-label="Transport health"
            >
              <Icon
                className={
                  isDhtConnected
                    ? 'slskr-footer-stat-icon connected'
                    : 'slskr-footer-stat-icon'
                }
                name="sitemap"
                title={
                  isLoggedIn && stats
                    ? `DHT Nodes: ${stats.dht}`
                    : 'DHT: Login to see stats'
                }
              />
              <Icon
                className={
                  isNatResolved
                    ? 'slskr-footer-stat-icon connected'
                    : 'slskr-footer-stat-icon'
                }
                name="shield alternate"
                title={natTooltip}
              />
              <Icon
                className={
                  isOverlayConnected
                    ? 'slskr-footer-stat-icon connected'
                    : 'slskr-footer-stat-icon'
                }
                name="globe"
                title={
                  isLoggedIn && stats
                    ? `Overlay Peers: ${stats.overlay}`
                    : 'Overlay: Login to see stats'
                }
              />
            </div>
          </div>
        </div>
      </footer>
    );
  }
}

export default Footer;
