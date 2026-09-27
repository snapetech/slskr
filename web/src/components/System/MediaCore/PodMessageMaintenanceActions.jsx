import * as mediacore from '../../../lib/mediacore';
import { toDisplayError } from '../../../lib/errors';
import Button from './MediaCoreButton';
import React, { useEffect, useRef } from 'react';
import { toast } from 'react-toastify';

const useMountedState = (mountedRef, initialValue) => {
  const [value, setValue] = React.useState(initialValue);
  const setMountedValue = React.useCallback(
    (nextValue) => {
      if (mountedRef.current) {
        setValue(nextValue);
      }
    },
    [mountedRef],
  );

  return [value, setMountedValue];
};

const PodMessageMaintenanceActions = ({ size = 'tiny' }) => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [rebuildIndexLoading, setRebuildIndexLoading] = useMountedState(
    mountedRef,
    false,
  );
  const [vacuumLoading, setVacuumLoading] = useMountedState(mountedRef, false);

  const handleRebuildSearchIndex = async () => {
    try {
      setRebuildIndexLoading(true);
      const result = await mediacore.rebuildSearchIndex();
      toast.success(
        result
          ? 'Search index rebuilt successfully'
          : 'Search index rebuild failed',
      );
    } catch (error_) {
      toast.error(`Failed to rebuild search index: ${toDisplayError(error_)}`);
    } finally {
      setRebuildIndexLoading(false);
    }
  };

  const handleVacuumDatabase = async () => {
    try {
      setVacuumLoading(true);
      const result = await mediacore.vacuumDatabase();
      toast.success(
        result
          ? 'Database vacuum completed successfully'
          : 'Database vacuum failed',
      );
    } catch (error_) {
      toast.error(`Failed to vacuum database: ${toDisplayError(error_)}`);
    } finally {
      setVacuumLoading(false);
    }
  };

  return (
    <>
      <Button
        color="blue"
        loading={rebuildIndexLoading}
        onClick={handleRebuildSearchIndex}
        size={size}
      >
        Rebuild Search Index
      </Button>
      <Button
        color="orange"
        loading={vacuumLoading}
        onClick={handleVacuumDatabase}
        size={size}
      >
        Vacuum Database
      </Button>
    </>
  );
};

export default React.memo(PodMessageMaintenanceActions);
