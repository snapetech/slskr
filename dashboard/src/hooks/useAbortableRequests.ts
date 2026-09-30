import { useCallback, useEffect, useRef } from 'react';

/** Tracks component-owned requests and aborts them when the component leaves. */
export function useAbortableRequests() {
  const controllersRef = useRef(new Set<AbortController>());

  const createController = useCallback(() => {
    const controller = new AbortController();
    controllersRef.current.add(controller);
    return controller;
  }, []);

  const releaseController = useCallback((controller: AbortController) => {
    controllersRef.current.delete(controller);
  }, []);

  useEffect(() => () => {
    for (const controller of controllersRef.current) {
      controller.abort();
    }
    controllersRef.current.clear();
  }, []);

  return { createController, releaseController };
}
