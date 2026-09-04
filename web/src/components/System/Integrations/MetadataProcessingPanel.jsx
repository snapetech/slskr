import React, { useState } from 'react';
import { Card, Header, Icon, Label, Message, Table } from 'semantic-ui-react';
import { getMetadataProcessingStatus } from '../../../lib/slskr';
import { useMountedRef } from '../../../lib/useMountedRef';
import { usePolling } from '../../../lib/usePolling';

const labelColor = (status) =>
  ({ complete: 'green', failed: 'red', running: 'blue', skipped: 'grey' })[
    status
  ] || 'grey';

const MetadataProcessingPanel = () => {
  const [status, setStatus] = useState({ active: [], history: [] });
  const mountedRef = useMountedRef();

  usePolling(async () => {
    const next = await getMetadataProcessingStatus(50);
    if (!mountedRef.current) return;
    setStatus({
      active: Array.isArray(next?.active) ? next.active : [],
      history: Array.isArray(next?.history) ? next.history : [],
    });
  }, 5_000);

  const rows = [...status.active, ...status.history].filter(
    (item) => item && typeof item === 'object',
  );
  return (
    <Card fluid>
      <Card.Content>
        <Header as="h3">
          <Icon name="music" /> Metadata processing activity
        </Header>
        <p>
          Live and recent hashing, Chromaprint, AcoustID, and MusicBrainz stages
          for completed audio downloads.
        </p>
        {rows.length === 0 ? (
          <Message
            info
            content="No metadata processing activity has been recorded since startup."
          />
        ) : (
          <Table compact selectable stackable>
            <Table.Header>
              <Table.Row>
                <Table.HeaderCell>File</Table.HeaderCell>
                <Table.HeaderCell>Stage</Table.HeaderCell>
                <Table.HeaderCell>Status</Table.HeaderCell>
                <Table.HeaderCell>Started</Table.HeaderCell>
                <Table.HeaderCell>Result</Table.HeaderCell>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {rows.map((item, index) => (
                <Table.Row key={item.id || `${item.filename || 'item'}-${index}`}>
                  <Table.Cell>{item.filename}</Table.Cell>
                  <Table.Cell>{item.stage}</Table.Cell>
                  <Table.Cell>
                    <Label color={labelColor(item.status)}>{item.status}</Label>
                  </Table.Cell>
                  <Table.Cell>
                    {item.startedAt ? new Date(item.startedAt).toLocaleString() : '-'}
                  </Table.Cell>
                  <Table.Cell>{item.detail || 'In progress'}</Table.Cell>
                </Table.Row>
              ))}
            </Table.Body>
          </Table>
        )}
      </Card.Content>
    </Card>
  );
};

export default MetadataProcessingPanel;
