import './Wishlist.css';
import { urlBase } from '../../config';
import {
  buildWishlistRequestReviewPacket,
  buildWishlistRequestSummary,
  formatWishlistRequestReviewPacket,
  getWishlistRequestState,
  getRunnableWishlistRequests,
} from '../../lib/acquisitionRequests';
import { toDisplayError } from '../../lib/errors';
import { readFileTextBounded } from '../../lib/fileReaders';
import * as wishlistAPI from '../../lib/wishlist';
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { toast } from 'react-toastify';
import {
  Button,
  Checkbox,
  Confirm,
  Form,
  Header,
  Icon,
  Label,
  Modal,
  Popup,
  Segment,
  Table,
} from 'semantic-ui-react';

const formatDate = (dateString) => {
  if (!dateString) return 'Never';
  const date = new Date(dateString);
  if (Number.isNaN(date.getTime())) return 'Never';
  return date.toLocaleString();
};

const useMountedFlag = () => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  return mountedRef;
};

const WishlistItemRow = ({
  item,
  onDelete,
  onEdit,
  onRunSearch,
  onSelect,
  selected,
}) => {
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [running, setRunning] = useState(false);
  const mountedRef = useMountedFlag();
  const requestIdRef = useRef(0);
  const inFlightRef = useRef(false);
  const requestState = getWishlistRequestState(item, []);

  const handleRunSearch = async () => {
    if (inFlightRef.current || !mountedRef.current) return;
    inFlightRef.current = true;
    const requestId = ++requestIdRef.current;
    setRunning(true);
    try {
      const result = await onRunSearch(item.id);
      if (
        mountedRef.current &&
        requestId === requestIdRef.current
      ) {
        const responseCount =
          result?.responseCount ?? result?.ResponseCount ?? 0;
        toast.success(`Search completed with ${responseCount} results`);
      }
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === requestIdRef.current
      ) {
        toast.error(`Search failed: ${toDisplayError(error)}`);
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === requestIdRef.current
      ) {
        setRunning(false);
      }
      inFlightRef.current = false;
    }
  };

  return (
    <Table.Row>
      <Table.Cell>
        <Checkbox
          aria-label={`Select ${item.searchText} for bulk actions`}
          checked={selected}
          onChange={(_, { checked }) => onSelect(item.id, checked)}
        />
      </Table.Cell>
      <Table.Cell>
        <Icon
          color={item.enabled ? 'green' : 'grey'}
          name={item.enabled ? 'check circle' : 'circle outline'}
        />
      </Table.Cell>
      <Table.Cell>
        <strong>{item.searchText}</strong>
        {item.filter && (
          <div className="wishlist-filter">Filter: {item.filter}</div>
        )}
      </Table.Cell>
      <Table.Cell textAlign="center">
        <Popup
          content="Auto-download best matches"
          trigger={
            <Icon
              color={item.autoDownload ? 'green' : 'grey'}
              name={item.autoDownload ? 'download' : 'download'}
            />
          }
        />
      </Table.Cell>
      <Table.Cell>{formatDate(item.lastSearchedAt)}</Table.Cell>
      <Table.Cell textAlign="center">{item.lastMatchCount}</Table.Cell>
      <Table.Cell textAlign="center">{item.totalSearchCount}</Table.Cell>
      <Table.Cell>
        <Popup
          content={requestState.summary}
          position="top center"
          trigger={
            <Label color={requestState.color}>
              {requestState.label}
            </Label>
          }
        />
      </Table.Cell>
      <Table.Cell>
        {item.lastSearchId && (
          <Link to={`${urlBase}/searches/${encodeURIComponent(item.lastSearchId)}`}>
            <Button
              compact
              icon="search"
              size="tiny"
              title="View last search results"
            />
          </Link>
        )}
        <Button
          compact
          disabled={running}
          icon="play"
          loading={running}
          onClick={handleRunSearch}
          primary
          size="tiny"
          title="Run search now"
        />
        <Button
          compact
          icon="edit"
          onClick={() => onEdit(item)}
          size="tiny"
          title="Edit"
        />
        <Button
          color="red"
          compact
          icon="trash"
          onClick={() => setConfirmDelete(true)}
          size="tiny"
          title="Delete"
        />
        <Confirm
          cancelButton="Cancel"
          confirmButton="Delete"
          content={`Delete wishlist item "${item.searchText}"?`}
          header="Confirm Delete"
          onCancel={() => setConfirmDelete(false)}
          onConfirm={() => {
            setConfirmDelete(false);
            onDelete(item.id);
          }}
          open={confirmDelete}
          size="mini"
        />
      </Table.Cell>
    </Table.Row>
  );
};

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

