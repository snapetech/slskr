import { clearCompleted } from '../../../lib/transfers';
import { toDisplayError } from '../../../lib/errors';
import { useMountedRef } from '../../../lib/useMountedRef';
import React, { useState } from 'react';
import { toast } from 'react-toastify';
import { Button, Divider, Header, Icon } from 'semantic-ui-react';

const clear = async ({ direction, isMounted, setState }) => {
  if (!isMounted()) return;
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
  }
};

const Data = () => {
  const [up, setUp] = useState(false);
  const [down, setDown] = useState(false);
  const mountedRef = useMountedRef();

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
        onClick={() =>
          clear({ direction: 'upload', isMounted: () => mountedRef.current, setState: setUp })
        }
        primary
      >
        <Icon name="trash alternate" />
        Clear All Completed Uploads
      </Button>
      <Button
        disabled={down}
        loading={down}
        onClick={() =>
          clear({ direction: 'download', isMounted: () => mountedRef.current, setState: setDown })
        }
        primary
      >
        <Icon name="trash alternate" />
        Clear All Completed Downloads
      </Button>
    </div>
  );
};

export default Data;
