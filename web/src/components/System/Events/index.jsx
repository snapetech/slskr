import { list } from '../../../lib/events';
import { toDisplayError } from '../../../lib/errors';
import { LoaderSegment } from '../../Shared';
import React, { useEffect, useState } from 'react';
import { Icon, Message, Pagination, Popup, Table } from 'semantic-ui-react';

const PER_PAGE = 10;

const replaceHyphensWithNonBreakingEquivalent = (string) =>
  string == null ? '' : String(string).replaceAll('-', '‑');

const formatEventData = (data) => {
  try {
    return JSON.stringify(
      typeof data === 'string' ? JSON.parse(data) : data,
      null,
      2,
    );
  } catch {
    if (typeof data === 'string') return data;
    try {
      return JSON.stringify(data);
    } catch {
      return String(data);
    }
  }
};

const getErrorMessage = (error) =>
  toDisplayError(error, 'The event history could not be loaded.');

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

        const numericTotalCount = Number(totalCount);
        const tp = Number.isFinite(numericTotalCount)
          ? Math.max(0, Math.ceil(numericTotalCount / PER_PAGE))
          : 0;

        setEvents(
          (Array.isArray(items) ? items : []).filter(
            (event) =>
              event && typeof event === 'object' && !Array.isArray(event),
          ),
        );
        setTotalPages(tp);
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
            events.map((event, index) => (
              <Table.Row key={`${event.id ?? 'event'}-${index}`}>
                <Table.Cell>
                  <Popup
                    content={String(event.id ?? '')}
                    on="hover"
                    style={{ fontFamily: 'monospace', width: '400px' }}
                    trigger={<Icon name="info circle" />}
                    wide="very"
                  />
                </Table.Cell>
                <Table.Cell>
                  {replaceHyphensWithNonBreakingEquivalent(event.timestamp)}
                </Table.Cell>
                <Table.Cell>{String(event.type ?? '')}</Table.Cell>
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
