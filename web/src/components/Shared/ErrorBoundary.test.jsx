import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import ErrorBoundary from './ErrorBoundary';

const BrokenView = () => {
  throw new Error('broken view');
};

describe('ErrorBoundary', () => {
  it('renders a reloadable fallback for descendant render errors', () => {
    const reload = vi.fn();
    const originalLocation = window.location;
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { ...originalLocation, reload },
    });
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <BrokenView />
      </ErrorBoundary>,
    );

    expect(screen.getByRole('alert')).toHaveTextContent('Something went wrong');
    fireEvent.click(screen.getByRole('button', { name: 'Reload application' }));
    expect(reload).toHaveBeenCalledOnce();

    Object.defineProperty(window, 'location', {
      configurable: true,
      value: originalLocation,
    });
    consoleSpy.mockRestore();
  });
});
