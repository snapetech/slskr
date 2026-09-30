import { useEffect, useRef, useState } from 'react';
import * as streaming from '../../lib/streaming';

const usePlayerStreamSource = ({ current, mountedRef }) => {
  const streamRequestIdRef = useRef(0);
  const [source, setSource] = useState('');

  useEffect(() => {
    let cancelled = false;
    const requestId = ++streamRequestIdRef.current;

    if (!current) {
      setSource('');
      return undefined;
    }

    if (current.streamUrl) {
      setSource(current.streamUrl);
      return undefined;
    }

    if (!current.contentId) {
      setSource('');
      return undefined;
    }

    streaming
      .createStreamTicket(current.contentId)
      .then((ticket) => {
        if (
          !cancelled &&
          mountedRef.current &&
          streamRequestIdRef.current === requestId
        ) {
          setSource(ticket
            ? streaming.buildTicketedStreamUrl(current.contentId, ticket)
            : streaming.buildDirectStreamUrl(current.contentId));
        }
      })
      .catch(() => {
        if (
          !cancelled &&
          mountedRef.current &&
          streamRequestIdRef.current === requestId
        ) {
          setSource(streaming.buildDirectStreamUrl(current.contentId));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [current, mountedRef]);
  return source;
};

export default usePlayerStreamSource;
