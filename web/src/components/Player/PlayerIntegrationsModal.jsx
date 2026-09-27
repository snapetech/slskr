import {
  Button,
  Icon,
  Input,
  Message,
  Modal,
  Popup,
} from 'semantic-ui-react';

const getExternalVisualizerStatusText = (status, loading) => {
  if (loading) return 'Checking external visualizer launcher...';
  if (!status) return 'Status unavailable.';
  if (!status.enabled) return 'Disabled in slskr.yml.';
  if (!status.configured) return 'No launcher path configured.';
  if (!status.available) return 'Configured launcher path was not found.';
  return 'Ready to launch on the slskr host.';
};

const PlayerIntegrationsModal = ({
  externalVisualizerLaunching,
  externalVisualizerLoading,
  externalVisualizerMessage,
  externalVisualizerStatus,
  listenBrainzToken,
  onClearListenBrainzToken,
  onClose,
  onLaunchExternalVisualizer,
  onListenBrainzTokenChange,
  onRefreshExternalVisualizerStatus,
  open,
}) => (
      <Modal
        className="player-browser-modal player-integrations-modal"
        onClose={onClose}
        open={open}
        size="tiny"
      >
        <Modal.Header>Player Integrations</Modal.Header>
        <Modal.Content>
          <p className="player-modal-copy">
            ListenBrainz submissions are opt-in and stored only in this browser.
          </p>
          <Input
            aria-label="ListenBrainz user token"
            action={
              <Button
                aria-label="Clear ListenBrainz token"
                data-testid="player-clear-listenbrainz-token"
                icon
                onClick={onClearListenBrainzToken}
                type="button"
              >
                <Icon name="trash alternate outline" />
              </Button>
            }
            data-testid="player-listenbrainz-token"
            fluid
            icon="cloud upload"
            onChange={onListenBrainzTokenChange}
            placeholder="ListenBrainz token"
            size="mini"
            type="password"
            value={listenBrainzToken}
          />
          <div
            className="player-token-save-state"
            data-testid="player-listenbrainz-save-state"
          >
            <Icon name="check circle outline" />
            Token changes are saved automatically in this browser.
          </div>
          <div
            className="player-external-visualizer"
            data-testid="player-external-visualizer"
          >
            <div className="player-panel-title">External Visualizer</div>
            <div className="player-external-visualizer-summary">
              <Icon
                name={externalVisualizerStatus?.enabled ? 'desktop' : 'ban'}
              />
              <div>
                <div className="player-external-visualizer-name">
                  {externalVisualizerStatus?.name || 'RustyMilk'}
                </div>
                <div className="player-external-visualizer-status">
                  {getExternalVisualizerStatusText(
                    externalVisualizerStatus,
                    externalVisualizerLoading,
                  )}
                </div>
                {externalVisualizerStatus?.path ? (
                  <div
                    className="player-external-visualizer-path"
                    title={externalVisualizerStatus.path}
                  >
                    {externalVisualizerStatus.path}
                  </div>
                ) : null}
              </div>
            </div>
            {externalVisualizerMessage ? (
              <Message
                className="player-external-visualizer-message"
                compact
                data-testid="player-external-visualizer-message"
                info={externalVisualizerStatus?.available}
                size="tiny"
                warning={!externalVisualizerStatus?.available}
              >
                {externalVisualizerMessage}
              </Message>
            ) : null}
            <div className="player-external-visualizer-actions">
              <Popup
                content="Start the configured external visualizer on the slskr host. Use this for RustyMilk or another local visualizer that captures system audio."
                trigger={
                  <Button
                    data-testid="player-launch-external-visualizer"
                    disabled={
                      externalVisualizerLaunching ||
                      !externalVisualizerStatus?.enabled ||
                      !externalVisualizerStatus?.available
                    }
                    loading={externalVisualizerLaunching}
                    onClick={onLaunchExternalVisualizer}
                    size="mini"
                    type="button"
                  >
                    <Icon name="external alternate" />
                    Launch
                  </Button>
                }
              />
              <Popup
                content="Refresh the configured external visualizer path and readiness from the server."
                trigger={
                  <Button
                    data-testid="player-refresh-external-visualizer"
                    disabled={externalVisualizerLoading}
                    loading={externalVisualizerLoading}
                    onClick={onRefreshExternalVisualizerStatus}
                    size="mini"
                    type="button"
                  >
                    <Icon name="refresh" />
                    Refresh
                  </Button>
                }
              />
            </div>
          </div>
        </Modal.Content>
        <Modal.Actions>
          <Popup
            content="Close settings. ListenBrainz token changes have already been saved."
            trigger={
              <Button
                data-testid="player-close-integrations"
                onClick={onClose}
                primary
              >
                <Icon name="check" />
                Done
              </Button>
            }
          />
        </Modal.Actions>
      </Modal>
);

export default PlayerIntegrationsModal;
