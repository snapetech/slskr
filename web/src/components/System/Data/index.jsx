import { clearCompleted } from '../../../lib/transfers';
import { toDisplayError } from '../../../lib/errors';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useState } from 'react';
import { toast } from 'react-toastify';
import { Button, Divider, Header, Icon } from 'semantic-ui-react';

const clear = async ({ direction, isMounted, isInFlight, setInFlight, setState }) => {
  if (!isMounted() || isInFlight()) return;
  setInFlight(true);
  setState(true);
  try {
    await clearCompleted({ direction });
    if (isMounted()) {
      toast.success(`Completed ${direction}s cleared!`);
    }
  } catch (error) {
    if (isMounted()) {
      toast.error(toDisplayError(error, `Failed to clear completed ${direction}s`));
    }
  } finally {
    if (isMounted()) setState(false);
    setInFlight(false);
  }
};

const Data = () => {
  const [up, setUp] = useState(false);
  const [down, setDown] = useState(false);
  const mountedRef = useMountedRef();
  const upInFlightRef = React.useRef(false);
  const downInFlightRef = React.useRef(false);

  return (
    <div>
      <Header
        as="h3"
        className="transfer-header"
      >
        Transfer Data
      </Header>
      <Divider />
      <p>
        <span>
          The Uploads and Downloads pages can become unresponsive if too many
          transfers are displayed. If you're having trouble with either page,
          try using the buttons below to remove completed transfers.
        </span>
      </p>
      <Button
        disabled={up}
        loading={up}
        onClick={() => clear({
          direction: 'upload',
          isInFlight: () => upInFlightRef.current,
          isMounted: () => mountedRef.current,
          setInFlight: (value) => {
            upInFlightRef.current = value;
          },
          setState: setUp,
        })}
        primary
      >
        <Icon name="trash alternate" />
        Clear All Completed Uploads
      </Button>
      <Button
        disabled={down}
        loading={down}
        onClick={() => clear({
          direction: 'download',
          isInFlight: () => downInFlightRef.current,
          isMounted: () => mountedRef.current,
          setInFlight: (value) => {
            downInFlightRef.current = value;
          },
          setState: setDown,
        })}
        primary
      >
        <Icon name="trash alternate" />
        Clear All Completed Downloads
      </Button>
    </div>
  );
};

export default Data;
