import * as transfers from '../../lib/transfers';
import { formatBytes } from '../../lib/util';
import FileList from '../Shared/FileList';
import React, { Component } from 'react';
import { Button, Card, Icon, Label } from 'semantic-ui-react';

const initialState = {
  downloadError: '',
  downloadRequest: undefined,
};

const getDownloadErrorMessage = (error) => {
  if (typeof error?.data === 'string') return error.data;
  if (typeof error?.data?.message === 'string') return error.data.message;
  return error?.message || 'Download failed';
};

class Directory extends Component {
  constructor(props) {
    super(props);

    this.state = {
      ...initialState,
      files: this.props.files.map((f) => ({ selected: false, ...f })),
    };
    this.isMountedFlag = false;
    this.downloadRequestId = 0;
  }

  componentDidMount() {
    this.isMountedFlag = true;
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    this.downloadRequestId += 1;
  }

  componentDidUpdate(previousProps) {
    if (this.props.name !== previousProps.name) {
      this.setState({
        files: this.props.files.map((f) => ({ selected: false, ...f })),
      });
    }
  }

  handleFileSelectionChange = (file, state) => {
    file.selected = state;
    this.setState((previousState) => ({
      downloadError: '',
      downloadRequest: undefined,
      tree: previousState.tree,
    }));
  };

  download = (username, files) => {
    const requestId = ++this.downloadRequestId;
    this.setState({ downloadRequest: 'inProgress' }, async () => {
      if (
        !this.isMountedFlag ||
        requestId !== this.downloadRequestId
      ) {
        return;
      }
      try {
        const requests = (files || []).map(({ filename, size }) => ({
          filename,
          size,
        }));
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
            downloadError: error.response || {
              data: error.message || 'Download failed',
              status: 0,
              statusText: 'Network error',
            },
            downloadRequest: 'error',
          });
        }
      }
    });
  };

  render() {
    const { locked, marginTop, name, onClose, username } = this.props;
    const { downloadError, downloadRequest, files } = this.state;

    const selectedFiles = files.filter((f) => f.selected);

    const selectedSize = formatBytes(
      selectedFiles.reduce((total, f) => total + f.size, 0),
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
