import React from 'react';
import { isSearchComplete } from '../../../lib/searchState';
import { Icon } from 'semantic-ui-react';

const SearchActionIcon = ({ loading, onRemove, onStop, search, ...props }) => {
  if (loading) {
    return (
      <Icon
        loading
        name="spinner"
        {...props}
      />
    );
  }

  if (isSearchComplete(search)) {
    return (
      <Icon
        color="red"
        name="trash alternate"
        onClick={() => onRemove()}
        style={{ cursor: 'pointer' }}
      />
    );
  }

  return (
    <Icon
      color="red"
      name="stop circle"
      onClick={() => onStop()}
      style={{ cursor: 'pointer' }}
    />
  );
};

export default SearchActionIcon;
