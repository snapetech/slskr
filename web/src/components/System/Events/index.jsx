import { list } from '../../../lib/events';
import { LoaderSegment } from '../../Shared';
import React, { useEffect, useState } from 'react';
import { Icon, Message, Pagination, Popup, Table } from 'semantic-ui-react';

const PER_PAGE = 10;

const replaceHyphensWithNonBreakingEquivalent = (string) =>
  string?.replaceAll('-', '‑');

const formatEventData = (data) => {
  try {
    return JSON.stringify(JSON.parse(data), null, 2);
  } catch {
    return typeof data === 'string' ? data : JSON.stringify(data);
  }
};

const getErrorMessage = (error) =>
  error?.response?.data?.detail ||
  error?.response?.data?.error ||
  error?.message ||
  'The event history could not be loaded.';

const Events = () => {
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(0);
  const [loading, setLoading] = useState(false);
  const [events, setEvents] = useState([]);
  const [error, setError] = useState();

  const paginationChanged = ({ activePage }) => {
    if (activePage >= 1) {
      setPage(activePage);
    }
  };

  useEffect(() => {
    let active = true;

    const loadEvents = async () => {
      setLoading(true);
      setError(undefined);

      try {
        const { events: items, totalCount } = await list({
          limit: PER_PAGE,
          offset: (page - 1) * PER_PAGE,
        });

        if (!active) {
          return;
        }

        const tp = Math.ceil(totalCount / PER_PAGE);

        setEvents(Array.isArray(items) ? items : []);
        setTotalPages(Number.isNaN(tp) ? 0 : tp);
      } catch (loadError) {
        if (!active) {
          return;
        }

        setEvents([]);
        setTotalPages(0);
        setError(getErrorMessage(loadError));
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    void loadEvents();

    return () => {
      active = false;
    };
  }, [page]); // eslint-disable-line react-hooks/exhaustive-deps

  if (loading) {
    return <LoaderSegment />;
  }

  return (
    <>
      <div className="header-buttons">
        <Pagination
          activePage={page}
          className="header-buttons"
          onPageChange={(event, data) => paginationChanged({ ...data })}
          totalPages={totalPages}
        />
      </div>
      {error ? <Message negative>{error}</Message> : null}
      <Table
        className="events-table, unstackable"
        compact="very"
      >
        <Table.Header>
          <Table.Row>
            <Table.HeaderCell className="events-list-id">Id</Table.HeaderCell>
            <Table.HeaderCell className="events-list-timestamp">
              Timestamp
            </Table.HeaderCell>
            <Table.HeaderCell className="events-list-type">
              Type
            </Table.HeaderCell>
            <Table.HeaderCell className="events-list-data">
              Data
            </Table.HeaderCell>
          </Table.Row>
        </Table.Header>
        <Table.Body className="events-table-body">
          {events?.length === 0 ? (
            <Table.Row>
              <Table.Cell
                colSpan={99}
                style={{
                  opacity: 0.5,
                  padding: '10px !important',
                  textAlign: 'center',
                }}
              >
                No events
              </Table.Cell>
            </Table.Row>
          ) : (
            events.map((event) => (
              <Table.Row key={event.id}>
                <Table.Cell>
                  <Popup
                    content={event.id}
                    on="hover"
                    style={{ fontFamily: 'monospace', width: '400px' }}
                    trigger={<Icon name="info circle" />}
                    wide="very"
                  />
                </Table.Cell>
                <Table.Cell>
                  {replaceHyphensWithNonBreakingEquivalent(event.timestamp)}
                </Table.Cell>
                <Table.Cell>{event.type}</Table.Cell>
                <Table.Cell className="events-table-data">
                  {formatEventData(event.data)}
                </Table.Cell>
              </Table.Row>
            ))
          )}
        </Table.Body>
      </Table>
    </>
  );
};

export default Events;
