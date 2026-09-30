import * as optionsApi from '../../../lib/options';
import { toDisplayError } from '../../../lib/errors';
import { useMountedRef } from '../../../lib/useMountedRef';
import AdminPoliciesForm from './AdminPoliciesForm';
import * as YAML from 'yaml';
import { useEffect, useRef, useState } from 'react';

const getOption = (source, ...keys) => {
  for (const key of keys) {
    if (source && Object.prototype.hasOwnProperty.call(source, key)) {
      return source[key];
    }
  }

  return undefined;
};

const getAuthOptions = (options = {}) =>
  getOption(getOption(options, 'web', 'Web') || {}, 'authentication', 'Authentication') ||
  {};

const getTransferOptions = (options = {}) =>
  getOption(options, 'transfers', 'Transfers', 'global', 'Global') || {};

const getUploadOptions = (options = {}) =>
  getOption(getTransferOptions(options), 'upload', 'Upload') || {};

const getDownloadOptions = (options = {}) =>
  getOption(getTransferOptions(options), 'download', 'Download') || {};

const getIntegrationOptions = (options = {}) =>
  getOption(options, 'integration', 'Integration', 'integrations', 'Integrations') ||
  {};

const getShareOptions = (options = {}) => getOption(options, 'shares', 'Shares') || {};

const isConfigured = (value) =>
  value !== undefined && value !== null && value !== '';

const toLines = (value = []) => {
  if (Array.isArray(value)) {
    return value.join('\n');
  }

  return String(value || '');
};

const parseLines = (value = '') =>
  String(value)
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);

const toNumber = (value, fallback) => {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
};

