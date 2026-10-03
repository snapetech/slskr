import React from 'react';
import { isSearchComplete } from '../../../lib/searchState';
import { Button, Icon } from 'semantic-ui-react';

const SearchActionIcon = ({ loading, onRemove, onStop, search }) => {
  const searchText = search.searchText ?? search.query ?? 'search';

  if (loading) {
    return (
      <Icon
        loading
        name="spinner"
        aria-label={`Updating search: ${searchText}`}
        role="status"
      />
    );
  }

  if (isSearchComplete(search)) {
    return (
      <Button
        aria-label={`Remove search: ${searchText}`}
        color="red"
        icon="trash alternate"
        onClick={() => onRemove()}
        size="small"
        title={`Remove search: ${searchText}`}
      />
    );
  }

  return (
    <Button
      aria-label={`Stop search: ${searchText}`}
      color="red"
      icon="stop circle"
      onClick={() => onStop()}
      size="small"
      title={`Stop search: ${searchText}`}
    />
  );
};

export default SearchActionIcon;
