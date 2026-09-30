import './Wishlist.css';
import WishlistItemRow from './WishlistItemRow';
import WishlistModal from './WishlistModal';
import CsvImportModal from './CsvImportModal';
import { useMountedFlag } from './wishlistHooks';
import {
  buildWishlistRequestReviewPacket,
  buildWishlistRequestSummary,
  formatWishlistRequestReviewPacket,
  getRunnableWishlistRequests,
} from '../../lib/acquisitionRequests';
import { toDisplayError } from '../../lib/errors';
import * as wishlistAPI from '../../lib/wishlist';
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import {
  Button,
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

const Wishlist = () => {
  const [items, setItems] = useState([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(null);
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
    if (mountedRef.current) setLoadError(null);
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
      setLoadError(null);
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
        const message = toDisplayError(error, 'Failed to load wishlist');
        setLoadError(message);
        toast.error(message);
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

      {loadError && (
        <Message
          data-testid="wishlist-load-error"
          error
        >
          <Message.Header>Wishlist unavailable</Message.Header>
          <p>{loadError}</p>
          <p>Previously loaded wishlist items remain visible until the next successful refresh.</p>
        </Message>
      )}

      {loading ? (
        <Segment
          loading
          placeholder
        />
      ) : items.length === 0 ? (
        loadError ? null : (
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
        )
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
