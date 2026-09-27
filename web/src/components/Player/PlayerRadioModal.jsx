import * as searches from '../../lib/searches';
import * as wishlistAPI from '../../lib/wishlist';
import {
  buildPlayerRadioPlan,
  getPlayerRadioQueries,
  getPlayerRadioCopyText,
} from '../../lib/playerRadio';
import { copyToClipboard } from '../../lib/clipboard';
import { toDisplayError } from '../../lib/errors';
import { useMountedRef } from '../../lib/useMountedRef';
import { useEffect, useRef, useState } from 'react';
import {
  Button,
  Icon,
  Label,
  Message,
  Modal,
  Popup,
} from 'semantic-ui-react';

const PlayerRadioModal = ({ current, onClose, onOpenSearch, open }) => {
  const plan = buildPlayerRadioPlan(current);
  const copyText = getPlayerRadioCopyText(plan);
  const [runningSearches, setRunningSearches] = useState(false);
  const [savingWishlist, setSavingWishlist] = useState(false);
  const [status, setStatus] = useState('');
  const mountedRef = useMountedRef();
  const openRef = useRef(open);
  const actionRequestIdRef = useRef(0);
  const actionInFlightRef = useRef(false);

  useEffect(() => {
    openRef.current = open;
    if (!open) {
      actionRequestIdRef.current += 1;
      setRunningSearches(false);
      setSavingWishlist(false);
    }
  }, [open]);

  const copyPlan = async () => {
    if (!copyText) return;
    try {
      const copied = await copyToClipboard(copyText);
      if (!mountedRef.current) return;
      setStatus(copied ? 'Radio plan copied.' : 'Radio plan prepared; clipboard is unavailable.');
    } catch (error) {
      if (mountedRef.current) {
        setStatus(toDisplayError(error, 'Unable to copy radio plan.'));
      }
    }
  };

  const startRadioSearches = async () => {
    if (
      !mountedRef.current ||
      !openRef.current ||
      actionInFlightRef.current
    ) {
      return;
    }
    const requestId = ++actionRequestIdRef.current;
    const isCurrentRequest = () =>
      mountedRef.current && openRef.current && actionRequestIdRef.current === requestId;
    const queries = getPlayerRadioQueries(plan, { limit: 3 });
    if (queries.length === 0) {
      setStatus('No smart-radio queries are ready.');
      return;
    }

    actionInFlightRef.current = true;
    try {
      setRunningSearches(true);
      const count = await searches.createBatch({ queries });
      if (!isCurrentRequest()) return;
      setStatus(`Started ${count} smart-radio search${count === 1 ? '' : 'es'}.`);
    } catch {
      if (!isCurrentRequest()) return;
      setStatus('Unable to start smart-radio searches.');
    } finally {
      actionInFlightRef.current = false;
      if (isCurrentRequest()) setRunningSearches(false);
    }
  };

  const addRadioWishlist = async () => {
    if (
      !mountedRef.current ||
      !openRef.current ||
      actionInFlightRef.current
    ) {
      return;
    }
    const requestId = ++actionRequestIdRef.current;
    const isCurrentRequest = () =>
      mountedRef.current && openRef.current && actionRequestIdRef.current === requestId;
    const queries = getPlayerRadioQueries(plan, { limit: 4 });
    if (queries.length === 0) {
      setStatus('No smart-radio queries are ready for Wishlist.');
      return;
    }

    actionInFlightRef.current = true;
    try {
      setSavingWishlist(true);
      await queries.reduce(
        (chain, searchText) =>
          chain.then(() =>
            wishlistAPI.create({
              autoDownload: false,
              enabled: true,
              filter: '',
              maxResults: 50,
              searchText,
            }),
          ),
        Promise.resolve(),
      );
      if (!isCurrentRequest()) return;
      setStatus(`Added ${queries.length} smart-radio seed${queries.length === 1 ? '' : 's'} to Wishlist.`);
    } catch {
      if (!isCurrentRequest()) return;
      setStatus('Unable to add smart-radio seeds to Wishlist.');
    } finally {
      actionInFlightRef.current = false;
      if (isCurrentRequest()) setSavingWishlist(false);
    }
  };

  return (
    <Modal
      className="player-browser-modal player-radio-modal"
      onClose={onClose}
      open={open}
      size="small"
    >
      <Modal.Header>Smart Radio Seed</Modal.Header>
      <Modal.Content>
        <p className="player-modal-copy">
          Build review-first radio searches from the current track. Nothing is
          searched or queued until you choose a query.
        </p>
        <div className="player-radio-seed" data-testid="player-radio-seed">
          <Icon name="random" />
          <div>
            <strong>{plan.seedLabel}</strong>
            <div>
              {plan.basis.length > 0 ? plan.basis.join(' | ') : 'Pick a track first.'}
            </div>
          </div>
        </div>
        <div className="player-radio-query-list">
          {plan.queries.map((item) => (
            <div className="player-radio-query" key={item.id}>
              <div>
                <Label color="violet" size="mini">
                  {item.reason}
                </Label>
                <code>{item.query}</code>
              </div>
              <Popup
                content="Open this as a normal Search page query. This is the point where network search work can begin."
                trigger={
                  <Button
                    data-testid={`player-radio-search-${item.id}`}
                    onClick={() => onOpenSearch(item.query)}
                    size="mini"
                    type="button"
                  >
                    <Icon name="search" />
                    Search
                  </Button>
                }
              />
            </div>
          ))}
        </div>
        {status ? (
          <Message compact size="mini">
            {status}
          </Message>
        ) : null}
      </Modal.Content>
      <Modal.Actions>
        <Popup
          content="Start up to three live searches from this smart-radio plan. This does not browse peers, queue downloads, or mutate files."
          trigger={
            <Button
              data-testid="player-radio-start-searches"
              disabled={!plan.ready || savingWishlist || runningSearches}
              loading={runningSearches}
              onClick={startRadioSearches}
              type="button"
            >
              <Icon name="search" />
              Start Searches
            </Button>
          }
        />
        <Popup
          content="Add smart-radio seeds to Wishlist as enabled manual requests with auto-download off."
          trigger={
            <Button
              data-testid="player-radio-add-wishlist"
              disabled={!plan.ready || savingWishlist || runningSearches}
              loading={savingWishlist}
              onClick={addRadioWishlist}
              type="button"
            >
              <Icon name="heart" />
              Add Wishlist
            </Button>
          }
        />
        <Popup
          content="Copy the generated radio search plan as plain text."
          trigger={
            <Button
              data-testid="player-radio-copy"
              disabled={!copyText}
              onClick={copyPlan}
              type="button"
            >
              <Icon name="copy outline" />
              Copy Plan
            </Button>
          }
        />
        <Popup
          content="Close smart radio without starting a search."
          trigger={
            <Button
              data-testid="player-radio-close"
              onClick={onClose}
              primary
              type="button"
            >
              <Icon name="check" />
              Done
            </Button>
          }
        />
      </Modal.Actions>
    </Modal>
  );
};

export default PlayerRadioModal;
