import { useState, useCallback } from 'react';

type BrowserStorage = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;

function useBrowserStorage<T>(
  key: string,
  initialValue: T,
  storageName: 'localStorage' | 'sessionStorage',
): [T, (value: T) => void] {
  const [storedValue, setStoredValue] = useState<T>(() => {
    try {
      const storage: BrowserStorage | null =
        typeof window !== 'undefined' ? window[storageName] : null;
      const item = storage?.getItem(key) ?? null;
      return item ? JSON.parse(item) : initialValue;
    } catch (error) {
      console.error(`Error reading ${storageName} key "${key}":`, error);
      return initialValue;
    }
  });

  const setValue = useCallback((value: T) => {
    try {
      setStoredValue(value);
      if (typeof window !== 'undefined') {
        const storage: BrowserStorage = window[storageName];
        if (value === null || value === undefined) {
          storage.removeItem(key);
        } else {
          storage.setItem(key, JSON.stringify(value));
        }
      }
    } catch (error) {
      console.error(`Error writing to ${storageName} key "${key}":`, error);
    }
  }, [key, storageName]);

  return [storedValue, setValue];
}

/**
 * Custom hook for managing localStorage with React state.
 */
export function useLocalStorage<T>(key: string, initialValue: T): [T, (value: T) => void] {
  return useBrowserStorage(key, initialValue, 'localStorage');
}

/**
 * Custom hook for managing sessionStorage with React state.
 */
export function useSessionStorage<T>(key: string, initialValue: T): [T, (value: T) => void] {
  return useBrowserStorage(key, initialValue, 'sessionStorage');
}
