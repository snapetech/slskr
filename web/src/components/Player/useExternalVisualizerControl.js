import { useCallback, useEffect, useRef, useState } from 'react';
import * as externalVisualizer from '../../lib/externalVisualizer';
import { toDisplayError } from '../../lib/errors';

const getExternalVisualizerError = (error) => {
  return toDisplayError(error, 'External visualizer did not launch.');
};



const useExternalVisualizerControl = ({ integrationsOpen, mountedRef }) => {
  const externalStatusRequestIdRef = useRef(0);
  const externalLaunchRequestIdRef = useRef(0);
  const externalLaunchInFlightRef = useRef(false);
  const [externalVisualizerStatus, setExternalVisualizerStatus] = useState(null);
  const [externalVisualizerLoading, setExternalVisualizerLoading] = useState(false);
  const [externalVisualizerLaunching, setExternalVisualizerLaunching] = useState(false);
  const [externalVisualizerMessage, setExternalVisualizerMessage] = useState('');

  const refreshExternalVisualizerStatus = useCallback(() => {
    if (!mountedRef.current) return Promise.resolve(undefined);
    const requestId = ++externalStatusRequestIdRef.current;
    setExternalVisualizerLoading(true);
    setExternalVisualizerMessage('');

    return externalVisualizer.getExternalVisualizerStatus()
      .then((status) => {
        if (
          mountedRef.current &&
          externalStatusRequestIdRef.current === requestId
        ) {
          setExternalVisualizerStatus(status);
        }
        return status;
      })
      .catch(() => {
        if (
          mountedRef.current &&
          externalStatusRequestIdRef.current === requestId
        ) {
          setExternalVisualizerMessage('External visualizer status is unavailable.');
        }
      })
      .finally(() => {
        if (
          mountedRef.current &&
          externalStatusRequestIdRef.current === requestId
        ) {
          setExternalVisualizerLoading(false);
        }
      });
  }, [mountedRef]);

  const launchExternalVisualizer = useCallback(() => {
    if (!mountedRef.current || externalLaunchInFlightRef.current) return;
    externalLaunchInFlightRef.current = true;
    const requestId = ++externalLaunchRequestIdRef.current;
    setExternalVisualizerLaunching(true);
    setExternalVisualizerMessage('');

    externalVisualizer.launchExternalVisualizer()
      .then((result) => {
        if (
          !mountedRef.current ||
          externalLaunchRequestIdRef.current !== requestId
        ) {
          return;
        }
        const name =
          typeof result?.name === 'string'
            ? result.name
            : typeof externalVisualizerStatus?.name === 'string'
              ? externalVisualizerStatus.name
              : 'External visualizer';
        setExternalVisualizerMessage(
          result?.started
            ? name + ' launched.'
            : toDisplayError(result, 'External visualizer did not launch.'),
        );
      })
      .catch((error) => {
        if (
          mountedRef.current &&
          externalLaunchRequestIdRef.current === requestId
        ) {
          setExternalVisualizerMessage(getExternalVisualizerError(error));
        }
      })
      .finally(() => {
        externalLaunchInFlightRef.current = false;
        if (
          mountedRef.current &&
          externalLaunchRequestIdRef.current === requestId
        ) {
          setExternalVisualizerLaunching(false);
        }
      });
  }, [externalVisualizerStatus, mountedRef]);

  useEffect(() => {
    if (integrationsOpen) {
      void refreshExternalVisualizerStatus();
    } else {
      externalStatusRequestIdRef.current += 1;
      externalLaunchRequestIdRef.current += 1;
    }
  }, [integrationsOpen, refreshExternalVisualizerStatus]);
  return {
    externalVisualizerLaunching,
    externalVisualizerLoading,
    externalVisualizerMessage,
    externalVisualizerStatus,
    launchExternalVisualizer,
    refreshExternalVisualizerStatus,
  };
};

export default useExternalVisualizerControl;
