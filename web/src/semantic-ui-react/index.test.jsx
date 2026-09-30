import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import { Icon, Menu } from './index';

describe('Menu.Item accessibility', () => {
  it('supports keyboard activation for clickable div items', () => {
    const onClick = vi.fn();
    render(<Menu.Item onClick={onClick}>Settings</Menu.Item>);

    const item = screen.getByRole('button', { name: 'Settings' });
    expect(item).toHaveAttribute('tabindex', '0');
    fireEvent.keyDown(item, { key: 'Enter' });
    fireEvent.keyDown(item, { key: ' ' });
    expect(onClick).toHaveBeenCalledTimes(2);
  });

  it('exposes clickable icons as keyboard-accessible controls', () => {
    const onClick = vi.fn();
    render(<Icon name="close" onClick={onClick} />);

    const icon = screen.getByRole('button');
    fireEvent.keyDown(icon, { key: 'Enter' });
    expect(onClick).toHaveBeenCalledOnce();
  });
});
