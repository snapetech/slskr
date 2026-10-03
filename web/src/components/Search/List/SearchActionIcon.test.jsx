import { fireEvent, render, screen } from '@testing-library/react';
import React from 'react';
import SearchActionIcon from './SearchActionIcon';
import { vi } from 'vitest';

describe('SearchActionIcon', () => {
  it('names and invokes the active search stop action', () => {
    const onStop = vi.fn();
    render(
      <SearchActionIcon
        onRemove={() => {}}
        onStop={onStop}
        search={{ searchText: 'ambient mix', state: 'InProgress' }}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Stop search: ambient mix' }));
    expect(onStop).toHaveBeenCalledOnce();
  });

  it('names and invokes the completed search remove action', () => {
    const onRemove = vi.fn();
    render(
      <SearchActionIcon
        onRemove={onRemove}
        onStop={() => {}}
        search={{ searchText: 'ambient mix', state: 'Completed' }}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Remove search: ambient mix' }));
    expect(onRemove).toHaveBeenCalledOnce();
  });
});
