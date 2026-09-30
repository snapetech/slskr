import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorBoundary } from './ErrorBoundary';

function BrokenChild(): never {
  throw new Error('render failed');
}

describe('ErrorBoundary', () => {
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  it('keeps a descendant render failure inside a reloadable fallback', () => {
    render(
      <ErrorBoundary>
        <BrokenChild />
      </ErrorBoundary>,
    );

    expect(screen.getByText('Something went wrong')).toBeTruthy();
    expect(screen.getByText('render failed')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Reload Page' })).toBeTruthy();
  });
});
