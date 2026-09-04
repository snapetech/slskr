import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it } from 'vitest';
import { ApiProvider } from '../context/ApiContext';
import Sidebar from './Sidebar';

describe('Sidebar', () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  it('clears the session API key when logging out', () => {
    window.sessionStorage.setItem('apiKey', JSON.stringify('session-token'));

    render(
      <ApiProvider>
        <MemoryRouter>
          <Sidebar />
        </MemoryRouter>
      </ApiProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: /logout/i }));

    expect(window.sessionStorage.getItem('apiKey')).toBeNull();
  });
});
