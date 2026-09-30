import { getWishlistRequestState } from '../../lib/acquisitionRequests';
import { toDisplayError } from '../../lib/errors';
import { encodePathSegment } from '../../lib/pathEncoding';
import { useMountedFlag } from './wishlistHooks';
import React, { useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { toast } from 'react-toastify';
import {
  Button,
  Checkbox,
  Confirm,
  Icon,
  Label,
  Popup,
  Table,
} from 'semantic-ui-react';

const formatDate = (dateString) => {
  if (!dateString) return 'Never';
  const date = new Date(dateString);
  if (Number.isNaN(date.getTime())) return 'Never';
  return date.toLocaleString();
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
          <Link to={`/searches/${encodePathSegment(item.lastSearchId)}`}>
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

export default WishlistItemRow;
