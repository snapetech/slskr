import './Transfers.css';
import * as autoReplaceLibrary from '../../lib/autoReplace';
import { toDisplayError } from '../../lib/errors';
import * as transfersLibrary from '../../lib/transfers';
import { usePolling } from '../../lib/usePolling';
import { LoaderSegment, PlaceholderSegment } from '../Shared';
import TransferGroup from './TransferGroup';
import TransfersHeader from './TransfersHeader';
import React, { useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';

const AUTO_REPLACE_THRESHOLD = 0; // 0% = exact match only (configurable on backend)

const getErrorMessage = (error) => toDisplayError(error);

const summarizeBulkFailures = ({ action, failures }) => {
  if (failures.length === 0) {
    return;
  }

  const [firstFailure] = failures;
  toast.error(
    failures.length === 1
      ? `Failed to ${action} ${firstFailure.label}: ${firstFailure.message}`
      : `Failed to ${action} ${failures.length} transfer(s). First error: ${firstFailure.label}: ${firstFailure.message}`,
  );
};

const getTransferKey = ({ file, suffix = '' }) => {
  return `${file.username}:${file.id}${suffix ? `:${suffix}` : ''}`;
};

const OPTIMISTIC_HIDE_MS = 15_000;
const QUEUE_POSITION_REFRESH_MS = 30_000;
const MAX_QUEUE_POSITION_LOOKUPS_PER_FETCH = 5;

const asRecords = (value) =>
  (Array.isArray(value) ? value : []).filter(
    (record) => record && typeof record === 'object' && !Array.isArray(record),
  );

const toText = (value, fallback = '') => {
  if (typeof value === 'string') return value;
  if (typeof value === 'number') return String(value);
  return fallback;
};

const toNonNegativeNumber = (value, fallback = 0) => {
  const number = Number(value);
  return Number.isFinite(number) && number >= 0 ? number : fallback;
};

const transferDirectory = (filename) => {
  const separator = Math.max(filename.lastIndexOf('/'), filename.lastIndexOf('\\'));
  return separator >= 0 ? filename.slice(0, separator) : '';
};

const normalizeTransferFile = (file, username) => {
  if (!file || typeof file !== 'object' || Array.isArray(file)) return null;

  const filename = toText(file.filename ?? file.name);
  const id = file.id === undefined || file.id === null ? '' : String(file.id);
  const direction = toText(file.direction);
  const directionName = direction.toLowerCase();

  if (!filename || !id || !['download', 'upload'].includes(directionName)) {
    return null;
  }

  const normalizedUsername = toText(file.username ?? file.user, username) || 'Unknown';
  const percentComplete = Math.min(
    100,
    toNonNegativeNumber(file.percentComplete, 0),
  );

  return {
    ...file,
    averageSpeed: toNonNegativeNumber(file.averageSpeed, 0),
    bytesTransferred: toNonNegativeNumber(file.bytesTransferred, 0),
    direction: directionName === 'download' ? 'Download' : 'Upload',
    filename,
    id,
    percentComplete,
    size: toNonNegativeNumber(file.size, 0),
    state: toText(file.state),
    username: normalizedUsername,
  };
};

const normalizeTransferGroups = (users, expectedDirection) =>
  asRecords(users)
    .map((user) => {
      const username = toText(user.username ?? user.user, 'Unknown') || 'Unknown';
      const directories = asRecords(user.directories)
        .map((directory) => ({
          ...directory,
          directory: toText(directory.directory ?? directory.name),
          files: asRecords(directory.files)
            .map((file) => normalizeTransferFile(file, username))
            .filter(
              (file) =>
                file &&
                (!expectedDirection ||
                  file.direction.toLowerCase() === expectedDirection),
            )
            .filter(Boolean),
        }))
        .filter((directory) => directory.files.length > 0);

      return { ...user, directories, username };
    })
    .filter((user) => user.directories.length > 0);

const groupFlatTransfers = (records) => {
  const groups = new Map();

  asRecords(records).forEach((record) => {
    const username = toText(record.username ?? record.user, 'Unknown') || 'Unknown';
    const filename = toText(record.filename ?? record.name);
    const directory =
      toText(record.directory ?? record.directoryName) ||
      transferDirectory(filename);
    let user = groups.get(username);

    if (!user) {
      user = { directories: [], username };
      groups.set(username, user);
    }

    let directoryGroup = user.directories.find(
      (candidate) => candidate.directory === directory,
    );
    if (!directoryGroup) {
      directoryGroup = { directory, files: [] };
      user.directories.push(directoryGroup);
    }

    directoryGroup.files.push({ ...record, filename });
  });

  return Array.from(groups.values());
};

const Transfers = ({ runtimeProfile, direction, server }) => {
  const testId = direction === 'download' ? 'downloads-root' : 'uploads-root';
  const [connecting, setConnecting] = useState(true);
  const [transfers, setTransfers] = useState([]);

  const [retryingSingle, setRetryingSingle] = useState(false);
  const [cancellingSingle, setCancellingSingle] = useState(false);
  const [removingSingle, setRemovingSingle] = useState(false);
  const [bulkCounts, setBulkCounts] = useState({ retry: 0, cancel: 0, remove: 0 });

  const [autoReplaceEnabled, setAutoReplaceEnabled] = useState(false);
  const [acceleratedEnabled, setAcceleratedEnabled] = useState(false);
  const autoReplaceThreshold = AUTO_REPLACE_THRESHOLD;

  const bulkQueueRef = useRef([]);
  const queuedBulkKeysRef = useRef(new Set());
  const bulkQueueRunningRef = useRef(false);
  const hiddenTransfersRef = useRef(new Map());
  const latestFetchIdRef = useRef(0);
  const lastQueuePositionBatchAtRef = useRef(0);
  const queuePositionCacheRef = useRef(new Map());
  const queuePositionRequestsRef = useRef(new Set());
  const mountedRef = useRef(false);
  const modeRequestIdsRef = useRef({ autoReplace: 0, accelerated: 0 });
  const modeInFlightRef = useRef({ autoReplace: false, accelerated: false });
  const [autoReplaceChanging, setAutoReplaceChanging] = useState(false);
  const [acceleratedChanging, setAcceleratedChanging] = useState(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      latestFetchIdRef.current += 1;
      modeRequestIdsRef.current.autoReplace += 1;
      modeRequestIdsRef.current.accelerated += 1;
    };
  }, []);

  const retrying = retryingSingle || bulkCounts.retry > 0;
  const cancelling = cancellingSingle || bulkCounts.cancel > 0;
  const removing = removingSingle || bulkCounts.remove > 0;

  const changeBulkCount = (action, delta) => {
    if (!mountedRef.current) return;
    setBulkCounts((previousCounts) => ({
      ...previousCounts,
      [action]: Math.max(0, previousCounts[action] + delta),
    }));
  };

  const isOptimisticallyHidden = (file, now = Date.now()) => {
    const entry = hiddenTransfersRef.current.get(getTransferKey({ file }));

    if (!entry) {
      return false;
    }

    if (entry.until <= now) {
      hiddenTransfersRef.current.delete(getTransferKey({ file }));
      return false;
    }

    return entry.matches(file);
  };

  const filterHiddenTransfers = (users) => {
    const now = Date.now();

    return normalizeTransferGroups(users, direction)
      .map((user) => ({
        ...user,
        directories: user.directories
          .map((directory) => ({
            ...directory,
            files: directory.files.filter(
              (file) => !isOptimisticallyHidden(file, now),
            ),
          }))
          .filter((directory) => directory.files.length > 0),
      }))
      .filter((user) => user.directories.length > 0);
  };

  const hideTransfers = (files, matches = () => true) => {
    if (!mountedRef.current) return;
    const until = Date.now() + OPTIMISTIC_HIDE_MS;

    files.forEach((file) => {
      hiddenTransfersRef.current.set(getTransferKey({ file }), {
        matches,
        until,
      });
    });

    setTransfers((previousTransfers) =>
      filterHiddenTransfers(previousTransfers),
    );
  };

  const runBulkQueue = async () => {
    if (bulkQueueRunningRef.current) {
      return;
    }

    bulkQueueRunningRef.current = true;

    while (bulkQueueRef.current.length > 0 && mountedRef.current) {
      const queuedOperation = bulkQueueRef.current.shift();

      try {
        await queuedOperation.run();
      } catch (error) {
        queuedOperation.batch.failures.push({
          label: queuedOperation.label,
          message: getErrorMessage(error),
        });
      } finally {
        queuedBulkKeysRef.current.delete(queuedOperation.key);
        changeBulkCount(queuedOperation.action, -1);
        queuedOperation.batch.remaining -= 1;

        if (mountedRef.current && queuedOperation.batch.remaining === 0) {
          summarizeBulkFailures({
            action: queuedOperation.batch.action,
            failures: queuedOperation.batch.failures,
          });
        }
      }
    }

    bulkQueueRunningRef.current = false;
  };

  const enqueueBulkOperations = ({ action, operations }) => {
    const batch = {
      action,
      failures: [],
      remaining: 0,
    };

    let enqueuedCount = 0;

    operations.forEach((operation) => {
      if (queuedBulkKeysRef.current.has(operation.key)) {
        return;
      }

      queuedBulkKeysRef.current.add(operation.key);
      bulkQueueRef.current.push({
        ...operation,
        action,
        batch,
      });
      batch.remaining += 1;
      enqueuedCount += 1;
    });

    if (enqueuedCount === 0) {
      return;
    }

    changeBulkCount(action, enqueuedCount);
    runBulkQueue();
  };

  const refreshQueuePositions = async (users, fetchId) => {
    if (direction !== 'download') {
      return users;
    }

    const now = Date.now();
    const queuedDownloads = users
      .flatMap((user) => user.directories.flatMap((dir) => dir.files))
      .filter((file) => file.state && file.state.includes('Queued'));

    const applyQueuePositionCache = (groups) =>
      groups.map((user) => ({
        ...user,
        directories: user.directories.map((directory) => ({
          ...directory,
          files: directory.files.map((file) => {
            const cached = queuePositionCacheRef.current.get(
              getTransferKey({ file }),
            );
            return cached && cached.placeInQueue !== null
              ? { ...file, placeInQueue: cached.placeInQueue }
              : file;
          }),
        })),
      }));

    if (
      lastQueuePositionBatchAtRef.current > 0 &&
      now - lastQueuePositionBatchAtRef.current < QUEUE_POSITION_REFRESH_MS
    ) {
      return applyQueuePositionCache(users);
    }

    const dueDownloads = queuedDownloads
      .filter((file) => {
        const key = getTransferKey({ file });
        const cached = queuePositionCacheRef.current.get(key);

        return (
          !queuePositionRequestsRef.current.has(key) &&
          (!cached || now - cached.updatedAt >= QUEUE_POSITION_REFRESH_MS)
        );
      })
      .slice(0, MAX_QUEUE_POSITION_LOOKUPS_PER_FETCH);

    if (dueDownloads.length === 0) {
      return applyQueuePositionCache(users);
    }

    lastQueuePositionBatchAtRef.current = now;

    const queuePositionPromises = dueDownloads.map(async (file) => {
      const key = getTransferKey({ file });
      queuePositionRequestsRef.current.add(key);

      try {
        const queueResponse = await transfersLibrary.getPlaceInQueue({
          id: file.id,
          username: file.username,
        });

        const placeInQueue = toNonNegativeNumber(queueResponse?.data, null);
        if (
          mountedRef.current &&
          fetchId === latestFetchIdRef.current &&
          placeInQueue !== null
        ) {
          queuePositionCacheRef.current.set(key, {
            placeInQueue,
            updatedAt: Date.now(),
          });
        }
      } catch (error) {
        console.debug(
          'Failed to fetch queue position for',
          file.filename,
          error,
        );
      } finally {
        queuePositionRequestsRef.current.delete(key);
      }
    });

    await Promise.allSettled(queuePositionPromises);
    return applyQueuePositionCache(users);
  };

  const fetch = async () => {
    if (!mountedRef.current) return;
    const fetchId = latestFetchIdRef.current + 1;
    latestFetchIdRef.current = fetchId;

    try {
      const response = normalizeTransferGroups(
        runtimeProfile === 'native'
          ? groupFlatTransfers((await transfersLibrary.getChanges()).transfers)
          : await transfersLibrary.getAll({ direction }),
        direction,
      );

      const responseWithQueuePositions = await refreshQueuePositions(
        response,
        fetchId,
      );

      if (
        mountedRef.current &&
        fetchId === latestFetchIdRef.current
      ) {
        setTransfers(filterHiddenTransfers(responseWithQueuePositions));
      }
    } catch (error) {
      console.error(error);
      if (mountedRef.current) {
        toast.error(getErrorMessage(error));
      }
    }
  };

  useEffect(() => {
    setConnecting(true);
  }, [runtimeProfile, direction]);

  usePolling(
    async () => {
      await fetch();
      if (mountedRef.current) {
        setConnecting(false);
      }
    },
    1_000,
    { resetKey: `${runtimeProfile}:${direction}` },
  );

  const retry = async ({
    file,
    suppressErrorToast = false,
    suppressStateChange = false,
  }) => {
    const { filename, size, username } = file;

    try {
      if (!suppressStateChange) {
        if (mountedRef.current) setRetryingSingle(true);
      }

      await transfersLibrary.download({
        files: [{ filename, size }],
        username,
      });
    } catch (error) {
      console.error(error);
      if (!suppressErrorToast && mountedRef.current) {
        toast.error(getErrorMessage(error));
      }

      throw error;
    } finally {
      if (!suppressStateChange && mountedRef.current) {
        setRetryingSingle(false);
      }
    }
  };

  const retryAll = (transfersToRetry) => {
    enqueueBulkOperations({
      action: 'retry',
      operations: transfersToRetry.map((file) => ({
        key: `retry:${getTransferKey({ file })}`,
        label: `${file.username}/${file.filename}`,
        run: async () => {
          await retry({
            file,
            suppressErrorToast: true,
            suppressStateChange: true,
          });
          hideTransfers([file], (candidate) =>
            transfersLibrary.isStateRetryable(candidate.state),
          );
        },
      })),
    });
  };

  const cancel = async ({
    file,
    suppressErrorToast = false,
    suppressStateChange = false,
  }) => {
    const { id, username } = file;

    try {
      if (!suppressStateChange) {
        if (mountedRef.current) setCancellingSingle(true);
      }

      await transfersLibrary.cancel({ direction, id, username });
    } catch (error) {
      console.error(error);
      if (!suppressErrorToast && mountedRef.current) {
        toast.error(getErrorMessage(error));
      }

      throw error;
    } finally {
      if (!suppressStateChange && mountedRef.current) {
        setCancellingSingle(false);
      }
    }
  };

  const cancelAll = (transfersToCancel) => {
    enqueueBulkOperations({
      action: 'cancel',
      operations: transfersToCancel.map((file) => ({
        key: `cancel:${getTransferKey({ file })}`,
        label: `${file.username}/${file.filename}`,
        run: () =>
          cancel({
            file,
            suppressErrorToast: true,
            suppressStateChange: true,
          }),
      })),
    });
  };

  const remove = async ({
    deleteFile = false,
    file,
    suppressErrorToast = false,
    suppressStateChange = false,
  }) => {
    const { id, username } = file;

    try {
      if (!suppressStateChange) {
        if (mountedRef.current) setRemovingSingle(true);
      }

      await transfersLibrary.cancel({
        deleteFile,
        direction,
        id,
        remove: true,
        username,
      });
    } catch (error) {
      console.error(error);
      if (!suppressErrorToast && mountedRef.current) {
        toast.error(getErrorMessage(error));
      }

      throw error;
    } finally {
      if (!suppressStateChange && mountedRef.current) {
        setRemovingSingle(false);
      }
    }
  };

  const removeAll = (
    transfersToRemove,
    deleteFile = false,
    { useBulkClear = false } = {},
  ) => {
    if (useBulkClear && !deleteFile) {
      enqueueBulkOperations({
        action: 'remove',
        operations: [
          {
            key: `remove:clear-completed:${direction}`,
            label: `all completed ${direction}s`,
            run: async () => {
              await transfersLibrary.clearCompleted({ direction });
              hideTransfers(transfersToRemove);
            },
          },
        ],
      });
      return;
    }

    enqueueBulkOperations({
      action: 'remove',
      operations: transfersToRemove.map((file) => ({
        key: `remove:${getTransferKey({ file, suffix: deleteFile ? 'delete' : 'keep' })}`,
        label: `${file.username}/${file.filename}`,
        run: async () => {
          await remove({
            deleteFile,
            file,
            suppressErrorToast: true,
            suppressStateChange: true,
          });
          hideTransfers([file]);
        },
      })),
    });
  };

  useEffect(() => {
    let cancelled = false;
    const fetchDownloadModeStatus = async () => {
      if (direction !== 'download' || runtimeProfile === 'legacy') {
        return;
      }

      try {
        const [autoReplaceStatus, acceleratedStatus] = await Promise.all([
          autoReplaceLibrary.getAutoReplaceStatus(),
          transfersLibrary.getAcceleratedMode(),
        ]);
        if (!cancelled && mountedRef.current) {
          setAutoReplaceEnabled(autoReplaceStatus?.enabled ?? false);
          setAcceleratedEnabled(acceleratedStatus?.enabled ?? false);
        }
      } catch (error) {
        if (!cancelled) {
          console.error('Failed to fetch download mode status:', error);
        }
      }
    };

    void fetchDownloadModeStatus();
    return () => {
      cancelled = true;
      modeRequestIdsRef.current.autoReplace += 1;
      modeRequestIdsRef.current.accelerated += 1;
    };
  }, [runtimeProfile, direction]);

  const handleAutoReplaceChange = async (enabled) => {
    if (
      !mountedRef.current ||
      modeInFlightRef.current.autoReplace
    ) {
      return;
    }
    modeInFlightRef.current.autoReplace = true;
    setAutoReplaceChanging(true);
    const requestId = ++modeRequestIdsRef.current.autoReplace;
    try {
      if (enabled) {
        await autoReplaceLibrary.enableAutoReplace();
        if (!mountedRef.current || requestId !== modeRequestIdsRef.current.autoReplace) return;
        setAutoReplaceEnabled(true);
        toast.info(
          'Auto-replace enabled. Backend will check for stuck downloads periodically.',
        );
      } else {
        await autoReplaceLibrary.disableAutoReplace();
        if (!mountedRef.current || requestId !== modeRequestIdsRef.current.autoReplace) return;
        setAutoReplaceEnabled(false);
        toast.info('Auto-replace disabled');
      }
    } catch (error) {
      console.error('Failed to toggle auto-replace:', error);
      if (mountedRef.current && requestId === modeRequestIdsRef.current.autoReplace) {
        toast.error(`Failed to toggle auto-replace: ${getErrorMessage(error)}`);
      }
    } finally {
      modeInFlightRef.current.autoReplace = false;
      if (mountedRef.current && requestId === modeRequestIdsRef.current.autoReplace) {
        setAutoReplaceChanging(false);
      }
    }
  };

  const handleAcceleratedChange = async (enabled) => {
    if (
      !mountedRef.current ||
      modeInFlightRef.current.accelerated
    ) {
      return;
    }
    modeInFlightRef.current.accelerated = true;
    setAcceleratedChanging(true);
    const requestId = ++modeRequestIdsRef.current.accelerated;
    try {
      const status = await transfersLibrary.setAcceleratedMode({ enabled });
      if (!mountedRef.current || requestId !== modeRequestIdsRef.current.accelerated) return;
      setAcceleratedEnabled(status?.enabled ?? enabled);
      toast.info(
        enabled
          ? 'Accelerated mode enabled. Slow or stalled downloads can use verified alternate sources.'
          : 'Accelerated mode disabled',
      );
    } catch (error) {
      console.error('Failed to toggle accelerated mode:', error);
      if (mountedRef.current && requestId === modeRequestIdsRef.current.accelerated) {
        toast.error(`Failed to toggle accelerated mode: ${getErrorMessage(error)}`);
      }
    } finally {
      modeInFlightRef.current.accelerated = false;
      if (mountedRef.current && requestId === modeRequestIdsRef.current.accelerated) {
        setAcceleratedChanging(false);
      }
    }
  };

  if (connecting) {
    return <LoaderSegment />;
  }

  return (
    <div data-testid={testId}>
      <TransfersHeader
        acceleratedEnabled={acceleratedEnabled}
        acceleratedChanging={acceleratedChanging}
        autoReplaceEnabled={autoReplaceEnabled}
        autoReplaceThreshold={autoReplaceThreshold}
        autoReplaceChanging={autoReplaceChanging}
        cancelling={cancelling}
        direction={direction}
        onAutoReplaceChange={handleAutoReplaceChange}
        onAcceleratedChange={handleAcceleratedChange}
        onCancelAll={cancelAll}
        onRemoveAll={removeAll}
        onRetryAll={retryAll}
        removing={removing}
        retrying={retrying}
        server={server}
        transfers={transfers}
      />
      {transfers.length === 0 ? (
        <PlaceholderSegment
          caption={`No ${direction}s to display`}
          icon={direction}
        />
      ) : (
        transfers.map((user) => (
          <TransferGroup
            cancel={cancel}
            cancelAll={cancelAll}
            direction={direction}
            key={user.username}
            remove={remove}
            removeAll={removeAll}
            retry={retry}
            retryAll={retryAll}
            user={user}
          />
        ))
      )}
    </div>
  );
};

export default Transfers;
