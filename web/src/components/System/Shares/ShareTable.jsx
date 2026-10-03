import { Switch } from '../../Shared';
import React from 'react';
import { Button, Table } from 'semantic-ui-react';

const ShareTable = ({ onClick, shares }) => {
  return (
    <Table data-testid="system-shares-table">
      <Table.Header>
        <Table.Row>
          <Table.HeaderCell>Host</Table.HeaderCell>
          <Table.HeaderCell>Local Path</Table.HeaderCell>
          <Table.HeaderCell className="share-count-column">
            Directories
          </Table.HeaderCell>
          <Table.HeaderCell className="share-count-column">
            Files
          </Table.HeaderCell>
          <Table.HeaderCell>Alias</Table.HeaderCell>
          <Table.HeaderCell>Remote Path</Table.HeaderCell>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        <Switch
          empty={
            shares.length === 0 && (
              <Table.Row>
                <Table.Cell
                  colSpan={6}
                  style={{
                    opacity: 0.5,
                    padding: '10px !important',
                    textAlign: 'center',
                  }}
                >
                  No shares configured
                </Table.Cell>
              </Table.Row>
            )
          }
        >
          {shares.map((share, index) => {
            const hasLocalPath = typeof share.localPath === 'string' && share.localPath.trim() !== '';
            const localPath = hasLocalPath ? share.localPath : 'Path unavailable';

            return (
              <Table.Row key={`${share.host}+${share.localPath ?? index}`}>
              <Table.Cell>{share.host}</Table.Cell>
              <Table.Cell>
                <Button
                  aria-label={hasLocalPath
                    ? `Browse shared directory ${share.localPath}`
                    : 'Shared directory path unavailable'}
                  basic
                  compact
                  disabled={!hasLocalPath}
                  icon="folder"
                  onClick={() => onClick(share)}
                  type="button"
                >
                  {localPath}
                </Button>
              </Table.Cell>
              <Table.Cell>{share.directories ?? '?'}</Table.Cell>
              <Table.Cell>{share.files ?? '?'}</Table.Cell>
              <Table.Cell>{share.alias}</Table.Cell>
              <Table.Cell>{share.remotePath}</Table.Cell>
            </Table.Row>
            );
          })}
        </Switch>
      </Table.Body>
    </Table>
  );
};

export default ShareTable;
