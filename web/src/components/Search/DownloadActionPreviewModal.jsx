import { formatSearchActionPreview } from '../../lib/searchActionPreview';
import { Button, Label, List, Modal, Popup } from 'semantic-ui-react';

const DownloadActionPreviewModal = ({
  onClose,
  onCopy,
  open,
  preview,
  selectedSize,
}) => (
    <Modal
      closeIcon
      onClose={onClose}
      open={open}
      size="small"
    >
      <Modal.Header>Download action preview</Modal.Header>
      <Modal.Content>
        <List relaxed>
          <List.Item>
            <List.Header>Source</List.Header>
            <List.Description>{preview.username || 'unknown'}</List.Description>
          </List.Item>
          <List.Item>
            <List.Header>Providers</List.Header>
            <List.Description>{preview.providerLabels.join(', ')}</List.Description>
          </List.Item>
          <List.Item>
            <List.Header>Selected files</List.Header>
            <List.Description>
              {preview.fileCount} file{preview.fileCount === 1 ? '' : 's'}, {selectedSize}
            </List.Description>
          </List.Item>
          {preview.candidateScore !== null && (
            <List.Item>
              <List.Header>Candidate score</List.Header>
              <List.Description>{preview.candidateScore}/100</List.Description>
            </List.Item>
          )}
        </List>
        {preview.warnings.length > 0 && (
          <div className="search-action-preview-warnings">
            {preview.warnings.map((warning) => (
              <Label
                color="orange"
                key={warning}
                size="small"
              >
                {warning}
              </Label>
            ))}
          </div>
        )}
        <pre className="search-action-preview-text">
          {formatSearchActionPreview(preview)}
        </pre>
      </Modal.Content>
      <Modal.Actions>
        <Popup
          content="Copy this planned action summary so you can review or export it before downloading."
          position="top center"
          trigger={
            <Button
              icon="copy"
              onClick={onCopy}
            />
          }
        />
        <Button onClick={onClose}>
          Close
        </Button>
      </Modal.Actions>
    </Modal>
);

export default DownloadActionPreviewModal;
