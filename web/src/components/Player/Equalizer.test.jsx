import '@testing-library/jest-dom';
import Equalizer from './Equalizer';
import React from 'react';
import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./audioGraph', () => ({
  bands: [60, 170, 310, 600, 1_000, 3_000, 6_000, 12_000, 14_000, 16_000],
  setEqGains: vi.fn(),
}));

const storageKey = 'slskr.player.equalizer';

describe('Equalizer', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('clamps malformed persisted gains before they reach the player', () => {
    localStorage.setItem(
      storageKey,
      JSON.stringify({
        enabled: true,
        gains: [99, -99, 'Infinity', 'not-a-number', 3],
        preset: 'x'.repeat(1_000),
      }),
    );

    render(<Equalizer audioElement={null} />);

    const sliders = screen.getAllByRole('slider');
    expect(sliders.map((slider) => slider.value)).toEqual([
      '12',
      '-12',
      '0',
      '0',
      '3',
      '0',
      '0',
      '0',
      '0',
      '0',
    ]);
  });

  it('ignores oversized persisted state', () => {
    localStorage.setItem(storageKey, 'x'.repeat(16_385));

    render(<Equalizer audioElement={null} />);

    expect(screen.getAllByRole('slider').every((slider) => slider.value === '0')).toBe(true);
    expect(screen.getByTestId('player-eq-toggle')).toHaveAttribute(
      'aria-label',
      'Enable equalizer',
    );
  });
});
