import * as sharesLibrary from '../../../lib/shares';
import { toDisplayError } from '../../../lib/errors';
import { LoaderSegment, ShrinkableButton, Switch } from '../../Shared';
import ContentsModal from './ContentsModal';
import ExclusionTable from './ExclusionTable';
import ShareTable from './ShareTable';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'react-toastify';
import { Divider } from 'semantic-ui-react';

const ScanButton = ({ rescan, scanPending, working }) => (
  <ShrinkableButton
    color={scanPending ? 'yellow' : undefined}
    disabled={working}
    icon="refresh"
    loading={working}
    mediaQuery="(max-width: 516px)"
    onClick={() => rescan()}
    primary={!scanPending}
  >
    Rescan Shares
  </ShrinkableButton>
);

const CancelButton = ({ cancel, working }) => (
  <ShrinkableButton
    color="red"
    disabled={working}
    icon="x"
    mediaQuery="(max-width: 516px)"
    onClick={() => cancel()}
  >
    Cancel Scan
  </ShrinkableButton>
);

const Shares = ({ state = {}, theme } = {}) => {
  const [loading, setLoading] = useState(true);
  const [working, setWorking] = useState(false);
  const [shares, setShares] = useState([]);
  const [modal, setModal] = useState(false);
  const mountedRef = useRef(false);
  const requestIdRef = useRef(0);
  const operationIdRef = useRef(0);
  const initialLoadRef = useRef(true);

  const { directories, files, scanPending, scanProgress, scanning } = state;

  const getAll = useCallback(async (quiet = false) => {
    if (!mountedRef.current) return;
    const requestId = ++requestIdRef.current;
    try {
      if (!quiet) setLoading(true);

      const sharesByHost = await sharesLibrary.getAll();
      const groupedShares =
        sharesByHost && typeof sharesByHost === 'object' && !Array.isArray(sharesByHost)
          ? sharesByHost
          : {};
      const flattened = Object.entries(groupedShares).reduce(
        (accumulator, [host, sharesForHost]) => {
          const hostShares = Array.isArray(sharesForHost) ? sharesForHost : [];
          return accumulator.concat(
            hostShares.map((share) => ({ host, ...share })),
          );
        },
        [],
      );

      if (!mountedRef.current || requestIdRef.current !== requestId) return;
      setShares(flattened);
    } catch (error) {
      if (!mountedRef.current || requestIdRef.current !== requestId) return;
      console.error(error);
      toast.error(toDisplayError(error, 'Failed to load shares'));
    } finally {
      if (mountedRef.current && requestIdRef.current === requestId) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    const initialLoad = initialLoadRef.current;
    initialLoadRef.current = false;
    void getAll(!initialLoad);

    let refreshTimeout;
    if (!initialLoad && !scanning) {
      // the state change out of scanning can fire before
      // shares are updated, which leaves them stale. wait a second
      // and fetch again.
      refreshTimeout = setTimeout(() => getAll(true), 1_000);
    }

    return () => {
      if (refreshTimeout) clearTimeout(refreshTimeout);
      mountedRef.current = false;
      requestIdRef.current += 1;
    };
  }, [getAll, scanPending, scanning]);

  const rescan = async () => {
    if (!mountedRef.current || working) return;
    const operationId = ++operationIdRef.current;
    try {
      setWorking(true);
      await sharesLibrary.rescan();
    } catch (error) {
      console.error(error);
      if (
        mountedRef.current &&
        operationId === operationIdRef.current
      ) {
        toast.error(toDisplayError(error, 'Failed to rescan shares'));
      }
    } finally {
      if (
        mountedRef.current &&
        operationId === operationIdRef.current
      ) {
        setWorking(false);
      }
    }
  };

  const cancel = async () => {
    if (!mountedRef.current || working) return;
    const operationId = ++operationIdRef.current;
    try {
      setWorking(true);
      await sharesLibrary.cancel();
    } catch (error) {
      console.error(error);
      if (
        mountedRef.current &&
        operationId === operationIdRef.current
      ) {
        toast.error(toDisplayError(error, 'Failed to cancel the scan'));
      }
    } finally {
      if (
        mountedRef.current &&
        operationId === operationIdRef.current
      ) {
        setWorking(false);
      }
    }
  };

  const shared = shares.filter((share) => !share.isExcluded);
  const excluded = shares.filter((share) => share.isExcluded);

  return (
    <Switch loading={loading && <LoaderSegment />}>
      <div className="header-buttons">
        <Switch
          scanning={
            scanning && (
              <CancelButton
                cancel={cancel}
                working={working}
              />
            )
          }
        >
          <ScanButton
            rescan={rescan}
            scanPending={scanPending}
            working={working}
          />
        </Switch>
      </div>
      <Divider />
      <Switch
        filling={
          scanning && (
            <LoaderSegment>
              <div>
                <div>{Math.round(scanProgress * 100)}%</div>
                <div className="share-scan-detail">
                  Found {files} files in {directories} directories
                </div>
              </div>
            </LoaderSegment>
          )
        }
      >
        <ShareTable
          onClick={setModal}
          shares={shared}
        />
        <ExclusionTable exclusions={excluded} />
      </Switch>
      <ContentsModal
        onClose={() => setModal(false)}
        share={modal}
        theme={theme}
      />
    </Switch>
  );
};

export default Shares;
