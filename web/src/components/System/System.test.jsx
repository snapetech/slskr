import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import System from './System';

vi.mock('./Info', () => ({
  default: () => <div>Info pane loaded</div>,
}));

vi.mock('./Options', () => ({
  default: () => <div>Options pane loaded</div>,
}));

describe('System pane loading', () => {
  it('loads the selected controller pane when its route becomes active', async () => {
    render(
      <MemoryRouter initialEntries={['/system/info']}>
        <Routes>
          <Route
            element={<System runtimeProfile="legacy" />}
            path="/system/:tab"
          />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText('Info pane loaded')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Options', { exact: true }));

    expect(await screen.findByText('Options pane loaded')).toBeInTheDocument();
    expect(screen.queryByText('Info pane loaded')).not.toBeInTheDocument();
  });
});
