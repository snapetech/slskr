import { getSearchStateKind } from '../../lib/searchState';
import React from 'react';
import { Icon, Popup } from 'semantic-ui-react';

const getIcon = ({ state, status, ...props }) => {
  switch (getSearchStateKind({ state, status })) {
    case 'active':
      if (
        ['none', 'queued', 'requested'].includes(
          String(state ?? status ?? '').trim().toLowerCase(),
        )
      ) {
        return (
          <Icon
            name="time"
            {...props}
          />
        );
      }
      return (
        <Icon
          color="green"
          loading
          name="circle notch"
          {...props}
        />
      );
    case 'completed':
      return (
        <Icon
          color="green"
          name="check"
          {...props}
        />
      );
    case 'cancelled':
      return (
        <Icon
          color="green"
          name="stop circle"
          {...props}
        />
      );
    case 'failed':
      return (
        <Icon
          color="red"
          name="x"
          {...props}
        />
      );
    default:
      return (
        <Icon
          color="yellow"
          name="question circle"
          {...props}
        />
      );
  }
};

const SearchStatusIcon = ({ state, status, ...props }) => (
  <Popup
    content={status ?? state ?? 'Unknown'}
    trigger={getIcon({ state, status, ...props })}
  />
);

export default SearchStatusIcon;
