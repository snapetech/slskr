import {
  formatAttributes,
  formatBytes,
  formatSeconds,
  getFileName,
} from '../../lib/util';
import React, { useMemo, useState } from 'react';
import { Checkbox, Header, Icon, List, Table } from 'semantic-ui-react';

const asFiles = (files) =>
  (Array.isArray(files) ? files : []).filter(
    (file) => file && typeof file === 'object' && !Array.isArray(file),
  );

const fileName = (file) =>
  typeof file?.filename === 'string' && file.filename
    ? file.filename
    : 'Unknown file';

const safeNumber = (value) => {
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 ? number : 0;
};

const FileList = ({
  directoryName = '',
  disabled,
  files = [],
  footer,
  locked,
  onClose,
  onSelectionChange,
}) => {
  const [folded, setFolded] = useState(false);
  const [lastSelectedIndex, setLastSelectedIndex] = useState(null);
  const safeFiles = useMemo(() => asFiles(files), [files]);
  const sortedFiles = useMemo(
    () =>
      [...safeFiles].sort((left, right) =>
        fileName(left).localeCompare(fileName(right)),
      ),
    [safeFiles],
  );
  const handleSelectionChange = (event, file, index, checked) => {
    const shiftKey = event.shiftKey || event.nativeEvent?.shiftKey;
    if (
      shiftKey &&
      lastSelectedIndex !== null &&
      lastSelectedIndex !== index
    ) {
      const start = Math.min(lastSelectedIndex, index);
      const end = Math.max(lastSelectedIndex, index);
      sortedFiles
        .slice(start, end + 1)
        .forEach((rangeFile) => onSelectionChange(rangeFile, checked));
    } else {
      onSelectionChange(file, checked);
    }

    setLastSelectedIndex(index);
  };

  return (
    <div
      className="filelist"
      style={{ opacity: locked ? 0.5 : 1 }}
    >
      <Header
        className="filelist-header"
        size="small"
      >
        <div className="filelist-title">
          <Icon
            link={!locked}
            name={locked ? 'lock' : folded ? 'folder' : 'folder open'}
            onClick={() => !locked && setFolded(!folded)}
            size="large"
          />
          {typeof directoryName === 'string' ? directoryName : ''}

          {Boolean(onClose) && (
            <Icon
              className="close-button"
              color="red"
              link
              name="close"
              onClick={() => onClose()}
            />
          )}
        </div>
      </Header>
      {!folded && safeFiles.length > 0 && (
        <List>
          <List.Item>
            <Table className="filelist-table">
              <Table.Header>
                <Table.Row>
                  <Table.HeaderCell className="filelist-selector">
                    <Checkbox
                      checked={
                        safeFiles.length > 0 &&
                        safeFiles.filter((f) => !f.selected).length === 0
                      }
                      disabled={disabled || locked}
                      fitted
                      onChange={(event, data) =>
                        safeFiles.map((f) => onSelectionChange(f, data.checked))
                      }
                    />
                  </Table.HeaderCell>
                  <Table.HeaderCell className="filelist-filename">
                    File
                  </Table.HeaderCell>
                  <Table.HeaderCell className="filelist-size">
                    Size
                  </Table.HeaderCell>
                  <Table.HeaderCell className="filelist-attributes">
                    Attributes
                  </Table.HeaderCell>
                  <Table.HeaderCell className="filelist-length">
                    Length
                  </Table.HeaderCell>
                </Table.Row>
              </Table.Header>
              <Table.Body>
                {sortedFiles.map((f, index) => (
                    <Table.Row key={f.id || f.filename || index}>
                      <Table.Cell className="filelist-selector">
                        <Checkbox
                          checked={Boolean(f.selected)}
                          disabled={disabled || locked}
                          fitted
                          onChange={(event, data) =>
                            handleSelectionChange(event, f, index, data.checked)
                          }
                        />
                      </Table.Cell>
                      <Table.Cell className="filelist-filename">
                        {locked ? <Icon name="lock" /> : ''}
                        {getFileName(fileName(f))}
                      </Table.Cell>
                      <Table.Cell className="filelist-size">
                        {formatBytes(safeNumber(f.size))}
                      </Table.Cell>
                      <Table.Cell className="filelist-attributes">
                        {formatAttributes(f)}
                      </Table.Cell>
                      <Table.Cell className="filelist-length">
                        {formatSeconds(safeNumber(f.length))}
                      </Table.Cell>
                    </Table.Row>
                  ))}
              </Table.Body>
              {footer && (
                <Table.Footer fullWidth>
                  <Table.Row>
                    <Table.HeaderCell colSpan="5">{footer}</Table.HeaderCell>
                  </Table.Row>
                </Table.Footer>
              )}
            </Table>
          </List.Item>
        </List>
      )}
    </div>
  );
};

export default FileList;
