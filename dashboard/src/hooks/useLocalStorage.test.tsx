import { renderHook, act } from '@testing-library/react';
import { describe, expect, it, beforeEach } from 'vitest';
import { useLocalStorage } from './useLocalStorage';

describe('useLocalStorage', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('persists state changes to localStorage', () => {
    const { result } = renderHook(() => useLocalStorage<string | null>('apiKey', null));

    act(() => {
      result.current[1]('test-token');
    });

    expect(result.current[0]).toBe('test-token');
    expect(window.localStorage.getItem('apiKey')).toBe('"test-token"');
  });
});
