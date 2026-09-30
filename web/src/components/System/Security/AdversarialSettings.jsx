import './Security.css';
import * as securityApi from '../../../lib/security';
import { toDisplayError } from '../../../lib/errors';
import { useMountedRef } from '../../../lib/useMountedRef';
import AdversarialSettingsView from './AdversarialSettingsView';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Button, Dimmer, Loader, Message, Segment } from 'semantic-ui-react';

const AdversarialSettings = () => {
  const [activeIndex, setActiveIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState(null);
  const [success, setSuccess] = useState(null);
  const [settings, setSettings] = useState(null);
  const [hasChanges, setHasChanges] = useState(false);
  const [status, setStatus] = useState(null);
  const [statusError, setStatusError] = useState(null);
  const [statusLoading, setStatusLoading] = useState(false);
  const [transportStatus, setTransportStatus] = useState(null);
  const [transportError, setTransportError] = useState(null);
  const [transportLoading, setTransportLoading] = useState(false);
  const [torStatus, setTorStatus] = useState(null);
  const [torError, setTorError] = useState(null);
  const [torLoading, setTorLoading] = useState(false);
  const [refreshingAll, setRefreshingAll] = useState(false);
  const mountedRef = useMountedRef();
  const requestIdsRef = useRef({
    save: 0,
    settings: 0,
    status: 0,
    testTor: 0,
    testTransport: 0,
    tor: 0,
    transport: 0,
  });
  const refreshAllInFlightRef = useRef(false);

  const fetchSettings = useCallback(async () => {
    if (!mountedRef.current) return null;
    const requestId = ++requestIdsRef.current.settings;
    try {
      setLoading(true);
      const data = await securityApi.getAdversarialSettings();
      if (
        !mountedRef.current ||
        requestIdsRef.current.settings !== requestId
      ) {
        return null;
      }
      if (data) {
        setSettings(data);
        setError(null);
        return data;
      } else {
        setError('Adversarial features are not configured on this server');
      }
    } catch (fetchError) {
      if (
        mountedRef.current &&
        requestIdsRef.current.settings === requestId
      ) {
        setError(
          fetchError?.response?.status === 404
            ? 'Adversarial features are not configured on this server'
            : toDisplayError(fetchError, 'Failed to load adversarial settings'),
        );
      }
    } finally {
      if (
        mountedRef.current &&
        requestIdsRef.current.settings === requestId
      ) {
        setLoading(false);
      }
    }

    return null;
  }, []);

  const fetchStatus = useCallback(async () => {
    if (!mountedRef.current) return;
    const requestId = ++requestIdsRef.current.status;
    try {
      setStatusLoading(true);
      const statusData = await securityApi.getAdversarialStats();
      if (
        mountedRef.current &&
        requestIdsRef.current.status === requestId
      ) {
        if (statusData) {
          setStatus(statusData);
          setStatusError(null);
        } else {
          setStatus(null);
          setStatusError('Anonymity status is unavailable');
        }
      }
    } catch (statusError) {
      if (mountedRef.current && requestIdsRef.current.status === requestId) {
        setStatus(null);
        setStatusError(
          toDisplayError(statusError, 'Failed to load adversarial status'),
        );
      }
    } finally {
      if (mountedRef.current && requestIdsRef.current.status === requestId) {
        setStatusLoading(false);
      }
    }
  }, []);

  const fetchTransportStatus = useCallback(async () => {
    if (!mountedRef.current) return;
    const requestId = ++requestIdsRef.current.transport;
    try {
      setTransportLoading(true);
      const transportData = await securityApi.getTransportStatus();
      if (
        mountedRef.current &&
        requestIdsRef.current.transport === requestId
      ) {
        if (transportData) {
          setTransportStatus(transportData);
          setTransportError(null);
        } else {
          setTransportStatus(null);
          setTransportError('Transport status is unavailable');
        }
      }
    } catch (transportError) {
      if (
        mountedRef.current &&
        requestIdsRef.current.transport === requestId
      ) {
        setTransportStatus(null);
        setTransportError(
          toDisplayError(transportError, 'Failed to load transport status'),
        );
      }
    } finally {
      if (
        mountedRef.current &&
        requestIdsRef.current.transport === requestId
      ) {
        setTransportLoading(false);
      }
    }
  }, []);

  const fetchTorStatus = useCallback(async () => {
    if (!mountedRef.current) return;
    const requestId = ++requestIdsRef.current.tor;
    try {
      setTorLoading(true);
      const torData = await securityApi.getTorStatus();
      if (
        mountedRef.current &&
        requestIdsRef.current.tor === requestId
      ) {
        if (torData) {
          setTorStatus(torData);
          setTorError(null);
        } else {
          setTorStatus(null);
          setTorError('Tor status is unavailable');
        }
      }
    } catch (torError) {
      if (mountedRef.current && requestIdsRef.current.tor === requestId) {
        setTorStatus(null);
        setTorError(toDisplayError(torError, 'Failed to load Tor status'));
      }
    } finally {
      if (mountedRef.current && requestIdsRef.current.tor === requestId) {
        setTorLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    const initialize = async () => {
      const configuredSettings = await fetchSettings();
      if (!mountedRef.current) return;
      void fetchStatus();

      const enabled = configuredSettings?.Enabled ?? configuredSettings?.enabled;
      if (!enabled) return;

      void fetchTransportStatus();
      const anonymity = configuredSettings.Anonymity || configuredSettings.anonymity;
      const anonymityEnabled = anonymity?.Enabled ?? anonymity?.enabled;
      const anonymityMode = anonymity?.Mode ?? anonymity?.mode;
      if (anonymityEnabled && anonymityMode === 'Tor') {
        void fetchTorStatus();
      }
    };

    void initialize();
  }, [fetchSettings, fetchStatus, fetchTorStatus, fetchTransportStatus]);

  const handleSave = async () => {
    if (!settings || !mountedRef.current || saving) return;
    const requestId = ++requestIdsRef.current.save;
    const isCurrentRequest = () =>
      mountedRef.current && requestIdsRef.current.save === requestId;

    try {
      setSaving(true);
      setError(null);
      setSuccess(null);

      await securityApi.updateAdversarialSettings(settings);
      if (isCurrentRequest()) {
        setSuccess('Adversarial settings updated successfully');
        setHasChanges(false);
      }
    } catch (saveError) {
      if (isCurrentRequest()) {
        setError(toDisplayError(saveError, 'Failed to save adversarial settings'));
      }
    } finally {
      if (isCurrentRequest()) setSaving(false);
    }
  };

  const refreshAll = async () => {
    if (!mountedRef.current || refreshingAll || refreshAllInFlightRef.current) return;
    refreshAllInFlightRef.current = true;
    setRefreshingAll(true);
    try {
      await Promise.allSettled([
        fetchSettings(),
        fetchStatus(),
        fetchTransportStatus(),
        fetchTorStatus(),
      ]);
    } finally {
      refreshAllInFlightRef.current = false;
      if (mountedRef.current) setRefreshingAll(false);
    }
  };

  const testTransportConnectivity = async () => {
    const requestId = ++requestIdsRef.current.testTransport;
    try {
      await securityApi.testTransportConnectivity();
      if (
        mountedRef.current &&
        requestIdsRef.current.testTransport === requestId
      ) {
        setSuccess('Transport connectivity test completed');
        void fetchTransportStatus();
      }
    } catch (error) {
      if (
        mountedRef.current &&
        requestIdsRef.current.testTransport === requestId
      ) {
        setError(toDisplayError(error, 'Transport test failed'));
      }
    }
  };

  const testTorConnectivity = async () => {
    const requestId = ++requestIdsRef.current.testTor;
    try {
      await securityApi.testTorConnectivity();
      if (
        mountedRef.current &&
        requestIdsRef.current.testTor === requestId
      ) {
        setSuccess('Tor connectivity test completed');
        void fetchTorStatus();
      }
    } catch (error) {
      if (
        mountedRef.current &&
        requestIdsRef.current.testTor === requestId
      ) {
        setError(toDisplayError(error, 'Tor test failed'));
      }
    }
  };

  const cloneSettings = (previous) => JSON.parse(JSON.stringify(previous || {}));

  const ensurePrivacyPadding = (settings_) => {
    settings_.Privacy = settings_.Privacy || {};
    settings_.Privacy.Padding = settings_.Privacy.Padding || {};
    return settings_.Privacy.Padding;
  };

  const ensurePrivacyTiming = (settings_) => {
    settings_.Privacy = settings_.Privacy || {};
    settings_.Privacy.Timing = settings_.Privacy.Timing || {};
    return settings_.Privacy.Timing;
  };

  const ensurePrivacyBatching = (settings_) => {
    settings_.Privacy = settings_.Privacy || {};
    settings_.Privacy.Batching = settings_.Privacy.Batching || {};
    return settings_.Privacy.Batching;
  };

  const ensurePrivacyCoverTraffic = (settings_) => {
    settings_.Privacy = settings_.Privacy || {};
    settings_.Privacy.CoverTraffic = settings_.Privacy.CoverTraffic || {};
    return settings_.Privacy.CoverTraffic;
  };

  const ensureAnonymityTor = (settings_) => {
    settings_.Anonymity = settings_.Anonymity || {};
    settings_.Anonymity.Tor = settings_.Anonymity.Tor || {};
    return settings_.Anonymity.Tor;
  };

  const ensureAnonymityI2P = (settings_) => {
    settings_.Anonymity = settings_.Anonymity || {};
    settings_.Anonymity.I2P = settings_.Anonymity.I2P || {};
    return settings_.Anonymity.I2P;
  };

  const ensureAnonymityRelayOnly = (settings_) => {
    settings_.Anonymity = settings_.Anonymity || {};
    settings_.Anonymity.RelayOnly = settings_.Anonymity.RelayOnly || {};
    return settings_.Anonymity.RelayOnly;
  };

  const ensureTransportWebSocket = (settings_) => {
    settings_.Transport = settings_.Transport || {};
    settings_.Transport.WebSocket = settings_.Transport.WebSocket || {};
    return settings_.Transport.WebSocket;
  };

  const ensureTransportHttpTunnel = (settings_) => {
    settings_.Transport = settings_.Transport || {};
    settings_.Transport.HttpTunnel = settings_.Transport.HttpTunnel || {};
    return settings_.Transport.HttpTunnel;
  };

  const ensureTransportObfs4 = (settings_) => {
    settings_.Transport = settings_.Transport || {};
    settings_.Transport.Obfs4 = settings_.Transport.Obfs4 || {};
    return settings_.Transport.Obfs4;
  };

  const ensureTransportMeek = (settings_) => {
    settings_.Transport = settings_.Transport || {};
    settings_.Transport.Meek = settings_.Transport.Meek || {};
    return settings_.Transport.Meek;
  };

  const updateSetting = (path, value) => {
    setSettings((previous) => {
      const newSettings = cloneSettings(previous);

      switch (path) {
        case 'Profile':
          newSettings.Profile = value;
          break;
        case 'Enabled':
          newSettings.Enabled = value;
          break;
        case 'Privacy.Enabled':
          newSettings.Privacy = newSettings.Privacy || {};
          newSettings.Privacy.Enabled = value;
          break;
        case 'Privacy.Padding.Enabled':
          ensurePrivacyPadding(newSettings).Enabled = value;
          break;
        case 'Privacy.Padding.UseRandomFill':
          ensurePrivacyPadding(newSettings).UseRandomFill = value;
          break;
        case 'Privacy.Timing.Enabled':
          ensurePrivacyTiming(newSettings).Enabled = value;
          break;
        case 'Privacy.Timing.JitterMs':
          ensurePrivacyTiming(newSettings).JitterMs = value;
          break;
        case 'Privacy.Batching.Enabled':
          ensurePrivacyBatching(newSettings).Enabled = value;
          break;
        case 'Privacy.Batching.BatchWindowMs':
          ensurePrivacyBatching(newSettings).BatchWindowMs = value;
          break;
        case 'Privacy.CoverTraffic.Enabled':
          ensurePrivacyCoverTraffic(newSettings).Enabled = value;
          break;
        case 'Privacy.CoverTraffic.IntervalSeconds':
          ensurePrivacyCoverTraffic(newSettings).IntervalSeconds = value;
          break;
        case 'Anonymity.Enabled':
          newSettings.Anonymity = newSettings.Anonymity || {};
          newSettings.Anonymity.Enabled = value;
          break;
        case 'Anonymity.Mode':
          newSettings.Anonymity = newSettings.Anonymity || {};
          newSettings.Anonymity.Mode = value;
          break;
        case 'Anonymity.Tor.SocksAddress':
          ensureAnonymityTor(newSettings).SocksAddress = value;
          break;
        case 'Anonymity.Tor.IsolateStreams':
          ensureAnonymityTor(newSettings).IsolateStreams = value;
          break;
        case 'Anonymity.I2P.SamAddress':
          ensureAnonymityI2P(newSettings).SamAddress = value;
          break;
        case 'Anonymity.RelayOnly.MaxChainLength':
          ensureAnonymityRelayOnly(newSettings).MaxChainLength = value;
          break;
        case 'Transport.Enabled':
          newSettings.Transport = newSettings.Transport || {};
          newSettings.Transport.Enabled = value;
          break;
        case 'Transport.PrimaryTransport':
          newSettings.Transport = newSettings.Transport || {};
          newSettings.Transport.PrimaryTransport = value;
          break;
        case 'Transport.WebSocket.ServerUrl':
          ensureTransportWebSocket(newSettings).ServerUrl = value;
          break;
        case 'Transport.WebSocket.UseWss':
          ensureTransportWebSocket(newSettings).UseWss = value;
          break;
        case 'Transport.HttpTunnel.ProxyUrl':
          ensureTransportHttpTunnel(newSettings).ProxyUrl = value;
          break;
        case 'Transport.HttpTunnel.Method':
          ensureTransportHttpTunnel(newSettings).Method = value;
          break;
        case 'Transport.HttpTunnel.UseHttps':
          ensureTransportHttpTunnel(newSettings).UseHttps = value;
          break;
        case 'Transport.Obfs4.Obfs4ProxyPath':
          ensureTransportObfs4(newSettings).Obfs4ProxyPath = value;
          break;
        case 'Transport.Obfs4.BridgeLines':
          ensureTransportObfs4(newSettings).BridgeLines = value;
          break;
        case 'Transport.Meek.BridgeUrl':
          ensureTransportMeek(newSettings).BridgeUrl = value;
          break;
        case 'Transport.Meek.FrontDomain':
          ensureTransportMeek(newSettings).FrontDomain = value;
          break;
        default:
          return newSettings;
      }
      return newSettings;
    });
    setHasChanges(true);
  };

  const updateArray = (path, index, value) => {
    setSettings((previous) => {
      const newSettings = cloneSettings(previous);
      let items;

      switch (path) {
        case 'Privacy.Padding.BucketSizes':
          items = ensurePrivacyPadding(newSettings).BucketSizes || [];
          ensurePrivacyPadding(newSettings).BucketSizes = items;
          break;
        case 'Transport.Obfs4.BridgeLines':
          items = ensureTransportObfs4(newSettings).BridgeLines || [];
          ensureTransportObfs4(newSettings).BridgeLines = items;
          break;
        default:
          return newSettings;
      }
      items[index] = value;
      return newSettings;
    });
    setHasChanges(true);
  };

  const addArrayItem = (path) => {
    setSettings((previous) => {
      const newSettings = cloneSettings(previous);

      switch (path) {
        case 'Privacy.Padding.BucketSizes':
          ensurePrivacyPadding(newSettings).BucketSizes = [
            ...(ensurePrivacyPadding(newSettings).BucketSizes || []),
            '',
          ];
          break;
        case 'Transport.Obfs4.BridgeLines':
          ensureTransportObfs4(newSettings).BridgeLines = [
            ...(ensureTransportObfs4(newSettings).BridgeLines || []),
            '',
          ];
          break;
        default:
          return newSettings;
      }
      return newSettings;
    });
    setHasChanges(true);
  };

  const removeArrayItem = (path, index) => {
    setSettings((previous) => {
      const newSettings = cloneSettings(previous);

      switch (path) {
        case 'Privacy.Padding.BucketSizes':
          ensurePrivacyPadding(newSettings).BucketSizes = (
            ensurePrivacyPadding(newSettings).BucketSizes || []
          ).filter((_, itemIndex) => itemIndex !== index);
          break;
        case 'Transport.Obfs4.BridgeLines':
          ensureTransportObfs4(newSettings).BridgeLines = (
            ensureTransportObfs4(newSettings).BridgeLines || []
          ).filter((_, itemIndex) => itemIndex !== index);
          break;
        default:
          return newSettings;
      }

      return newSettings;
    });
    setHasChanges(true);
  };

  if (loading) {
    return (
      <Segment placeholder>
        <Dimmer
          active
          inverted
        >
          <Loader>Loading Adversarial Settings...</Loader>
        </Dimmer>
      </Segment>
    );
  }

  if (error && !settings) {
    return (
      <Message negative>
        <Message.Header>Adversarial Features Unavailable</Message.Header>
        <p>{error}</p>
        <p>
          Adversarial features are advanced security options designed for users
          in adversarial environments. They are disabled by default and require
          explicit configuration.
        </p>
        <Button
          onClick={fetchSettings}
          size="small"
        >
          Retry
        </Button>
      </Message>
    );
  }

  if (!settings) {
    return (
      <Message info>
        <Message.Header>Adversarial Settings Not Configured</Message.Header>
        <p>No adversarial configuration found.</p>
      </Message>
    );
  }

  const viewState = {
    activeIndex,
    error,
    hasChanges,
    loading,
    refreshingAll,
    saving,
    settings,
    status,
    statusError,
    statusLoading,
    success,
    torError,
    torLoading,
    torStatus,
    transportError,
    transportLoading,
    transportStatus,
  };
  const viewActions = {
    addArrayItem,
    fetchTorStatus,
    fetchTransportStatus,
    handleSave,
    refreshAll,
    removeArrayItem,
    setActiveIndex,
    testTorConnectivity,
    testTransportConnectivity,
    updateArray,
    updateSetting,
  };

  return <AdversarialSettingsView actions={viewActions} state={viewState} />;
};

export default AdversarialSettings;
