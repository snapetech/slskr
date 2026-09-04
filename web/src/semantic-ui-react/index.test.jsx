import { fireEvent, render, screen } from '@testing-library/react';
import React from 'react';
import { Button, Input } from './index';

describe('Semantic UI compatibility controls', () => {
  it('renders object shorthand labels on inputs and buttons', () => {
    render(
      <>
        <Input
          aria-label="Local port"
          label={{ basic: true, content: 'localhost:' }}
          labelPosition="left"
          onChange={() => undefined}
          value="8080"
        />
        <Button
          label={{ as: 'a', content: '2 files' }}
          labelPosition="right"
        >
          Download
        </Button>
      </>,
    );

    expect(screen.getByText('localhost:')).toHaveClass('ui', 'basic', 'label');
    expect(screen.getByText('2 files')).toHaveClass('ui', 'label');
    expect(screen.getByText('2 files').tagName).toBe('A');
  });

  it('renders icon shorthand inside labels without leaking objects to React', () => {
    render(
      <Input
        aria-label="Filter"
        label={{ content: 'Filter', icon: 'filter' }}
        onChange={() => undefined}
        value=""
      />,
    );

    expect(screen.getByText('Filter')).toBeInTheDocument();
    expect(document.querySelector('i.filter.icon')).toBeInTheDocument();
  });

  it('keeps input change data compatible with Semantic UI callers', () => {
    const onChange = vi.fn();
    render(<Input aria-label="Filter" onChange={onChange} value="" />);

    fireEvent.change(screen.getByRole('textbox', { name: 'Filter' }), {
      target: { value: 'cover' },
    });

    expect(onChange).toHaveBeenCalledWith(
      expect.any(Object),
      expect.objectContaining({ value: 'cover' }),
    );
  });
});