const buildForm = (options = {}) => {
  const auth = getAuthOptions(options);
  const jwt = getOption(auth, 'jwt', 'Jwt') || {};
  const passthrough = getOption(auth, 'passthrough', 'Passthrough') || {};
  const apiKeys = getOption(auth, 'apiKeys', 'api_keys', 'ApiKeys') || {};
  const web = getOption(options, 'web', 'Web') || {};
  const https = getOption(web, 'https', 'Https', 'HTTPS') || {};
  const certificate = getOption(https, 'certificate', 'Certificate') || {};
  const rateLimiting = getOption(web, 'rateLimiting', 'rate_limiting', 'RateLimiting') || {};
  const upload = getUploadOptions(options);
  const download = getDownloadOptions(options);
  const autoReplace = getOption(options, 'autoReplace', 'auto_replace', 'AutoReplace') || {};
  const scheduledLimits =
    getOption(options, 'scheduledLimits', 'scheduled_limits', 'ScheduledLimits') || {};
  const filters = getOption(options, 'filters', 'Filters') || {};
  const searchFilters = getOption(filters, 'search', 'Search') || {};
  const searchRetention =
    getOption(filters, 'searchRetention', 'search_retention', 'SearchRetention') || {};
  const blacklist = getOption(options, 'blacklist', 'Blacklist') || {};
  const throttling = getOption(options, 'throttling', 'Throttling') || {};
  const incoming =
    getOption(
      getOption(getOption(throttling, 'search', 'Search') || {}, 'incoming', 'Incoming') ||
        {},
      'incoming',
      'Incoming',
    ) || getOption(getOption(throttling, 'search', 'Search') || {}, 'incoming', 'Incoming') || {};
  const retention = getOption(options, 'retention', 'Retention') || {};
  const retentionTransfers = getOption(retention, 'transfers', 'Transfers') || {};
  const retentionUpload = getOption(retentionTransfers, 'upload', 'Upload') || {};
  const retentionDownload = getOption(retentionTransfers, 'download', 'Download') || {};
  const retentionFiles = getOption(retention, 'files', 'Files') || {};
  const sharesCache = getOption(getShareOptions(options), 'cache', 'Cache') || {};
  const webhooks = getOption(getIntegrationOptions(options), 'webhooks', 'Webhooks') || {};
  const scripts = getOption(getIntegrationOptions(options), 'scripts', 'Scripts') || {};
  const firstWebhookName = Object.keys(webhooks)[0] || '';
  const firstWebhook = webhooks[firstWebhookName] || {};
  const firstWebhookCall = getOption(firstWebhook, 'call', 'Call') || {};
  const firstWebhookRetry = getOption(firstWebhook, 'retry', 'Retry') || {};
  const firstScriptName = Object.keys(scripts)[0] || '';
  const firstScript = scripts[firstScriptName] || {};
  const firstScriptRun = getOption(firstScript, 'run', 'Run') || {};
  const firstApiKeyName = Object.keys(apiKeys)[0] || 'automation';
  const firstApiKey = apiKeys[firstApiKeyName] || {};
  const retry = getOption(download, 'retry', 'Retry') || {};
  const uploadScheduledLimits =
    getOption(upload, 'scheduledLimits', 'scheduled_limits', 'ScheduledLimits') ||
    scheduledLimits;
  const downloadScheduledLimits =
    getOption(download, 'scheduledLimits', 'scheduled_limits', 'ScheduledLimits') ||
    scheduledLimits;
  const shares = getShareOptions(options);
  const features = getOption(options, 'feature', 'Feature', 'features', 'Features') || {};
  const dht = getOption(options, 'dht', 'dhtRendezvous', 'DhtRendezvous') || {};
  const rescueMode = getOption(options, 'rescueMode', 'rescue_mode', 'RescueMode') || {};

  return {
    allowRemoteNoAuth: Boolean(getOption(web, 'allowRemoteNoAuth', 'allow_remote_no_auth', 'AllowRemoteNoAuth')),
    apiKeyCidr: getOption(firstApiKey, 'cidr', 'Cidr') || '127.0.0.1/32,::1/128',
    apiKeyConfigured: isConfigured(getOption(firstApiKey, 'key', 'Key')),
    apiKeyName: firstApiKeyName,
    apiKeyRole: getOption(firstApiKey, 'role', 'Role') || 'ReadOnly',
    apiKeyScopes: getOption(firstApiKey, 'scopes', 'Scopes') || '*',
    apiKeyValue: '',
    autoReplaceInterval: String(
      getOption(autoReplace, 'intervalSeconds', 'interval_seconds', 'IntervalSeconds') ??
        getOption(download, 'autoReplaceInterval', 'auto_replace_interval') ??
        300,
    ),
    autoReplaceMaxRetries: String(
      getOption(autoReplace, 'maxRetries', 'max_retries', 'MaxRetries') ?? 3,
    ),
    autoReplaceStuck: Boolean(getOption(download, 'autoReplaceStuck', 'auto_replace_stuck')),
    autoReplaceThreshold: String(
      getOption(
        autoReplace,
        'sizeThresholdPercent',
        'size_threshold_percent',
        'SizeThresholdPercent',
      ) ??
        getOption(download, 'autoReplaceThreshold', 'auto_replace_threshold') ??
        0,
    ),
    blacklistEnabled: Boolean(getOption(blacklist, 'enabled', 'Enabled')),
    blacklistFile: getOption(blacklist, 'file', 'File') || '',
    downloadSlots: String(getOption(download, 'slots', 'Slots') ?? 500),
    downloadSpeedLimit: String(getOption(download, 'speedLimit', 'speed_limit', 'SpeedLimit') ?? 0),
    downloadRetryAttempts: String(getOption(retry, 'attempts', 'Attempts') ?? 1),
    downloadRetryDelay: String(getOption(retry, 'delay', 'Delay') ?? 5000),
    downloadRetryIncomplete: getOption(retry, 'incomplete', 'Incomplete') || 'resume',
    downloadRetryMaxDelay: String(getOption(retry, 'maxDelay', 'max_delay', 'MaxDelay') ?? 60000),
    downloadScheduledLimitsEnabled: Boolean(getOption(downloadScheduledLimits, 'enabled', 'Enabled')),
    dhtAnnounceIntervalSeconds: String(getOption(dht, 'announceIntervalSeconds', 'announce_interval_seconds', 'AnnounceIntervalSeconds') ?? 900),
    dhtBootstrapRouters: toLines(getOption(dht, 'bootstrapRouters', 'bootstrap_routers', 'BootstrapRouters') || []),
    dhtEnabled: getOption(dht, 'enabled', 'Enabled') ?? true,
    dhtLanOnly: Boolean(getOption(dht, 'lanOnly', 'lan_only', 'LanOnly')),
    dhtPort: String(getOption(dht, 'dhtPort', 'dht_port', 'DhtPort') ?? 50300),
    dhtOverlayPort: String(getOption(dht, 'overlayPort', 'overlay_port', 'OverlayPort') ?? 50305),
    enforceSecurity: Boolean(getOption(web, 'enforceSecurity', 'enforce_security', 'EnforceSecurity')),
    eventsRetention: String(getOption(retention, 'events', 'Events') ?? 30),
    featureScenePodBridge: Boolean(getOption(features, 'scenePodBridge', 'ScenePodBridge')),
    fileCompleteRetention: String(getOption(retentionFiles, 'complete', 'Complete') ?? ''),
    fileIncompleteRetention: String(getOption(retentionFiles, 'incomplete', 'Incomplete') ?? ''),
    forceHttps: Boolean(getOption(https, 'force', 'Force')),
    httpsCertificatePassword: '',
    httpsCertificatePasswordConfigured: isConfigured(
      getOption(certificate, 'password', 'Password'),
    ),
    httpsCertificatePfx: getOption(certificate, 'pfx', 'Pfx') || '',
    httpsDisabled: getOption(https, 'disabled', 'Disabled') ?? true,
    httpsPort: String(getOption(https, 'port', 'Port') ?? 5031),
    jwtConfigured: isConfigured(getOption(jwt, 'key', 'Key')),
    jwtKey: '',
    jwtTtl: String(getOption(jwt, 'ttl', 'Ttl') ?? 3600000),
    logRetention: String(getOption(retention, 'logs', 'Logs') ?? 180),
    noAuth: Boolean(getOption(auth, 'disabled', 'Disabled')),
    passthroughCidrs: getOption(passthrough, 'allowedCidrs', 'allowed_cidrs', 'AllowedCidrs') || '',
    rateLimitApi: String(getOption(rateLimiting, 'apiPermitLimit', 'api_permit_limit', 'ApiPermitLimit') ?? 200),
    rateLimitApiWindow: String(getOption(rateLimiting, 'apiWindowSeconds', 'api_window_seconds', 'ApiWindowSeconds') ?? 60),
    rateLimitEnabled: getOption(rateLimiting, 'enabled', 'Enabled') ?? true,
    rateLimitFederation: String(
      getOption(rateLimiting, 'federationPermitLimit', 'federation_permit_limit', 'FederationPermitLimit') ?? 30,
    ),
    rateLimitFederationWindow: String(
      getOption(rateLimiting, 'federationWindowSeconds', 'federation_window_seconds', 'FederationWindowSeconds') ?? 60,
    ),
    rateLimitMesh: String(
      getOption(rateLimiting, 'meshGatewayPermitLimit', 'mesh_gateway_permit_limit', 'MeshGatewayPermitLimit') ?? 60,
    ),
    rateLimitMeshWindow: String(
      getOption(rateLimiting, 'meshGatewayWindowSeconds', 'mesh_gateway_window_seconds', 'MeshGatewayWindowSeconds') ?? 60,
    ),
    retentionDownloadCancelled: String(getOption(retentionDownload, 'cancelled', 'Cancelled') ?? ''),
    retentionDownloadErrored: String(getOption(retentionDownload, 'errored', 'Errored') ?? ''),
    retentionDownloadSucceeded: String(getOption(retentionDownload, 'succeeded', 'Succeeded') ?? ''),
    retentionSearch: String(getOption(retention, 'search', 'Search') ?? ''),
    searchRetentionMaxAgeDays: String(
      getOption(searchRetention, 'maxAgeDays', 'max_age_days', 'MaxAgeDays') ?? 30,
    ),
    searchRetentionMaxCount: String(
      getOption(searchRetention, 'maxCount', 'max_count', 'MaxCount') ?? 1000,
    ),
    searchRetentionCleanupInterval: String(
      getOption(
        searchRetention,
        'cleanupIntervalSeconds',
        'cleanup_interval_seconds',
        'CleanupIntervalSeconds',
      ) ?? 86400,
    ),
    retentionUploadCancelled: String(getOption(retentionUpload, 'cancelled', 'Cancelled') ?? ''),
    retentionUploadErrored: String(getOption(retentionUpload, 'errored', 'Errored') ?? ''),
    retentionUploadSucceeded: String(getOption(retentionUpload, 'succeeded', 'Succeeded') ?? ''),
    rescueModeEnabled: Boolean(getOption(rescueMode, 'enabled', 'Enabled')),
    rescueModeMaxQueueSeconds: String(getOption(rescueMode, 'maxQueueTimeSeconds', 'max_queue_time_seconds', 'MaxQueueTimeSeconds') ?? 1800),
    scheduledLimitsEnabled: Boolean(getOption(scheduledLimits, 'enabled', 'Enabled')),
    scriptArgs: getOption(firstScriptRun, 'args', 'Args') || '',
    scriptCommand: getOption(firstScriptRun, 'command', 'Command') || '',
    scriptEvents: toLines(getOption(firstScript, 'on', 'On') || []),
    scriptExecutable: getOption(firstScriptRun, 'executable', 'Executable') || '',
    scriptName: firstScriptName,
    searchFilterRequest: toLines(getOption(searchFilters, 'request', 'Request') || []),
    searchIncomingCircuitBreaker: String(
      getOption(incoming, 'circuitBreaker', 'circuit_breaker', 'CircuitBreaker') ?? 500,
    ),
    searchIncomingConcurrency: String(getOption(incoming, 'concurrency', 'Concurrency') ?? 10),
    searchIncomingResponseFileLimit: String(
      getOption(incoming, 'responseFileLimit', 'response_file_limit', 'ResponseFileLimit') ?? 500,
    ),
    shareCacheRetention: String(getOption(sharesCache, 'retention', 'Retention') ?? ''),
    shareCacheWorkers: String(getOption(sharesCache, 'workers', 'Workers') ?? 4),
    shareProbeMediaAttributes: getOption(shares, 'probeMediaAttributes', 'probe_media_attributes', 'ProbeMediaAttributes') ?? true,
    uploadSlots: String(getOption(upload, 'slots', 'Slots') ?? 20),
    uploadSpeedLimit: String(getOption(upload, 'speedLimit', 'speed_limit', 'SpeedLimit') ?? 0),
    uploadScheduledLimitsEnabled: Boolean(getOption(uploadScheduledLimits, 'enabled', 'Enabled')),
    webhookEvents: toLines(getOption(firstWebhook, 'on', 'On') || []),
    webhookIgnoreCertificateErrors: Boolean(getOption(firstWebhookCall, 'ignoreCertificateErrors', 'ignore_certificate_errors', 'IgnoreCertificateErrors')),
    webhookName: firstWebhookName,
    webhookRetryAttempts: String(
      getOption(firstWebhookRetry, 'attempts', 'Attempts') ?? 1,
    ),
    webhookTimeout: String(getOption(firstWebhook, 'timeout', 'Timeout') ?? 5000),
    webhookUrl: getOption(firstWebhookCall, 'url', 'Url') || '',
  };
};