const CsvImportModal = ({ onClose, onImport }) => {
  const [csvText, setCsvText] = useState('');
  const [filter, setFilter] = useState('');
  const [enabled, setEnabled] = useState(true);
  const [autoDownload, setAutoDownload] = useState(false);
  const [includeAlbum, setIncludeAlbum] = useState(false);
  const [maxResults, setMaxResults] = useState(100);
  const [importing, setImporting] = useState(false);
  const mountedRef = useMountedFlag();
  const operationRequestIdRef = useRef(0);
  const importInFlightRef = useRef(false);

  const handleFile = async (event) => {
    const file = event.target.files?.[0];
    if (!file) return;
    try {
      const text = await readFileTextBounded(file, 768 * 1024);
      if (mountedRef.current) {
        setCsvText(text);
      }
    } catch (error) {
      if (mountedRef.current) {
        toast.error(`Failed to read CSV file: ${toDisplayError(error)}`);
      }
    }
  };

  const handleImport = async () => {
    if (!csvText.trim()) {
      toast.error('CSV text is required');
      return;
    }
    if (importInFlightRef.current || !mountedRef.current) return;
    importInFlightRef.current = true;

    const requestId = ++operationRequestIdRef.current;
    setImporting(true);
    try {
      await onImport({
        autoDownload,
        csvText,
        enabled,
        filter: filter.trim() || undefined,
        includeAlbum,
        maxResults,
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
        toast.error(`CSV import failed: ${toDisplayError(error)}`);
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        setImporting(false);
      }
      importInFlightRef.current = false;
    }
  };

  return (
    <Modal
      onClose={onClose}
      open
      size="small"
    >
      <Modal.Header>
        <Icon name="file alternate outline" />
        Import CSV Playlist
      </Modal.Header>
      <Modal.Content>
        <Form>
          <Form.Input
            accept=".csv,text/csv"
            label="CSV File"
            onChange={handleFile}
            type="file"
          />
          <Form.TextArea
            label="CSV Text"
            onChange={(event) => setCsvText(event.target.value)}
            placeholder="Track name,Artist name,Album name"
            rows={8}
            value={csvText}
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
            min={1}
            onChange={(event) =>
              setMaxResults(Number.parseInt(event.target.value, 10) || 100)
            }
            type="number"
            value={maxResults}
          />
          <Form.Group widths="equal">
            <Form.Field>
              <Checkbox
                checked={enabled}
                label="Enabled"
                onChange={(_, data) => setEnabled(data.checked)}
                toggle
              />
            </Form.Field>
            <Form.Field>
              <Checkbox
                checked={autoDownload}
                label="Auto-download matches"
                onChange={(_, data) => setAutoDownload(data.checked)}
                toggle
              />
            </Form.Field>
            <Form.Field>
              <Checkbox
                checked={includeAlbum}
                label="Include album"
                onChange={(_, data) => setIncludeAlbum(data.checked)}
                toggle
              />
            </Form.Field>
          </Form.Group>
        </Form>
      </Modal.Content>
      <Modal.Actions>
        <Popup
          content="Close the CSV importer without adding any wishlist searches."
          trigger={<Button onClick={onClose}>Cancel</Button>}
        />
        <Popup
          content="Create wishlist searches from the parsed CSV rows using the selected options."
          trigger={
            <Button
              disabled={importing}
              loading={importing}
              onClick={handleImport}
              primary
            >
              Import
            </Button>
          }
        />
      </Modal.Actions>
    </Modal>
  );
};

const Wishlist = () => {
  const [items, setItems] = useState([]);
  const [loading, setLoading] = useState(true);
  const [modalItem, setModalItem] = useState(null);
  const [showModal, setShowModal] = useState(false);
  const [showImportModal, setShowImportModal] = useState(false);
  const [requestCopyStatus, setRequestCopyStatus] = useState('');
  const [bulkRunning, setBulkRunning] = useState(false);
  const [bulkFilter, setBulkFilter] = useState('');
  const [selectedIds, setSelectedIds] = useState(() => new Set());
  const mountedRef = useMountedFlag();
  const loadRequestIdRef = useRef(0);
  const operationRequestIdRef = useRef(0);
  const operationInFlightRef = useRef(false);
  const requestSummary = useMemo(
    () =>
      buildWishlistRequestSummary({
        items,
      }),
    [items],
  );
  const runnableRequests = useMemo(
    () => getRunnableWishlistRequests(items, { limit: 3 }),
    [items],
  );

  const beginOperation = () => {
    if (!mountedRef.current || operationInFlightRef.current) return false;
    operationInFlightRef.current = true;
    return true;
  };

  const finishOperation = () => {
    operationInFlightRef.current = false;
  };

  const copyRequestReviewPacket = async () => {
    const packet = buildWishlistRequestReviewPacket({
      items,
    });
    const report = formatWishlistRequestReviewPacket(packet);

    if (!navigator.clipboard?.writeText) {
      setRequestCopyStatus('Clipboard unavailable; copy the request summary manually.');
      return;
    }

    try {
      await navigator.clipboard.writeText(report);
      if (mountedRef.current) {
        setRequestCopyStatus('Wishlist request review copied.');
      }
    } catch {
      if (mountedRef.current) {
        setRequestCopyStatus('Unable to copy Wishlist request review.');
      }
    }
  };

  const runEnabledSearches = async () => {
    if (!beginOperation()) return;
    const requestId = ++operationRequestIdRef.current;
    setBulkRunning(true);
    const results = [];

    try {
      for (const item of runnableRequests) {
        if (
          !mountedRef.current ||
          requestId !== operationRequestIdRef.current
        ) {
          return;
        }
        try {
          const result = await wishlistAPI.runSearch(item.id);
          if (
            !mountedRef.current ||
            requestId !== operationRequestIdRef.current
          ) {
            return;
          }
          results.push({
            id: item.id,
            responseCount: result?.responseCount ?? result?.ResponseCount ?? 0,
            status: 'ran',
          });
        } catch (error) {
          if (
            !mountedRef.current ||
            requestId !== operationRequestIdRef.current
          ) {
            return;
          }
          results.push({
            error: toDisplayError(error, 'Search failed'),
            id: item.id,
            status: 'failed',
          });
        }
      }

      const ran = results.filter((result) => result.status === 'ran').length;
      const failed = results.filter((result) => result.status === 'failed').length;
      if (
        !mountedRef.current ||
        requestId !== operationRequestIdRef.current
      ) {
        return;
      }
      setRequestCopyStatus(
        `Ran ${ran} enabled Wishlist search${ran === 1 ? '' : 'es'}${
          failed ? `; ${failed} failed` : ''
        }. Downloads still require normal result selection and policy.`,
      );
      await loadItems();
    } finally {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        setBulkRunning(false);
      }
      finishOperation();
    }
  };

  const loadItems = useCallback(async () => {
    const requestId = ++loadRequestIdRef.current;
    try {
      const data = await wishlistAPI.getAll();
      if (
        !mountedRef.current ||
        requestId !== loadRequestIdRef.current
      ) {
        return;
      }
      const nextItems = Array.isArray(data) ? data : [];
      setItems(nextItems);
      setSelectedIds((current) => {
        const availableIds = new Set(nextItems.map((item) => item.id));
        return new Set(
          [...current].filter((itemId) => availableIds.has(itemId)),
        );
      });
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === loadRequestIdRef.current
      ) {
        toast.error(`Failed to load wishlist: ${toDisplayError(error)}`);
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === loadRequestIdRef.current
      ) {
        setLoading(false);
      }
    }
  }, []);

  const toggleSelection = (id, selected) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (selected) {
        next.add(id);
      } else {
        next.delete(id);
      }
      return next;
    });
  };

  const toggleAllSelections = (selected) => {
    setSelectedIds(selected ? new Set(items.map((item) => item.id)) : new Set());
  };

  const handleBulkFilter = async () => {
    const ids = [...selectedIds];
    if (ids.length === 0 || !beginOperation()) return;

    const requestId = ++operationRequestIdRef.current;
    setBulkRunning(true);
    try {
      const result = await wishlistAPI.updateFilters(ids, bulkFilter.trim());
      if (
        !mountedRef.current ||
        requestId !== operationRequestIdRef.current
      ) {
        return;
      }
      const updatedCount = result?.updatedCount ?? result?.UpdatedCount ?? ids.length;
      toast.success(`Updated filters for ${updatedCount} item(s)`);
      setSelectedIds(new Set());
      setBulkFilter('');
      await loadItems();
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        toast.error(`Failed to update filters: ${toDisplayError(error)}`);
      }
    } finally {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        setBulkRunning(false);
      }
      finishOperation();
    }
  };

  useEffect(() => {
    void loadItems();
  }, [loadItems]);

  const handleAdd = () => {
    setModalItem(null);
    setShowModal(true);
  };

  const handleImportClick = () => {
    setShowImportModal(true);
  };

  const handleEdit = (item) => {
    setModalItem(item);
    setShowModal(true);
  };

  const handleSave = async (item) => {
    if (!beginOperation()) return;
    try {
      if (item.id) {
        await wishlistAPI.update(item.id, item);
        if (!mountedRef.current) return;
        toast.success('Wishlist item updated');
      } else {
        await wishlistAPI.create(item);
        if (!mountedRef.current) return;
        toast.success('Added to wishlist');
      }

      await loadItems();
    } finally {
      finishOperation();
    }
  };

  const handleDelete = async (id) => {
    if (!beginOperation()) return;
    const requestId = ++operationRequestIdRef.current;
    try {
      await wishlistAPI.remove(id);
      if (
        !mountedRef.current ||
        requestId !== operationRequestIdRef.current
      ) {
        return;
      }
      toast.success('Wishlist item deleted');
      await loadItems();
    } catch (error) {
      if (
        mountedRef.current &&
        requestId === operationRequestIdRef.current
      ) {
        toast.error(`Failed to delete: ${toDisplayError(error)}`);
      }
    } finally {
      finishOperation();
    }
  };

  const handleRunSearch = async (id) => {
    if (!beginOperation()) return undefined;
    try {
      const result = await wishlistAPI.runSearch(id);
      await loadItems();
      return result;
    } finally {
      finishOperation();
    }
  };

  const handleImport = async (request) => {
    if (!beginOperation()) return;
    try {
      const result = await wishlistAPI.importCsv(request);
      if (!mountedRef.current) return;
      toast.success(
        `Imported ${result?.createdCount ?? 0} searches (${result?.duplicateCount ?? 0} duplicates, ${result?.skippedCount ?? 0} skipped)`,
      );
      await loadItems();
    } finally {
      finishOperation();
    }
  };

  return (
    <div className="wishlist-container">
      <Segment
        className="wishlist-header"
        clearing
      >
        <Header
          as="h2"
          floated="left"
        >
          <Icon name="star" />
          <Header.Content>
            Wishlist
            <Header.Subheader>
              Saved searches that run automatically
            </Header.Subheader>
          </Header.Content>
        </Header>
        <Popup
          content="Add one saved search to the wishlist. Enabled wishlist entries run later using the normal conservative scheduler."
          trigger={
            <Button
              floated="right"
              icon
              labelPosition="left"
              onClick={handleAdd}
              primary
            >
              <Icon name="plus" />
              Add Search
            </Button>
          }
        />
        <Popup
          content="Import a playlist CSV, such as a TuneMyMusic export, into wishlist searches without starting a large search burst immediately."
          trigger={
            <Button
              floated="right"
              icon
              labelPosition="left"
              onClick={handleImportClick}
            >
              <Icon name="file alternate outline" />
              Import CSV
            </Button>
          }
        />
      </Segment>

      {!loading && (
        <Segment className="wishlist-request-summary">
          <div className="wishlist-request-summary-header">
            <Header as="h3">
              <Icon name="clipboard check" />
              Request Portal Summary
              <Header.Subheader>
                Operator view of wanted music before acquisition jobs are wired.
              </Header.Subheader>
            </Header>
            <Popup
              content="Copy the current Wishlist request review packet. This does not start searches, peer browsing, downloads, or automation."
              position="top center"
              trigger={
                <Button
                  aria-label="Copy Wishlist request review"
                  onClick={copyRequestReviewPacket}
                  size="small"
                >
                  <Icon name="copy" />
                  Copy Review
                </Button>
              }
            />
            <Popup
              content="Run up to three enabled Wishlist searches now through the backend. This starts search jobs only; downloads still require the normal result selection and policy."
              position="top center"
              trigger={
                <Button
                  aria-label="Run enabled Wishlist searches"
                  disabled={runnableRequests.length === 0}
                  loading={bulkRunning}
                  onClick={runEnabledSearches}
                  primary
                  size="small"
                >
                  <Icon name="play" />
                  Run Enabled
                </Button>
              }
            />
          </div>
          <div className="wishlist-request-summary-grid">
            {/* Plain counts get a neutral pill; color is reserved for the two
                pills below that actually report a state worth noticing. */}
            <Label basic>
              Requests
              <Label.Detail>{requestSummary.total}</Label.Detail>
            </Label>
            <Label basic>
              Enabled
              <Label.Detail>{requestSummary.enabled}</Label.Detail>
            </Label>
            <Label basic>
              Automatic
              <Label.Detail>{requestSummary.automatic}</Label.Detail>
            </Label>
            <Label color={requestSummary.reviewCount > 0 ? 'yellow' : 'grey'}>
              Needs Review
              <Label.Detail>{requestSummary.reviewCount}</Label.Detail>
            </Label>
            <Label color={requestSummary.quotaStatus === 'Within quota' ? 'green' : 'orange'}>
              {requestSummary.quotaStatus}
              <Label.Detail>{requestSummary.quotaRemaining} left</Label.Detail>
            </Label>
          </div>
          {requestCopyStatus && (
            <Label basic>
              {requestCopyStatus}
            </Label>
          )}
        </Segment>
      )}

      {selectedIds.size > 0 && (
        <Segment className="wishlist-bulk-actions">
          <Header as="h4">
            <Icon name="tasks" />
            Bulk actions ({selectedIds.size})
          </Header>
          <Form>
            <Form.Input
              aria-label="Bulk wishlist filter"
              label="Apply filter to selected items"
              onChange={(event) => setBulkFilter(event.target.value)}
              placeholder="e.g., flac OR mp3"
              value={bulkFilter}
            />
            <Button
              aria-label="Apply filter to selected wishlist items"
              disabled={bulkRunning}
              loading={bulkRunning}
              onClick={handleBulkFilter}
              primary
            >
              <Icon name="filter" />
              Apply Filter
            </Button>
          </Form>
        </Segment>
      )}

      {loading ? (
        <Segment
          loading
          placeholder
        />
      ) : items.length === 0 ? (
        <Segment
          inverted
          placeholder
        >
          <Header
            icon
            inverted
          >
            <Icon name="star outline" />
            No wishlist items yet
          </Header>
          <p>
            Add searches to your wishlist and they&apos;ll run automatically.
          </p>
          <Button
            onClick={handleAdd}
            primary
          >
            Add Your First Search
          </Button>
        </Segment>
      ) : (
        <Table
          celled
          striped
        >
          <Table.Header>
            <Table.Row>
              <Table.HeaderCell width={1}>
                <Checkbox
                  aria-label="Select all wishlist items for bulk actions"
                  checked={items.length > 0 && selectedIds.size === items.length}
                  onChange={(_, { checked }) => toggleAllSelections(checked)}
                />
              </Table.HeaderCell>
              <Table.HeaderCell width={1}>Active</Table.HeaderCell>
              <Table.HeaderCell>Search</Table.HeaderCell>
              <Table.HeaderCell
                textAlign="center"
                width={1}
              >
                Auto
              </Table.HeaderCell>
              <Table.HeaderCell width={3}>Last Run</Table.HeaderCell>
              <Table.HeaderCell
                textAlign="center"
                width={1}
              >
                Matches
              </Table.HeaderCell>
              <Table.HeaderCell
                textAlign="center"
                width={1}
              >
                Runs
              </Table.HeaderCell>
              <Table.HeaderCell width={2}>Request State</Table.HeaderCell>
              <Table.HeaderCell width={3}>Actions</Table.HeaderCell>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {items.map((item) => (
              <WishlistItemRow
                item={item}
                key={item.id}
                onDelete={handleDelete}
                onEdit={handleEdit}
                onRunSearch={handleRunSearch}
                onSelect={toggleSelection}
                selected={selectedIds.has(item.id)}
              />
            ))}
          </Table.Body>
        </Table>
      )}

      {showModal && (
        <WishlistModal
          item={modalItem}
          onClose={() => setShowModal(false)}
          onSave={handleSave}
        />
      )}

      {showImportModal && (
        <CsvImportModal
          onClose={() => setShowImportModal(false)}
          onImport={handleImport}
        />
      )}
    </div>
  );
};

export default Wishlist;
