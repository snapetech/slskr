import { renderHook, act } from '@testing-library/react';
import { describe, expect, it, beforeEach } from 'vitest';
import { useLocalStorage, useSessionStorage } from './useLocalStorage';

describe('useLocalStorage', () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  it('persists state changes to localStorage', () => {
    const { result } = renderHook(() => useLocalStorage<string | null>('apiKey', null));

    act(() => {
      result.current[1]('test-token');
    });

    expect(result.current[0]).toBe('test-token');
    expect(window.localStorage.getItem('apiKey')).toBe('"test-token"');
  });

  it('restores persisted falsy JSON values instead of treating them as missing', () => {
    window.localStorage.setItem('autoRefresh', 'false');

    const { result } = renderHook(() => useLocalStorage('autoRefresh', true));

    expect(result.current[0]).toBe(false);
  });

  it('persists sensitive state changes to sessionStorage only', () => {
    const { result } = renderHook(() => useSessionStorage<string | null>('apiKey', null));

    act(() => {
      result.current[1]('test-token');
    });

    expect(result.current[0]).toBe('test-token');
    expect(window.sessionStorage.getItem('apiKey')).toBe('"test-token"');
    expect(window.localStorage.getItem('apiKey')).toBeNull();
  });

  it('removes storage entries when set to null', () => {
    const { result } = renderHook(() => useSessionStorage<string | null>('apiKey', 'initial'));

    act(() => {
      result.current[1](null);
    });

    expect(result.current[0]).toBeNull();
    expect(window.sessionStorage.getItem('apiKey')).toBeNull();
  });
});