const setOptionalNumber = (document, path, value) => {
  const trimmed = String(value ?? '').trim();
  document.setIn(path, trimmed ? toNumber(trimmed, null) : null);
};

const AdminPolicies = ({ options = {} }) => {
  const remoteConfiguration = Boolean(
    getOption(options, 'remoteConfiguration', 'RemoteConfiguration'),
  );
  const [form, setForm] = useState(() => buildForm(options));
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState(null);
  const mountedRef = useMountedRef();
  const saveRequestIdRef = useRef(0);

  useEffect(() => {
    setForm(buildForm(options));
  }, [options]);

  const update = (key, value) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const missing = [
    form.webhookUrl.trim() &&
      !form.webhookName.trim() &&
      'Webhook settings need a stable name.',
    form.webhookName.trim() &&
      !form.webhookUrl.trim() &&
      'Webhook settings need a target URL.',
    form.scriptCommand.trim() &&
      !form.scriptName.trim() &&
      'Script settings need a stable name.',
    form.scriptExecutable.trim() &&
      !form.scriptName.trim() &&
      'Script settings need a stable name.',
    form.scriptName.trim() &&
      !form.scriptCommand.trim() &&
      !form.scriptExecutable.trim() &&
      'Script settings need either a command or an executable.',
    form.scriptCommand.trim() &&
      form.scriptExecutable.trim() &&
      'Script settings must use either a command or an executable, not both.',
    form.noAuth &&
      form.allowRemoteNoAuth &&
      !form.passthroughCidrs.trim() &&
      'Remote no-auth mode needs an explicit CIDR allowlist.',
    form.blacklistEnabled &&
      !form.blacklistFile.trim() &&
      'Managed blacklist needs a file path.',
  ].filter(Boolean);

  const reset = () => {
    setForm(buildForm(options));
    setMessage(null);
  };

  const saveYaml = async () => {
    if (!mountedRef.current || saving) return;
    const requestId = ++saveRequestIdRef.current;
    setSaving(true);
    setMessage(null);

    try {
      const yaml = await optionsApi.getYaml();
      const document = YAML.parseDocument(yaml || '{}');

      if (form.webhookName.trim()) {
        const base = ['integrations', 'webhooks', form.webhookName.trim()];
        document.setIn([...base, 'on'], parseLines(form.webhookEvents));
        document.setIn([...base, 'call', 'url'], form.webhookUrl.trim());
        document.setIn(
          [...base, 'call', 'ignore_certificate_errors'],
          form.webhookIgnoreCertificateErrors,
        );
        document.setIn([...base, 'timeout'], toNumber(form.webhookTimeout, 5000));
        document.setIn([...base, 'retry', 'attempts'], toNumber(form.webhookRetryAttempts, 1));
      }

      if (form.scriptName.trim()) {
        const base = ['integrations', 'scripts', form.scriptName.trim()];
        document.setIn([...base, 'on'], parseLines(form.scriptEvents));
        document.setIn([...base, 'run', 'command'], form.scriptCommand.trim());
        document.setIn([...base, 'run', 'executable'], form.scriptExecutable.trim());
        document.setIn([...base, 'run', 'args'], form.scriptArgs.trim());
      }

      document.setIn(['transfers', 'upload', 'slots'], toNumber(form.uploadSlots, 20));
      document.setIn(
        ['transfers', 'upload', 'speed_limit'],
        toNumber(form.uploadSpeedLimit, 0),
      );
      document.setIn(['transfers', 'download', 'slots'], toNumber(form.downloadSlots, 500));
      document.setIn(
        ['transfers', 'download', 'speed_limit'],
        toNumber(form.downloadSpeedLimit, 0),
      );
      document.setIn(
        ['transfers', 'download', 'retry', 'incomplete'],
        form.downloadRetryIncomplete,
      );
      document.setIn(
        ['transfers', 'download', 'retry', 'attempts'],
        toNumber(form.downloadRetryAttempts, 1),
      );
      document.setIn(
        ['transfers', 'download', 'retry', 'delay'],
        toNumber(form.downloadRetryDelay, 5000),
      );
      document.setIn(
        ['transfers', 'download', 'retry', 'max_delay'],
        toNumber(form.downloadRetryMaxDelay, 60000),
      );
      document.setIn(['transfers', 'download', 'auto_replace_stuck'], form.autoReplaceStuck);
      document.setIn(
        ['transfers', 'download', 'auto_replace_threshold'],
        Number.parseFloat(form.autoReplaceThreshold) || 0,
      );
      document.setIn(
        ['transfers', 'download', 'auto_replace_interval'],
        toNumber(form.autoReplaceInterval, 60),
      );
      document.setIn(['auto_replace', 'interval_seconds'], toNumber(form.autoReplaceInterval, 300));
      document.setIn(
        ['auto_replace', 'size_threshold_percent'],
        Number.parseFloat(form.autoReplaceThreshold) || 0,
      );
      document.setIn(['auto_replace', 'max_retries'], toNumber(form.autoReplaceMaxRetries, 3));
      document.setIn(
        ['transfers', 'upload', 'scheduled_limits', 'enabled'],
        form.uploadScheduledLimitsEnabled,
      );
      document.setIn(
        ['transfers', 'download', 'scheduled_limits', 'enabled'],
        form.downloadScheduledLimitsEnabled,
      );

      document.setIn(['web', 'enforce_security'], form.enforceSecurity);
      document.setIn(['web', 'allow_remote_no_auth'], form.allowRemoteNoAuth);
      document.setIn(['web', 'authentication', 'disabled'], form.noAuth);
      document.setIn(['web', 'authentication', 'jwt', 'ttl'], toNumber(form.jwtTtl, 3600000));
      if (form.jwtKey.trim()) {
        document.setIn(['web', 'authentication', 'jwt', 'key'], form.jwtKey.trim());
      }
      if (form.apiKeyName.trim()) {
        const base = ['web', 'authentication', 'api_keys', form.apiKeyName.trim()];
        if (form.apiKeyValue.trim()) {
          document.setIn([...base, 'key'], form.apiKeyValue.trim());
        }
        document.setIn([...base, 'role'], form.apiKeyRole);
        document.setIn([...base, 'cidr'], form.apiKeyCidr.trim());
        document.setIn([...base, 'scopes'], form.apiKeyScopes.trim() || '*');
      }
      document.setIn(
        ['web', 'authentication', 'passthrough', 'allowed_cidrs'],
        form.passthroughCidrs.trim(),
      );
      document.setIn(['web', 'https', 'disabled'], form.httpsDisabled);
      document.setIn(['web', 'https', 'port'], toNumber(form.httpsPort, 5031));
      document.setIn(['web', 'https', 'force'], form.forceHttps);
      document.setIn(['web', 'https', 'certificate', 'pfx'], form.httpsCertificatePfx.trim());
      if (form.httpsCertificatePassword.trim()) {
        document.setIn(
          ['web', 'https', 'certificate', 'password'],
          form.httpsCertificatePassword.trim(),
        );
      }
      document.setIn(['web', 'rate_limiting', 'enabled'], form.rateLimitEnabled);
      document.setIn(['web', 'rate_limiting', 'api_permit_limit'], toNumber(form.rateLimitApi, 200));
      document.setIn(
        ['web', 'rate_limiting', 'api_window_seconds'],
        toNumber(form.rateLimitApiWindow, 60),
      );
      document.setIn(
        ['web', 'rate_limiting', 'federation_permit_limit'],
        toNumber(form.rateLimitFederation, 30),
      );
      document.setIn(
        ['web', 'rate_limiting', 'federation_window_seconds'],
        toNumber(form.rateLimitFederationWindow, 60),
      );
      document.setIn(
        ['web', 'rate_limiting', 'mesh_gateway_permit_limit'],
        toNumber(form.rateLimitMesh, 60),
      );
      document.setIn(
        ['web', 'rate_limiting', 'mesh_gateway_window_seconds'],
        toNumber(form.rateLimitMeshWindow, 60),
      );

      document.setIn(['filters', 'search', 'request'], parseLines(form.searchFilterRequest));
      document.setIn(
        ['filters', 'search_retention', 'max_age_days'],
        toNumber(form.searchRetentionMaxAgeDays, 30),
      );
      document.setIn(
        ['filters', 'search_retention', 'max_count'],
        toNumber(form.searchRetentionMaxCount, 1000),
      );
      document.setIn(
        ['filters', 'search_retention', 'cleanup_interval_seconds'],
        toNumber(form.searchRetentionCleanupInterval, 86400),
      );
      document.setIn(['blacklist', 'enabled'], form.blacklistEnabled);
      document.setIn(['blacklist', 'file'], form.blacklistFile.trim());
      document.setIn(['dht', 'enabled'], form.dhtEnabled);
      document.setIn(['dht', 'lan_only'], form.dhtLanOnly);
      document.setIn(['dht', 'overlay_port'], toNumber(form.dhtOverlayPort, 50305));
      document.setIn(['dht', 'dht_port'], toNumber(form.dhtPort, 50300));
      document.setIn(['dht', 'bootstrap_routers'], parseLines(form.dhtBootstrapRouters));
      document.setIn(
        ['dht', 'announce_interval_seconds'],
        toNumber(form.dhtAnnounceIntervalSeconds, 900),
      );
      document.setIn(['feature', 'scene_pod_bridge'], form.featureScenePodBridge);
      document.setIn(['rescue_mode', 'enabled'], form.rescueModeEnabled);
      document.setIn(
        ['rescue_mode', 'max_queue_time_seconds'],
        toNumber(form.rescueModeMaxQueueSeconds, 1800),
      );
      document.setIn(
        ['throttling', 'search', 'incoming', 'concurrency'],
        toNumber(form.searchIncomingConcurrency, 10),
      );
      document.setIn(
        ['throttling', 'search', 'incoming', 'circuit_breaker'],
        toNumber(form.searchIncomingCircuitBreaker, 500),
      );
      document.setIn(
        ['throttling', 'search', 'incoming', 'response_file_limit'],
        toNumber(form.searchIncomingResponseFileLimit, 500),
      );

      setOptionalNumber(document, ['retention', 'search'], form.retentionSearch);
      document.setIn(['retention', 'events'], toNumber(form.eventsRetention, 30));
      document.setIn(['retention', 'logs'], toNumber(form.logRetention, 180));
      setOptionalNumber(
        document,
        ['retention', 'transfers', 'upload', 'succeeded'],
        form.retentionUploadSucceeded,
      );
      setOptionalNumber(
        document,
        ['retention', 'transfers', 'upload', 'errored'],
        form.retentionUploadErrored,
      );
      setOptionalNumber(
        document,
        ['retention', 'transfers', 'upload', 'cancelled'],
        form.retentionUploadCancelled,
      );
      setOptionalNumber(
        document,
        ['retention', 'transfers', 'download', 'succeeded'],
        form.retentionDownloadSucceeded,
      );
      setOptionalNumber(
        document,
        ['retention', 'transfers', 'download', 'errored'],
        form.retentionDownloadErrored,
      );
      setOptionalNumber(
        document,
        ['retention', 'transfers', 'download', 'cancelled'],
        form.retentionDownloadCancelled,
      );
      setOptionalNumber(document, ['retention', 'files', 'complete'], form.fileCompleteRetention);
      setOptionalNumber(document, ['retention', 'files', 'incomplete'], form.fileIncompleteRetention);
      document.setIn(['shares', 'probe_media_attributes'], form.shareProbeMediaAttributes);
      document.setIn(['shares', 'cache', 'workers'], toNumber(form.shareCacheWorkers, 4));
      setOptionalNumber(document, ['shares', 'cache', 'retention'], form.shareCacheRetention);

      await optionsApi.updateYaml({ yaml: document.toString() });
      if (
        !mountedRef.current ||
        requestId !== saveRequestIdRef.current
      ) {
        return;
      }
      setForm((current) => ({
        ...current,
        apiKeyConfigured: current.apiKeyConfigured || Boolean(current.apiKeyValue.trim()),
        apiKeyValue: '',
        httpsCertificatePassword: '',
        httpsCertificatePasswordConfigured:
          current.httpsCertificatePasswordConfigured ||
          Boolean(current.httpsCertificatePassword.trim()),
        jwtConfigured: current.jwtConfigured || Boolean(current.jwtKey.trim()),
        jwtKey: '',
      }));
      setMessage({
        positive: true,
        text: 'Policy settings saved to YAML. Restart-signalled options still require a daemon restart.',
      });
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === saveRequestIdRef.current
      ) {
        setMessage({
          negative: true,
          text: toDisplayError(error, 'Failed to save policy settings.'),
        });
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === saveRequestIdRef.current
      ) {
        setSaving(false);
      }
    }
  };

  return (
    <AdminPoliciesForm
      form={form}
      message={message}
      missing={missing}
      remoteConfiguration={remoteConfiguration}
      reset={reset}
      saveYaml={saveYaml}
      saving={saving}
      update={update}
    />
  );
};

export default AdminPolicies;
