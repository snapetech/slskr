import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { axe } from 'jest-axe';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ApiProvider } from '../context/ApiContext';
import Sidebar from './Sidebar';

describe('Sidebar', () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  afterEach(() => {
    cleanup();
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

  it('marks the active navigation page for assistive technology', () => {
    render(
      <ApiProvider>
        <MemoryRouter initialEntries={['/monitoring']}>
          <Sidebar />
        </MemoryRouter>
      </ApiProvider>,
    );

    expect(screen.getByRole('link', { name: 'Monitoring' }).getAttribute('aria-current'))
      .toBe('page');
    expect(screen.getByRole('link', { name: 'Dashboard' }).getAttribute('aria-current'))
      .toBeNull();
  });

  it('has no axe violations in the navigation shell', async () => {
    const { container } = render(
      <ApiProvider>
        <MemoryRouter initialEntries={['/monitoring']}>
          <Sidebar />
        </MemoryRouter>
      </ApiProvider>,
    );

    const results = await axe(container);
    expect(results.violations).toHaveLength(0);
  });
});
