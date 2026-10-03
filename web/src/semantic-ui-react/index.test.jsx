import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import {
  Checkbox,
  Dropdown,
  Form,
  Icon,
  Input,
  Menu,
  Progress,
} from './index';

describe('Dropdown accessibility', () => {
  it('uses a controlled combobox and selects enabled options with the keyboard', () => {
    const onChange = vi.fn();
    render(
      <Dropdown
        aria-label="Acquisition profile"
        onChange={onChange}
        options={[
          { text: 'Fast', value: 'fast' },
          { disabled: true, text: 'Unavailable', value: 'unavailable' },
          { text: 'Thorough', value: 'thorough' },
        ]}
        value="fast"
      />,
    );

    const combobox = screen.getByRole('combobox', { name: 'Acquisition profile' });
    const listbox = screen.getByRole('listbox', { hidden: true });
    expect(listbox).toHaveAttribute('id', combobox.getAttribute('aria-controls'));
    expect(screen.getAllByRole('option', { hidden: true })).toHaveLength(3);

    fireEvent.keyDown(combobox, { key: 'ArrowDown' });
    expect(combobox).toHaveAttribute('aria-expanded', 'true');
    expect(document.getElementById(combobox.getAttribute('aria-activedescendant')))
      .toHaveTextContent('Fast');

    fireEvent.keyDown(combobox, { key: 'ArrowDown' });
    expect(document.getElementById(combobox.getAttribute('aria-activedescendant')))
      .toHaveTextContent('Thorough');
    fireEvent.keyDown(combobox, { key: 'Enter' });

    expect(onChange).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({ value: 'thorough' }));
    expect(combobox).toHaveAttribute('aria-expanded', 'false');
  });

  it('inherits the visible label from its form field', () => {
    render(
      <Form>
        <Form.Field>
          <label>Seed scope</label>
          <Dropdown
            options={[{ text: 'Artist', value: 'artist' }]}
            value="artist"
          />
        </Form.Field>
      </Form>,
    );

    expect(screen.getByRole('combobox', { name: 'Seed scope' })).toBeInTheDocument();
  });
});

describe('form control labels', () => {
  it('associates checkbox labels with the input', () => {
    render(<Checkbox label="Accept terms" />);

    expect(screen.getByRole('checkbox', { name: 'Accept terms' })).toBeInTheDocument();
  });

  it('uses a Form.Field label to name nested input controls', () => {
    render(
      <Form>
        <Form.Field>
          <label>Server port</label>
          <Input type="number" value={2242} />
        </Form.Field>
      </Form>,
    );

    expect(screen.getByRole('spinbutton', { name: 'Server port' })).toBeInTheDocument();
  });

  it('preserves text navigation keys in searchable dropdowns', () => {
    render(
      <Dropdown
        options={[{ text: 'Artist', value: 'artist' }]}
        search
      />,
    );

    const combobox = screen.getByRole('combobox');
    fireEvent.click(combobox);
    expect(fireEvent.keyDown(combobox, { key: 'Home' })).toBe(true);
    expect(fireEvent.keyDown(combobox, { key: 'End' })).toBe(true);
  });

  it('names a progress bar from its visible description', () => {
    render(<Progress percent={50}>File transfer progress</Progress>);

    expect(screen.getByRole('progressbar', { name: 'File transfer progress' }))
      .toBeInTheDocument();
  });
});

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
