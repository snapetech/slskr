import FileList from './FileList';
import { fireEvent, render, screen } from '@testing-library/react';
import React from 'react';

describe('FileList selection', () => {
  it('selects the inclusive range on shift-click', () => {
    const onSelectionChange = vi.fn();

    render(
      <FileList
        files={[
          { filename: 'C.flac', selected: false, size: 3 },
          { filename: 'A.flac', selected: false, size: 1 },
          { filename: 'B.flac', selected: false, size: 2 },
        ]}
        onSelectionChange={onSelectionChange}
      />,
    );

    const checkboxes = screen.getAllByRole('checkbox');
    fireEvent.click(checkboxes[1]);
    fireEvent.click(checkboxes[3], { shiftKey: true });

    expect(onSelectionChange.mock.calls.map(([file, checked]) => [
      file.filename,
      checked,
    ])).toEqual([
      ['A.flac', true],
      ['A.flac', true],
      ['B.flac', true],
      ['C.flac', true],
    ]);
  });
});
