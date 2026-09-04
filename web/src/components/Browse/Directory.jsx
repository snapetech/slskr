import * as transfers from '../../lib/transfers';
import { toDisplayError } from '../../lib/errors';
import { formatBytes } from '../../lib/util';
import FileList from '../Shared/FileList';
import React, { Component } from 'react';
import { Button, Card, Icon, Label } from 'semantic-ui-react';

const initialState = {
  downloadError: '',
  downloadRequest: undefined,
};

const getDownloadErrorMessage = (error) =>
  toDisplayError(error, 'Download failed');

const asFiles = (files) =>
  (Array.isArray(files) ? files : []).filter(
    (file) => file && typeof file === 'object' && !Array.isArray(file),
  );

class Directory extends Component {
  constructor(props) {
    super(props);

    this.state = {
      ...initialState,
      files: asFiles(this.props.files).map((f) => ({ selected: false, ...f })),
    };
    this.isMountedFlag = false;
    this.downloadRequestId = 0;
    this.downloadInFlight = false;
    this.fileSignature = JSON.stringify(
      asFiles(this.props.files).map((file) => [file.filename, file.size]),
    );
  }

  componentDidMount() {
    this.isMountedFlag = true;
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    this.downloadRequestId += 1;
  }

  componentDidUpdate(previousProps) {
    const nextFileSignature = JSON.stringify(
      asFiles(this.props.files).map((file) => [file.filename, file.size]),
    );
    if (
      this.props.name !== previousProps.name ||
      nextFileSignature !== this.fileSignature
    ) {
      this.fileSignature = nextFileSignature;
      this.setState({
        downloadError: '',
        downloadRequest: undefined,
        files: asFiles(this.props.files).map((f) => ({ selected: false, ...f })),
      });
    }
  }

  handleFileSelectionChange = (file, state) => {
    this.setState((previousState) => ({
      downloadError: '',
      downloadRequest: undefined,
      files: previousState.files.map((candidate) =>
        candidate === file || candidate.filename === file.filename
          ? { ...candidate, selected: state }
          : candidate,
      ),
    }));
  };

  download = (username, files) => {
    if (!this.isMountedFlag || this.downloadInFlight) return;
    const requestId = ++this.downloadRequestId;
    this.downloadInFlight = true;
    this.setState({ downloadRequest: 'inProgress' }, async () => {
      if (
        !this.isMountedFlag ||
        requestId !== this.downloadRequestId
      ) {
        this.downloadInFlight = false;
        return;
      }
      try {
        const requests = asFiles(files).map(({ filename, size }) => ({
          filename,
          size: Number.isFinite(Number(size)) ? Number(size) : 0,
        }));
        if (requests.length === 0) {
          this.downloadInFlight = false;
          return;
        }
        await transfers.download({ files: requests, username });

        if (
          this.isMountedFlag &&
          requestId === this.downloadRequestId
        ) {
          this.setState({ downloadRequest: 'complete' });
        }
      } catch (error) {
        if (
          this.isMountedFlag &&
          requestId === this.downloadRequestId
        ) {
          this.setState({
            downloadError: error.response || error,
            downloadRequest: 'error',
          });
        }
      } finally {
        if (
          this.isMountedFlag &&
          requestId === this.downloadRequestId
        ) {
          this.downloadInFlight = false;
        }
      }
    });
  };

  render() {
    const { locked, marginTop, name, onClose, username } = this.props;
    const { downloadError, downloadRequest, files } = this.state;

    const selectedFiles = files.filter((f) => f.selected);

   const selectedSize = formatBytes(
      selectedFiles.reduce(
        (total, f) =>
          total + (Number.isFinite(Number(f.size)) ? Number(f.size) : 0),
        0,
      ),
   );

    return (
      <Card
        className="result-card"
        raised
      >
        <Card.Content>
          <div style={{ marginTop: marginTop || 0 }}>
            <FileList
              directoryName={name}
              disabled={downloadRequest === 'inProgress'}
              files={files}
              locked={locked}
              onClose={onClose}
              onSelectionChange={this.handleFileSelectionChange}
            />
          </div>
        </Card.Content>
        {selectedFiles.length > 0 && (
          <Card.Content extra>
            <span>
              <Button
                color="green"
                content="Download"
                disabled={downloadRequest === 'inProgress'}
                icon="download"
                label={{
                  as: 'a',
                  basic: false,
                  content: `${selectedFiles.length} file${selectedFiles.length === 1 ? '' : 's'}, ${selectedSize}`,
                }}
                labelPosition="right"
                onClick={() => this.download(username, selectedFiles)}
              />
              {downloadRequest === 'inProgress' && (
                <Icon
                  loading
                  name="circle notch"
                  size="large"
                />
              )}
              {downloadRequest === 'complete' && (
                <Icon
                  color="green"
                  name="checkmark"
                  size="large"
                />
              )}
              {downloadRequest === 'error' && (
                <span>
                  <Icon
                    color="red"
                    name="x"
                    size="large"
                  />
                  <Label>
                    {getDownloadErrorMessage(downloadError)}
                    {downloadError?.status
                      ? ` (HTTP ${downloadError.status} ${downloadError.statusText})`
                      : ''}
                  </Label>
                </span>
              )}
            </span>
          </Card.Content>
        )}
      </Card>
    );
  }
}

export default Directory;
