import {
  clearDiscoveryShelf,
  exportDiscoveryShelfPolicyReport,
  getDiscoveryShelf,
  getDiscoveryShelfActionLabel,
  getDiscoveryShelfPolicyPreview,
  getDiscoveryShelfSummary,
  removeDiscoveryShelfItem,
} from '../../lib/discoveryShelf';
import { copyToClipboard } from '../../lib/clipboard';
import { toDisplayError } from '../../lib/errors';
import { useMountedRef } from '../../lib/useMountedRef';
import { useEffect, useState } from 'react';
import {
  Button,
  Checkbox,
  Icon,
  Input,
  Label,
  Message,
  Modal,
  Popup,
} from 'semantic-ui-react';

const PlayerDiscoveryShelfModal = ({ onClose, open }) => {
  const [expiryDays, setExpiryDays] = useState(14);
  const [items, setItems] = useState(() => getDiscoveryShelf());
  const [message, setMessage] = useState('');
  const [requireConsensus, setRequireConsensus] = useState(true);
  const mountedRef = useMountedRef();
  const summary = getDiscoveryShelfSummary();
  const policyPreview = getDiscoveryShelfPolicyPreview({
    expiryDays,
    items,
    requireConsensus,
  });

  const refreshShelf = () => {
    setItems(getDiscoveryShelf());
  };

  useEffect(() => {
    if (open) {
      refreshShelf();
      setMessage('');
    }
  }, [open]);

  const previewAction = (item) => {
    setMessage(
      `${getDiscoveryShelfActionLabel(item.action)} prepared for ${item.title}. No files were moved or deleted.`,
    );
  };

  const removeItem = (key) => {
    removeDiscoveryShelfItem(key);
    refreshShelf();
  };

  const clearShelf = () => {
    clearDiscoveryShelf();
    refreshShelf();
    setMessage('Discovery shelf cleared from this browser.');
  };

  const copyPolicyReport = async () => {
    const report = exportDiscoveryShelfPolicyReport({
      expiryDays,
      items,
      requireConsensus,
    });

    try {
      const copied = await copyToClipboard(report);
      if (!mountedRef.current) return;
      setMessage(
        copied
          ? `Policy report copied for ${items.length} shelf items.`
          : `Policy report prepared for ${items.length} shelf items; clipboard is unavailable.`,
      );
    } catch (error) {
      if (mountedRef.current) {
        setMessage(toDisplayError(error, 'Unable to copy policy report.'));
      }
    }
  };

  return (
    <Modal
      className="player-browser-modal player-discovery-shelf-modal"
      onClose={onClose}
      open={open}
      size="small"
    >
      <Modal.Header>Discovery Shelf</Modal.Header>
      <Modal.Content>
        <div className="player-shelf-summary" data-testid="player-shelf-summary">
          <div>
            <strong>{summary.total}</strong>
            <span>local review items</span>
          </div>
          <div>
            <strong>{summary['promote-preview']}</strong>
            <span>promote previews</span>
          </div>
          <div>
            <strong>{summary['archive-preview']}</strong>
            <span>archive previews</span>
          </div>
          <div>
            <strong>{summary['expiry-watch']}</strong>
            <span>expiry watch</span>
          </div>
        </div>
        {message ? (
          <Message compact size="mini">
            {message}
          </Message>
        ) : null}
        <section className="player-shelf-policy">
          <div className="player-panel-title">Policy Preview</div>
          <div className="player-shelf-policy-controls">
            <label htmlFor="player-shelf-expiry-days">Expire unrated after</label>
            <Input
              aria-label="Discovery shelf expiry days"
              data-testid="player-shelf-expiry-days"
              id="player-shelf-expiry-days"
              min="1"
              onChange={(event) => setExpiryDays(event.target.value)}
              size="mini"
              type="number"
              value={expiryDays}
            />
            <Popup
              content="Require shared-library consensus before any destructive archive or expiry action can be applied later."
              trigger={
                <Checkbox
                  checked={requireConsensus}
                  data-testid="player-shelf-require-consensus"
                  label="Consensus for destructive actions"
                  onChange={(_, data) => setRequireConsensus(Boolean(data.checked))}
                  toggle
                />
              }
            />
          </div>
          <div className="player-shelf-policy-preview" data-testid="player-shelf-policy-preview">
            <span>{policyPreview.promote} promote</span>
            <span>{policyPreview.archive} archive</span>
            <span>{policyPreview.expire} expire</span>
            <span>{policyPreview.review} review</span>
            <span>{policyPreview.blockedByConsensus} consensus gated</span>
          </div>
          <Popup
            content="Copy a text report of the current shelf policy preview for review. This does not apply any action."
            trigger={
              <Button
                data-testid="player-shelf-copy-policy-report"
                disabled={items.length === 0}
                onClick={copyPolicyReport}
                size="mini"
                type="button"
              >
                <Icon name="copy" />
                Copy Report
              </Button>
            }
          />
        </section>
        <div className="player-shelf-list">
          {items.length > 0 ? items.map((item) => (
            <div
              className="player-shelf-row"
              data-testid={`player-shelf-row-${item.key}`}
              key={item.key}
            >
              <div className="player-shelf-rating">{item.rating || '-'}</div>
              <div className="player-shelf-track">
                <strong>{item.title}</strong>
                <span>
                  {[item.artist, item.album].filter(Boolean).join(' - ') || 'Local discovery item'}
                </span>
              </div>
              <Label size="mini">
                {getDiscoveryShelfActionLabel(item.action)}
              </Label>
              <Popup
                content="Preview the shelf action. This does not move, delete, share, download, or publish anything."
                trigger={
                  <Button
                    data-testid={`player-shelf-preview-${item.key}`}
                    icon
                    onClick={() => previewAction(item)}
                    size="mini"
                    type="button"
                  >
                    <Icon name="eye" />
                  </Button>
                }
              />
              <Popup
                content="Remove this local review item from the browser-only shelf."
                trigger={
                  <Button
                    data-testid={`player-shelf-remove-${item.key}`}
                    icon
                    onClick={() => removeItem(item.key)}
                    size="mini"
                    type="button"
                  >
                    <Icon name="trash alternate outline" />
                  </Button>
                }
              />
            </div>
          )) : (
            <div className="player-queue-manager-empty">
              Rate tracks in the player to build a local discovery review shelf.
            </div>
          )}
        </div>
      </Modal.Content>
      <Modal.Actions>
        <Popup
          content="Clear browser-local discovery shelf review items. This does not affect files or ratings."
          trigger={
            <Button
              data-testid="player-clear-discovery-shelf"
              disabled={items.length === 0}
              onClick={clearShelf}
              type="button"
            >
              <Icon name="trash" />
              Clear Shelf
            </Button>
          }
        />
        <Popup
          content="Close the local discovery shelf."
          trigger={
            <Button
              data-testid="player-close-discovery-shelf"
              onClick={onClose}
              primary
              type="button"
            >
              <Icon name="check" />
              Done
            </Button>
          }
        />
      </Modal.Actions>
    </Modal>
  );
};

export default PlayerDiscoveryShelfModal;
