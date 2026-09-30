import { toDisplayError } from '../../lib/errors';
import { readFileTextBounded } from '../../lib/fileReaders';
import { useMountedFlag } from './wishlistHooks';
import React, { useRef, useState } from 'react';
import { toast } from 'react-toastify';
import { Button, Checkbox, Form, Icon, Modal, Popup } from 'semantic-ui-react';

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

export default CsvImportModal;
