import React from 'react';
import {
  Button,
  Card,
  Checkbox,
  Form,
  Header,
  Icon,
  Label,
  Message,
  Popup,
  Segment,
  Table,
} from 'semantic-ui-react';

const boolLabel = (value, trueText = 'Enabled', falseText = 'Disabled') => (
  <Label color={value ? 'green' : 'grey'}>
    <Icon name={value ? 'check circle' : 'minus circle'} />
    {value ? trueText : falseText}
  </Label>
);

const AdminPoliciesForm = ({
  form,
  message,
  missing,
  remoteConfiguration,
  reset,
  saveYaml,
  saving,
  update,
}) => (
    <div className="admin-policies">
      <Segment>
        <Header as="h3">
          <Icon name="sliders horizontal" />
          Policies
        </Header>
        <p>
          Guided YAML controls for operator policies that are otherwise easy to
          miss in raw configuration.
        </p>
      </Segment>

      {!remoteConfiguration && (
        <Message
          info
          size="small"
        >
          Remote configuration is disabled. These controls show the settings
          shape, but YAML saving is disabled until remote configuration is
          enabled.
        </Message>
      )}

      {message && (
        <Message
          negative={message.negative}
          positive={message.positive}
          size="small"
        >
          {message.text}
        </Message>
      )}
      {missing.length > 0 && (
        <Message
          size="small"
          warning
        >
          <Message.List items={missing} />
        </Message>
      )}

      <Card.Group
        itemsPerRow={1}
        stackable
      >
        <Card fluid>
          <Card.Content>
            <Card.Header>
              <Icon name="bolt" />
              Actions: Webhooks and Scripts
            </Card.Header>
            <Card.Meta>Event hooks, remote targets, command hooks, and retry shape.</Card.Meta>
          </Card.Content>
          <Card.Content>
            <Form>
              <Form.Group grouped>
                <Popup
                  content="Probe shared audio files for bitrate, duration, and sample metadata during share scans. Disable on slow or remote storage."
                  trigger={
                    <Checkbox
                      aria-label="Probe share media attributes"
                      checked={form.shareProbeMediaAttributes}
                      disabled={!remoteConfiguration || saving}
                      label="Probe share media attributes"
                      onChange={(_, { checked }) =>
                        update('shareProbeMediaAttributes', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Webhook policy name"
                  disabled={!remoteConfiguration || saving}
                  label="Webhook Name"
                  onChange={(_, { value }) => update('webhookName', value)}
                  value={form.webhookName}
                />
                <Form.Input
                  aria-label="Webhook target URL"
                  disabled={!remoteConfiguration || saving}
                  label="Webhook URL"
                  onChange={(_, { value }) => update('webhookUrl', value)}
                  placeholder="https://example.invalid/slskr-hook"
                  value={form.webhookUrl}
                />
                <Form.Input
                  aria-label="Webhook retry attempts"
                  disabled={!remoteConfiguration || saving}
                  label="Retry Attempts"
                  min={1}
                  onChange={(_, { value }) => update('webhookRetryAttempts', value)}
                  type="number"
                  value={form.webhookRetryAttempts}
                />
                <Form.Input
                  aria-label="Webhook timeout milliseconds"
                  disabled={!remoteConfiguration || saving}
                  label="Timeout ms"
                  min={1000}
                  onChange={(_, { value }) => update('webhookTimeout', value)}
                  type="number"
                  value={form.webhookTimeout}
                />
              </Form.Group>
              <Form.TextArea
                aria-label="Webhook event names"
                disabled={!remoteConfiguration || saving}
                label="Webhook Events"
                onChange={(_, { value }) => update('webhookEvents', value)}
                placeholder="PrivateMessageReceived&#10;DownloadFileComplete"
                value={form.webhookEvents}
              />
              <Popup
                content="Allow this webhook to call a target with a self-signed or otherwise untrusted certificate."
                trigger={
                  <Checkbox
                    aria-label="Ignore webhook certificate errors"
                    checked={form.webhookIgnoreCertificateErrors}
                    disabled={!remoteConfiguration || saving}
                    label="Ignore webhook certificate errors"
                    onChange={(_, { checked }) =>
                      update('webhookIgnoreCertificateErrors', Boolean(checked))
                    }
                    toggle
                  />
                }
              />
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Script policy name"
                  disabled={!remoteConfiguration || saving}
                  label="Script Name"
                  onChange={(_, { value }) => update('scriptName', value)}
                  value={form.scriptName}
                />
                <Form.Input
                  aria-label="Script command"
                  disabled={!remoteConfiguration || saving}
                  label="Script Command"
                  onChange={(_, { value }) => update('scriptCommand', value)}
                  placeholder="/usr/local/bin/slskr-hook"
                  value={form.scriptCommand}
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Script executable"
                  disabled={!remoteConfiguration || saving}
                  label="Script Executable"
                  onChange={(_, { value }) => update('scriptExecutable', value)}
                  placeholder="/bin/sh"
                  value={form.scriptExecutable}
                />
                <Form.Input
                  aria-label="Script arguments"
                  disabled={!remoteConfiguration || saving}
                  label="Script Args"
                  onChange={(_, { value }) => update('scriptArgs', value)}
                  placeholder="-c ./hook.sh"
                  value={form.scriptArgs}
                />
              </Form.Group>
              <Form.TextArea
                aria-label="Script event names"
                disabled={!remoteConfiguration || saving}
                label="Script Events"
                onChange={(_, { value }) => update('scriptEvents', value)}
                placeholder="DownloadFileComplete"
                value={form.scriptEvents}
              />
            </Form>
          </Card.Content>
        </Card>

        <Card fluid>
          <Card.Content>
            <Card.Header>
              <Icon name="exchange" />
              Transfer Policy
            </Card.Header>
            <Card.Meta>Slots, speed ceilings, auto-replace, and schedule enablement.</Card.Meta>
          </Card.Content>
          <Card.Content>
            <div className="integration-status-row">
              {boolLabel(form.scheduledLimitsEnabled, 'Schedules On', 'Schedules Off')}
              {boolLabel(form.autoReplaceStuck, 'Auto-Replace On', 'Auto-Replace Off')}
            </div>
            <Form>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Global upload slots"
                  disabled={!remoteConfiguration || saving}
                  label="Upload Slots"
                  onChange={(_, { value }) => update('uploadSlots', value)}
                  type="number"
                  value={form.uploadSlots}
                />
                <Form.Input
                  aria-label="Global upload speed limit"
                  disabled={!remoteConfiguration || saving}
                  label="Upload KiB/s"
                  onChange={(_, { value }) => update('uploadSpeedLimit', value)}
                  type="number"
                  value={form.uploadSpeedLimit}
                />
                <Form.Input
                  aria-label="Global download slots"
                  disabled={!remoteConfiguration || saving}
                  label="Download Slots"
                  onChange={(_, { value }) => update('downloadSlots', value)}
                  type="number"
                  value={form.downloadSlots}
                />
                <Form.Input
                  aria-label="Global download speed limit"
                  disabled={!remoteConfiguration || saving}
                  label="Download KiB/s"
                  onChange={(_, { value }) => update('downloadSpeedLimit', value)}
                  type="number"
                  value={form.downloadSpeedLimit}
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Select
                  aria-label="Download retry incomplete strategy"
                  disabled={!remoteConfiguration || saving}
                  label="Retry Partial Files"
                  onChange={(_, { value }) => update('downloadRetryIncomplete', value)}
                  options={[
                    { key: 'resume', text: 'resume', value: 'resume' },
                    { key: 'overwrite', text: 'overwrite', value: 'overwrite' },
                  ]}
                  value={form.downloadRetryIncomplete}
                />
                <Form.Input
                  aria-label="Download retry attempts"
                  disabled={!remoteConfiguration || saving}
                  label="Retry Attempts"
                  min={1}
                  onChange={(_, { value }) => update('downloadRetryAttempts', value)}
                  type="number"
                  value={form.downloadRetryAttempts}
                />
                <Form.Input
                  aria-label="Download retry delay milliseconds"
                  disabled={!remoteConfiguration || saving}
                  label="Retry Delay ms"
                  min={0}
                  onChange={(_, { value }) => update('downloadRetryDelay', value)}
                  type="number"
                  value={form.downloadRetryDelay}
                />
                <Form.Input
                  aria-label="Download retry max delay milliseconds"
                  disabled={!remoteConfiguration || saving}
                  label="Retry Max Delay ms"
                  min={1000}
                  onChange={(_, { value }) => update('downloadRetryMaxDelay', value)}
                  type="number"
                  value={form.downloadRetryMaxDelay}
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Auto replace interval seconds"
                  disabled={!remoteConfiguration || saving}
                  label="Auto-Replace Interval"
                  min={60}
                  onChange={(_, { value }) => update('autoReplaceInterval', value)}
                  type="number"
                  value={form.autoReplaceInterval}
                />
                <Form.Input
                  aria-label="Auto replace size threshold"
                  disabled={!remoteConfiguration || saving}
                  label="Size Threshold %"
                  min={0}
                  onChange={(_, { value }) => update('autoReplaceThreshold', value)}
                  type="number"
                  value={form.autoReplaceThreshold}
                />
                <Form.Input
                  aria-label="Auto replace max retries"
                  disabled={!remoteConfiguration || saving}
                  label="Max Retries"
                  min={0}
                  onChange={(_, { value }) => update('autoReplaceMaxRetries', value)}
                  type="number"
                  value={form.autoReplaceMaxRetries}
                />
              </Form.Group>
              <Form.Group grouped>
                <Popup
                  content="Enable automatic replacement of stuck downloads with alternative sources."
                  trigger={
                    <Checkbox
                      aria-label="Enable auto replace stuck downloads"
                      checked={form.autoReplaceStuck}
                      disabled={!remoteConfiguration || saving}
                      label="Enable auto-replace stuck downloads"
                      onChange={(_, { checked }) =>
                        update('autoReplaceStuck', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Enable the legacy top-level scheduled speed-limit policy in YAML."
                  trigger={
                    <Checkbox
                      aria-label="Enable scheduled speed limits"
                      checked={form.scheduledLimitsEnabled}
                      disabled={!remoteConfiguration || saving}
                      label="Enable scheduled speed limits"
                      onChange={(_, { checked }) =>
                        update('scheduledLimitsEnabled', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Enable scheduled upload speed limits under global upload policy."
                  trigger={
                    <Checkbox
                      aria-label="Enable upload scheduled speed limits"
                      checked={form.uploadScheduledLimitsEnabled}
                      disabled={!remoteConfiguration || saving}
                      label="Enable upload schedules"
                      onChange={(_, { checked }) =>
                        update('uploadScheduledLimitsEnabled', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Enable scheduled download speed limits under global download policy."
                  trigger={
                    <Checkbox
                      aria-label="Enable download scheduled speed limits"
                      checked={form.downloadScheduledLimitsEnabled}
                      disabled={!remoteConfiguration || saving}
                      label="Enable download schedules"
                      onChange={(_, { checked }) =>
                        update('downloadScheduledLimitsEnabled', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
              </Form.Group>
            </Form>
          </Card.Content>
        </Card>

        <Card fluid>
          <Card.Content>
            <Card.Header>
              <Icon name="lock" />
              Security and Access
            </Card.Header>
            <Card.Meta>Authentication, API keys, HTTPS, CORS-adjacent rate limits, and restart cues.</Card.Meta>
          </Card.Content>
          <Card.Content>
            <div className="integration-status-row">
              {boolLabel(!form.noAuth, 'Auth On', 'No Auth')}
              {boolLabel(form.jwtConfigured, 'JWT Key Set', 'JWT Key Missing')}
              {boolLabel(form.apiKeyConfigured, 'API Key Set', 'API Key Missing')}
              {boolLabel(!form.httpsDisabled, 'HTTPS On', 'HTTPS Off')}
              {boolLabel(
                form.httpsCertificatePasswordConfigured,
                'Certificate Password Set',
                'Certificate Password Missing',
              )}
            </div>
            <Form>
              <Form.Group grouped>
                <Popup
                  content="Enable strict startup and request hardening checks for exposed deployments."
                  trigger={
                    <Checkbox
                      aria-label="Enforce web security hardening"
                      checked={form.enforceSecurity}
                      disabled={!remoteConfiguration || saving}
                      label="Enforce security hardening"
                      onChange={(_, { checked }) =>
                        update('enforceSecurity', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Disable Web UI authentication only for tightly controlled loopback deployments."
                  trigger={
                    <Checkbox
                      aria-label="Disable web authentication"
                      checked={form.noAuth}
                      disabled={!remoteConfiguration || saving}
                      label="Disable authentication"
                      onChange={(_, { checked }) => update('noAuth', Boolean(checked))}
                      toggle
                    />
                  }
                />
                <Popup
                  content="Allow no-auth access from explicitly configured non-loopback CIDRs."
                  trigger={
                    <Checkbox
                      aria-label="Allow remote no-auth CIDRs"
                      checked={form.allowRemoteNoAuth}
                      disabled={!remoteConfiguration || saving}
                      label="Allow remote no-auth CIDRs"
                      onChange={(_, { checked }) =>
                        update('allowRemoteNoAuth', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Enable HTTP rate limiting for API, federation, and mesh gateway policies."
                  trigger={
                    <Checkbox
                      aria-label="Enable HTTP rate limiting"
                      checked={form.rateLimitEnabled}
                      disabled={!remoteConfiguration || saving}
                      label="Enable HTTP rate limiting"
                      onChange={(_, { checked }) =>
                        update('rateLimitEnabled', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="JWT replacement key"
                  disabled={!remoteConfiguration || saving}
                  label="JWT Key"
                  onChange={(_, { value }) => update('jwtKey', value)}
                  placeholder={form.jwtConfigured ? 'Configured' : 'New JWT key'}
                  type="password"
                  value={form.jwtKey}
                />
                <Form.Input
                  aria-label="JWT TTL milliseconds"
                  disabled={!remoteConfiguration || saving}
                  label="JWT TTL ms"
                  min={3600}
                  onChange={(_, { value }) => update('jwtTtl', value)}
                  type="number"
                  value={form.jwtTtl}
                />
                <Form.Input
                  aria-label="No auth allowed CIDRs"
                  disabled={!remoteConfiguration || saving}
                  label="No-Auth CIDRs"
                  onChange={(_, { value }) => update('passthroughCidrs', value)}
                  value={form.passthroughCidrs}
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="API key policy name"
                  disabled={!remoteConfiguration || saving}
                  label="API Key Name"
                  onChange={(_, { value }) => update('apiKeyName', value)}
                  value={form.apiKeyName}
                />
                <Form.Input
                  aria-label="API key replacement value"
                  disabled={!remoteConfiguration || saving}
                  label="API Key"
                  onChange={(_, { value }) => update('apiKeyValue', value)}
                  placeholder={form.apiKeyConfigured ? 'Configured' : 'New API key'}
                  type="password"
                  value={form.apiKeyValue}
                />
                <Form.Select
                  aria-label="API key role"
                  disabled={!remoteConfiguration || saving}
                  label="Role"
                  onChange={(_, { value }) => update('apiKeyRole', value)}
                  options={[
                    { key: 'ReadOnly', text: 'ReadOnly', value: 'ReadOnly' },
                    { key: 'ReadWrite', text: 'ReadWrite', value: 'ReadWrite' },
                    {
                      key: 'Administrator',
                      text: 'Administrator',
                      value: 'Administrator',
                    },
                  ]}
                  value={form.apiKeyRole}
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="API key CIDR allowlist"
                  disabled={!remoteConfiguration || saving}
                  label="API CIDRs"
                  onChange={(_, { value }) => update('apiKeyCidr', value)}
                  value={form.apiKeyCidr}
                />
                <Form.Input
                  aria-label="API key scopes"
                  disabled={!remoteConfiguration || saving}
                  label="API Scopes"
                  onChange={(_, { value }) => update('apiKeyScopes', value)}
                  value={form.apiKeyScopes}
                />
              </Form.Group>
              <Form.Group grouped>
                <Popup
                  content="Disable HTTPS listener. Keep this off when exposing the Web UI beyond localhost."
                  trigger={
                    <Checkbox
                      aria-label="Disable HTTPS listener"
                      checked={form.httpsDisabled}
                      disabled={!remoteConfiguration || saving}
                      label="Disable HTTPS"
                      onChange={(_, { checked }) =>
                        update('httpsDisabled', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Redirect HTTP requests to HTTPS after certificate settings are valid."
                  trigger={
                    <Checkbox
                      aria-label="Force HTTPS redirects"
                      checked={form.forceHttps}
                      disabled={!remoteConfiguration || saving}
                      label="Force HTTPS"
                      onChange={(_, { checked }) =>
                        update('forceHttps', Boolean(checked))
                      }
                    />
                  }
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="HTTPS port"
                  disabled={!remoteConfiguration || saving}
                  label="HTTPS Port"
                  onChange={(_, { value }) => update('httpsPort', value)}
                  type="number"
                  value={form.httpsPort}
                />
                <Form.Input
                  aria-label="HTTPS certificate PFX path"
                  disabled={!remoteConfiguration || saving}
                  label="Certificate PFX (optional)"
                  onChange={(_, { value }) => update('httpsCertificatePfx', value)}
                  value={form.httpsCertificatePfx}
                />
                <Form.Input
                  aria-label="HTTPS certificate password"
                  disabled={!remoteConfiguration || saving}
                  label="Certificate Password"
                  onChange={(_, { value }) =>
                    update('httpsCertificatePassword', value)
                  }
                  placeholder={
                    form.httpsCertificatePasswordConfigured
                      ? 'Configured'
                      : 'Certificate password'
                  }
                  type="password"
                  value={form.httpsCertificatePassword}
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="API rate limit permits"
                  disabled={!remoteConfiguration || saving}
                  label="API Permits/min"
                  onChange={(_, { value }) => update('rateLimitApi', value)}
                  type="number"
                  value={form.rateLimitApi}
                />
                <Form.Input
                  aria-label="API rate limit window seconds"
                  disabled={!remoteConfiguration || saving}
                  label="API Window sec"
                  onChange={(_, { value }) => update('rateLimitApiWindow', value)}
                  type="number"
                  value={form.rateLimitApiWindow}
                />
                <Form.Input
                  aria-label="Federation rate limit permits"
                  disabled={!remoteConfiguration || saving}
                  label="Federation Permits/min"
                  onChange={(_, { value }) => update('rateLimitFederation', value)}
                  type="number"
                  value={form.rateLimitFederation}
                />
                <Form.Input
                  aria-label="Federation rate limit window seconds"
                  disabled={!remoteConfiguration || saving}
                  label="Federation Window sec"
                  onChange={(_, { value }) =>
                    update('rateLimitFederationWindow', value)
                  }
                  type="number"
                  value={form.rateLimitFederationWindow}
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Mesh gateway rate limit permits"
                  disabled={!remoteConfiguration || saving}
                  label="Mesh Permits/min"
                  onChange={(_, { value }) => update('rateLimitMesh', value)}
                  type="number"
                  value={form.rateLimitMesh}
                />
                <Form.Input
                  aria-label="Mesh gateway rate limit window seconds"
                  disabled={!remoteConfiguration || saving}
                  label="Mesh Window sec"
                  onChange={(_, { value }) => update('rateLimitMeshWindow', value)}
                  type="number"
                  value={form.rateLimitMeshWindow}
                />
              </Form.Group>
            </Form>
          </Card.Content>
        </Card>

        <Card fluid>
          <Card.Content>
            <Card.Header>
              <Icon name="sitemap" />
              Search and Network Policy
            </Card.Header>
            <Card.Meta>Incoming search throttles, request filters, and managed blacklist setup.</Card.Meta>
          </Card.Content>
          <Card.Content>
            <div className="integration-status-row">
              {boolLabel(form.blacklistEnabled, 'Blacklist On', 'Blacklist Off')}
              {boolLabel(form.dhtEnabled, 'DHT On', 'DHT Off')}
              {boolLabel(form.dhtLanOnly, 'LAN Only', 'Public DHT')}
              {boolLabel(form.featureScenePodBridge, 'Bridge On', 'Bridge Off')}
              {boolLabel(form.rescueModeEnabled, 'Rescue On', 'Rescue Off')}
            </div>
            <Form>
              <Form.TextArea
                aria-label="Incoming search request filters"
                disabled={!remoteConfiguration || saving}
                label="Incoming Search Filter Regexes"
                onChange={(_, { value }) => update('searchFilterRequest', value)}
                placeholder="(?i)forbidden term"
                value={form.searchFilterRequest}
              />
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Incoming search concurrency"
                  disabled={!remoteConfiguration || saving}
                  label="Incoming Concurrency"
                  onChange={(_, { value }) =>
                    update('searchIncomingConcurrency', value)
                  }
                  type="number"
                  value={form.searchIncomingConcurrency}
                />
                <Form.Input
                  aria-label="Incoming search circuit breaker"
                  disabled={!remoteConfiguration || saving}
                  label="Circuit Breaker"
                  onChange={(_, { value }) =>
                    update('searchIncomingCircuitBreaker', value)
                  }
                  type="number"
                  value={form.searchIncomingCircuitBreaker}
                />
                <Form.Input
                  aria-label="Incoming search response file limit"
                  disabled={!remoteConfiguration || saving}
                  label="Response File Limit"
                  onChange={(_, { value }) =>
                    update('searchIncomingResponseFileLimit', value)
                  }
                  type="number"
                  value={form.searchIncomingResponseFileLimit}
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Popup
                  content="Enable loading a managed CIDR/P2P/DAT blacklist file at startup."
                  trigger={
                    <Checkbox
                      aria-label="Enable managed blacklist"
                      checked={form.blacklistEnabled}
                      disabled={!remoteConfiguration || saving}
                      label="Enable managed blacklist"
                      onChange={(_, { checked }) =>
                        update('blacklistEnabled', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Form.Input
                  aria-label="Managed blacklist file path"
                  disabled={!remoteConfiguration || saving}
                  label="Blacklist File"
                  onChange={(_, { value }) => update('blacklistFile', value)}
                  value={form.blacklistFile}
                />
              </Form.Group>
              <Form.Group grouped>
                <Popup
                  content="Enable DHT rendezvous. Disable this when you want no DHT peer discovery."
                  trigger={
                    <Checkbox
                      aria-label="Enable DHT rendezvous"
                      checked={form.dhtEnabled}
                      disabled={!remoteConfiguration || saving}
                      label="Enable DHT rendezvous"
                      onChange={(_, { checked }) =>
                        update('dhtEnabled', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Keep DHT discovery away from public bootstrap routers and use only private or local discovery paths."
                  trigger={
                    <Checkbox
                      aria-label="Use LAN-only DHT rendezvous"
                      checked={form.dhtLanOnly}
                      disabled={!remoteConfiguration || saving}
                      label="LAN-only DHT"
                      onChange={(_, { checked }) =>
                        update('dhtLanOnly', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Opt in to experimental Scene and Pod bridge aggregation."
                  trigger={
                    <Checkbox
                      aria-label="Enable Scene Pod Bridge"
                      checked={form.featureScenePodBridge}
                      disabled={!remoteConfiguration || saving}
                      label="Enable Scene Pod Bridge"
                      onChange={(_, { checked }) =>
                        update('featureScenePodBridge', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
                <Popup
                  content="Enable underperformance rescue policies for queued or stalled downloads."
                  trigger={
                    <Checkbox
                      aria-label="Enable rescue mode"
                      checked={form.rescueModeEnabled}
                      disabled={!remoteConfiguration || saving}
                      label="Enable rescue mode"
                      onChange={(_, { checked }) =>
                        update('rescueModeEnabled', Boolean(checked))
                      }
                      toggle
                    />
                  }
                />
              </Form.Group>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="DHT overlay TCP port"
                  disabled={!remoteConfiguration || saving}
                  label="Overlay TCP Port"
                  onChange={(_, { value }) => update('dhtOverlayPort', value)}
                  type="number"
                  value={form.dhtOverlayPort}
                />
                <Form.Input
                  aria-label="DHT UDP port"
                  disabled={!remoteConfiguration || saving}
                  label="DHT UDP Port"
                  onChange={(_, { value }) => update('dhtPort', value)}
                  type="number"
                  value={form.dhtPort}
                />
                <Form.Input
                  aria-label="DHT announce interval seconds"
                  disabled={!remoteConfiguration || saving}
                  label="DHT Announce sec"
                  onChange={(_, { value }) =>
                    update('dhtAnnounceIntervalSeconds', value)
                  }
                  type="number"
                  value={form.dhtAnnounceIntervalSeconds}
                />
                <Form.Input
                  aria-label="Rescue max queue seconds"
                  disabled={!remoteConfiguration || saving}
                  label="Rescue Queue sec"
                  onChange={(_, { value }) =>
                    update('rescueModeMaxQueueSeconds', value)
                  }
                  type="number"
                  value={form.rescueModeMaxQueueSeconds}
                />
              </Form.Group>
              <Form.TextArea
                aria-label="DHT bootstrap routers"
                disabled={!remoteConfiguration || saving}
                label="DHT Bootstrap Routers"
                onChange={(_, { value }) => update('dhtBootstrapRouters', value)}
                placeholder="router.bittorrent.com"
                value={form.dhtBootstrapRouters}
              />
            </Form>
          </Card.Content>
        </Card>

        <Card fluid>
          <Card.Content>
            <Card.Header>
              <Icon name="archive" />
              Retention and Storage
            </Card.Header>
            <Card.Meta>Search/event/log cleanup, transfer history, file records, and share cache pressure.</Card.Meta>
          </Card.Content>
          <Card.Content>
            <Form>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Search retention minutes"
                  disabled={!remoteConfiguration || saving}
                  label="Search Retention min"
                  onChange={(_, { value }) => update('retentionSearch', value)}
                  value={form.retentionSearch}
                />
                <Form.Input
                  aria-label="Event retention days"
                  disabled={!remoteConfiguration || saving}
                  label="Events Retention days"
                  onChange={(_, { value }) => update('eventsRetention', value)}
                  type="number"
                  value={form.eventsRetention}
                />
                <Form.Input
                  aria-label="Log retention days"
                  disabled={!remoteConfiguration || saving}
                  label="Logs Retention days"
                  onChange={(_, { value }) => update('logRetention', value)}
                  type="number"
                  value={form.logRetention}
                />
              </Form.Group>
              <Table
                celled
                compact
              >
                <Table.Header>
                  <Table.Row>
                    <Table.HeaderCell>History</Table.HeaderCell>
                    <Table.HeaderCell>Succeeded</Table.HeaderCell>
                    <Table.HeaderCell>Errored</Table.HeaderCell>
                    <Table.HeaderCell>Cancelled</Table.HeaderCell>
                  </Table.Row>
                </Table.Header>
                <Table.Body>
                  <Table.Row>
                    <Table.Cell>Uploads</Table.Cell>
                    <Table.Cell>
                      <Form.Input
                        aria-label="Upload succeeded retention minutes"
                        disabled={!remoteConfiguration || saving}
                        onChange={(_, { value }) =>
                          update('retentionUploadSucceeded', value)
                        }
                        value={form.retentionUploadSucceeded}
                      />
                    </Table.Cell>
                    <Table.Cell>
                      <Form.Input
                        aria-label="Upload errored retention minutes"
                        disabled={!remoteConfiguration || saving}
                        onChange={(_, { value }) =>
                          update('retentionUploadErrored', value)
                        }
                        value={form.retentionUploadErrored}
                      />
                    </Table.Cell>
                    <Table.Cell>
                      <Form.Input
                        aria-label="Upload cancelled retention minutes"
                        disabled={!remoteConfiguration || saving}
                        onChange={(_, { value }) =>
                          update('retentionUploadCancelled', value)
                        }
                        value={form.retentionUploadCancelled}
                      />
                    </Table.Cell>
                  </Table.Row>
                  <Table.Row>
                    <Table.Cell>Downloads</Table.Cell>
                    <Table.Cell>
                      <Form.Input
                        aria-label="Download succeeded retention minutes"
                        disabled={!remoteConfiguration || saving}
                        onChange={(_, { value }) =>
                          update('retentionDownloadSucceeded', value)
                        }
                        value={form.retentionDownloadSucceeded}
                      />
                    </Table.Cell>
                    <Table.Cell>
                      <Form.Input
                        aria-label="Download errored retention minutes"
                        disabled={!remoteConfiguration || saving}
                        onChange={(_, { value }) =>
                          update('retentionDownloadErrored', value)
                        }
                        value={form.retentionDownloadErrored}
                      />
                    </Table.Cell>
                    <Table.Cell>
                      <Form.Input
                        aria-label="Download cancelled retention minutes"
                        disabled={!remoteConfiguration || saving}
                        onChange={(_, { value }) =>
                          update('retentionDownloadCancelled', value)
                        }
                        value={form.retentionDownloadCancelled}
                      />
                    </Table.Cell>
                  </Table.Row>
                </Table.Body>
              </Table>
              <Form.Group widths="equal">
                <Form.Input
                  aria-label="Complete file retention minutes"
                  disabled={!remoteConfiguration || saving}
                  label="Complete Files min"
                  onChange={(_, { value }) => update('fileCompleteRetention', value)}
                  value={form.fileCompleteRetention}
                />
                <Form.Input
                  aria-label="Incomplete file retention minutes"
                  disabled={!remoteConfiguration || saving}
                  label="Incomplete Files min"
                  onChange={(_, { value }) =>
                    update('fileIncompleteRetention', value)
                  }
                  value={form.fileIncompleteRetention}
                />
                <Form.Input
                  aria-label="Share cache workers"
                  disabled={!remoteConfiguration || saving}
                  label="Share Cache Workers"
                  onChange={(_, { value }) => update('shareCacheWorkers', value)}
                  type="number"
                  value={form.shareCacheWorkers}
                />
                <Form.Input
                  aria-label="Share cache retention minutes"
                  disabled={!remoteConfiguration || saving}
                  label="Share Cache Retention min"
                  onChange={(_, { value }) => update('shareCacheRetention', value)}
                  value={form.shareCacheRetention}
                />
              </Form.Group>
            </Form>
          </Card.Content>
        </Card>
      </Card.Group>

      <div className="integration-actions admin-policy-actions">
        <Popup
          content="Persist the policy settings above to YAML. This does not test webhooks, run scripts, contact peers, restart the daemon, or mutate downloads."
          trigger={
            <Button
              disabled={!remoteConfiguration || missing.length > 0}
              icon
              labelPosition="left"
              loading={saving}
              onClick={saveYaml}
              primary
            >
              <Icon name="file alternate" />
              Save YAML
            </Button>
          }
        />
        <Popup
          content="Discard unsaved policy edits and restore values currently reported by the daemon."
          trigger={
            <Button
              disabled={saving}
              icon
              labelPosition="left"
              onClick={reset}
            >
              <Icon name="undo" />
              Reset
            </Button>
          }
        />
      </div>
    </div>
);

export default AdminPoliciesForm;
