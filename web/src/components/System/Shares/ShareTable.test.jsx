import { fireEvent, render, screen } from '@testing-library/react';
import React from 'react';
import ShareTable from './ShareTable';
import { vi } from 'vitest';

describe('ShareTable', () => {
  it('exposes a named directory browse action and opens that directory', () => {
    const onClick = vi.fn();
    const share = {
      alias: 'open-fixtures',
      directories: 1,
      files: 2,
      host: 'local',
      localPath: '/srv/media/open-fixtures',
      remotePath: 'open-fixtures',
    };

    render(<ShareTable onClick={onClick} shares={[share]} />);

    fireEvent.click(screen.getByRole('button', {
      name: 'Browse shared directory /srv/media/open-fixtures',
    }));

    expect(onClick).toHaveBeenCalledWith(share);
  });

  it('keeps malformed paths visible without exposing an empty action', () => {
    render(<ShareTable onClick={() => {}} shares={[{ host: 'local' }]} />);

    expect(screen.getByRole('button', {
      name: 'Shared directory path unavailable',
    })).toBeDisabled();
    expect(screen.getByText('Path unavailable')).toBeInTheDocument();
  });
});
