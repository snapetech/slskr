import { toDisplayError } from '../../lib/errors';
import * as wishlistAPI from '../../lib/wishlist';
import { useMountedFlag } from './wishlistHooks';
import React, { useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import {
  Button,
  Checkbox,
  Form,
  Header,
  Icon,
  Popup,
  Segment,
  Table,
  Modal,
} from 'semantic-ui-react';

const WishlistModal = ({ item, onClose, onSave }) => {
  const [searchText, setSearchText] = useState(item?.searchText || '');
  const [filter, setFilter] = useState(item?.filter || '');
  const [enabled, setEnabled] = useState(item?.enabled ?? true);
  const [autoDownload, setAutoDownload] = useState(item?.autoDownload ?? false);
  const [maxResults, setMaxResults] = useState(item?.maxResults ?? 100);
  const [saving, setSaving] = useState(false);
  const [ignoredResults, setIgnoredResults] = useState([]);
  const [loadingIgnoredResults, setLoadingIgnoredResults] = useState(false);
  const mountedRef = useMountedFlag();
  const operationRequestIdRef = useRef(0);
  const restoreInFlightRef = useRef(false);

  const isEdit = Boolean(item?.id);

  useEffect(() => {
    if (!isEdit) return undefined;

    let cancelled = false;
    setLoadingIgnoredResults(true);
    wishlistAPI
      .getIgnoredResults(item.id)
      .then((rules) => {
        if (!cancelled) {
          setIgnoredResults(Array.isArray(rules) ? rules : []);
        }
      })
      .catch((error) => {
        console.error(error);
        if (!cancelled) {
          toast.error('Failed to load ignored wishlist folders');
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingIgnoredResults(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [isEdit, item?.id]);

  const restoreIgnoredResult = async (rule) => {
    if (restoreInFlightRef.current || !mountedRef.current) return;
    restoreInFlightRef.current = true;
    const requestId = ++operationRequestIdRef.current;
    try {
      await wishlistAPI.removeIgnoredResult(item.id, rule.id);
      if (
        !mountedRef.current ||
        requestId !== operationRequestIdRef.current
      ) {
        return;
      }
      setIgnoredResults((current) =>
        current.filter((candidate) => candidate.id !== rule.id),
      );
      toast.info(`Restored ${rule.directory} from ${rule.username}`);
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        console.error(error);
        toast.error(toDisplayError(error));
      }
    } finally {
      restoreInFlightRef.current = false;
    }
  };

  const handleSave = async () => {
    if (!searchText.trim() || saving || !mountedRef.current) {
      if (!searchText.trim()) toast.error('Search text is required');
      return;
    }

    const requestId = ++operationRequestIdRef.current;
    setSaving(true);
    try {
      await onSave({
        autoDownload,
        enabled,
        filter: filter.trim() || undefined,
        id: item?.id,
        maxResults,
        searchText: searchText.trim(),
      });
      if (
        !mountedRef.current ||
        requestId !== operationRequestIdRef.current
      ) {
        return;
      }
      onClose();
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        toast.error(`Failed to save: ${toDisplayError(error)}`);
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        setSaving(false);
      }
    }
  };

  return (
    <Modal
      onClose={onClose}
      open
      size="small"
    >
      <Modal.Header>
        <Icon name="star" />
        {isEdit ? 'Edit Wishlist Item' : 'Add to Wishlist'}
      </Modal.Header>
      <Modal.Content>
        <Form>
          <Form.Input
            label="Search Text"
            onChange={(event) => setSearchText(event.target.value)}
            placeholder="Enter search terms..."
            required
            value={searchText}
          />
          <Form.Input
            label="Filter (optional)"
            onChange={(event) => setFilter(event.target.value)}
            placeholder="e.g., flac OR mp3"
            value={filter}
          />
          <Form.Input
            label="Max Results"
            max={1_000}
            min={10}
            onChange={(event) =>
              setMaxResults(Number.parseInt(event.target.value, 10) || 100)
            }
            type="number"
            value={maxResults}
          />
          <Form.Field>
            <Checkbox
              checked={enabled}
              label="Enabled (run automatically)"
              onChange={(_, data) => setEnabled(data.checked)}
              toggle
            />
          </Form.Field>
          <Form.Field>
            <Checkbox
              checked={autoDownload}
              label="Auto-download best matches"
              onChange={(_, data) => setAutoDownload(data.checked)}
              toggle
            />
          </Form.Field>
        </Form>
        {isEdit && (
          <Segment>
            <Header as="h4">
              <Icon name="eye slash" />
              <Header.Content>
                Ignored Result Folders
                <Header.Subheader>
                  These peer folders stay hidden only for this wishlist item.
                  Restore one to allow it in future searches and auto-download
                  decisions.
                </Header.Subheader>
              </Header.Content>
            </Header>
            {loadingIgnoredResults ? (
              <Icon loading name="spinner" />
            ) : ignoredResults.length === 0 ? (
              <span>No folders are ignored.</span>
            ) : (
              <Table basic="very" compact>
                <Table.Body>
                  {ignoredResults.map((rule) => (
                    <Table.Row key={rule.id}>
                      <Table.Cell>
                        <strong>{rule.username}</strong>
                        <div
                          className="truncate-cell"
                          title={rule.directory}
                        >
                          {rule.directory}
                        </div>
                      </Table.Cell>
                      <Table.Cell collapsing>
                        <Popup
                          content="Allow this peer folder to appear again in future runs of this wishlist item."
                          trigger={
                            <Button
                              aria-label={`Restore ignored folder ${rule.directory}`}
                              compact
                              icon="undo"
                              onClick={() => restoreIgnoredResult(rule)}
                              size="tiny"
                            />
                          }
                        />
                      </Table.Cell>
                    </Table.Row>
                  ))}
                </Table.Body>
              </Table>
            )}
          </Segment>
        )}
      </Modal.Content>
      <Modal.Actions>
        <Button onClick={onClose}>Cancel</Button>
        <Button
          disabled={saving}
          loading={saving}
          onClick={handleSave}
          primary
        >
          {isEdit ? 'Save' : 'Add'}
        </Button>
      </Modal.Actions>
    </Modal>
  );
};

export default WishlistModal;
