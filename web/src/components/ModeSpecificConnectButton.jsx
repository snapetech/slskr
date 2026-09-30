import * as relayAPI from '../lib/relay';
import { connect, disconnect } from '../lib/server';
import { Icon, Menu } from 'semantic-ui-react';

const ModeSpecificConnectButton = ({
  runtimeProfile,
  connectionWatchdog,
  controller = {},
  mode,
  pendingReconnect,
  server,
  onConnect,
  user,
}) => {
  const compatibilityRole = runtimeProfile ? 'presentation' : undefined;

  if (mode === 'Agent') {
    const isConnected = controller?.state === 'Connected';
    const isTransitioning = ['Connecting', 'Reconnecting'].includes(
      controller?.state,
    );

    return (
      <Menu.Item
        onClick={() =>
          isConnected ? relayAPI.disconnect() : relayAPI.connect()
        }
        role={compatibilityRole}
      >
        <Icon.Group className="menu-icon-group">
          <Icon
            color={
              controller?.state === 'Connected'
                ? 'green'
                : isTransitioning
                  ? 'yellow'
                  : 'grey'
            }
            name="plug"
          />
          {!isConnected && (
            <Icon
              className="menu-icon-no-shadow"
              color="red"
              corner="bottom right"
              name="close"
            />
          )}
        </Icon.Group>
        Controller {controller?.state}
      </Menu.Item>
    );
  } else {
    if (server?.isConnected) {
      return (
        <Menu.Item
          disabled={server?.isDisconnecting}
          onClick={() => {
            if (!server?.isDisconnecting) {
              disconnect().catch((error) => {
                console.error('Failed to disconnect from Soulseek:', error);
              });
            }
          }}
          role={compatibilityRole}
        >
          <Icon.Group className="menu-icon-group">
            <Icon
              color={pendingReconnect ? 'yellow' : 'green'}
              name="plug"
            />
            {user?.privileges?.isPrivileged && (
              <Icon
                className="menu-icon-no-shadow"
                color="yellow"
                corner
                name="star"
              />
            )}
          </Icon.Group>
          Connected
        </Menu.Item>
      );
    }

    // the server is disconnected, and we need to give the user some information about what the client is doing
    // options are:
    // - nothing. the client was manually disconnected, kicked off by another login, etc., and we're not trying to connect
    // - actively trying to make a connection to the server
    // - still trying to connect, but waiting for the next connection attempt
    let icon = 'close';
    let color = 'red';

    if (connectionWatchdog?.isAttemptingConnection) {
      icon = 'clock';
      color = 'yellow';
    }

    const isSessionTransitioning =
      server?.isConnecting ||
      server?.IsConnecting ||
      server?.isLoggingIn ||
      server?.IsLoggingIn ||
      connectionWatchdog?.isAttemptingConnection;

    if (isSessionTransitioning) {
      icon = 'sync alternate loading';
      color = 'green';
    }

    const label = isSessionTransitioning
      ? 'Connecting'
      : server?.lastError
        ? 'Connection Failed'
        : 'Disconnected';

    return (
      <Menu.Item
        disabled={isSessionTransitioning}
        onClick={() => {
          if (!isSessionTransitioning) {
            onConnect?.(server) ?? connect();
          }
        }}
        role={compatibilityRole}
        title={server?.lastError || undefined}
      >
        <Icon.Group className="menu-icon-group">
          <Icon
            color="grey"
            name="plug"
          />
          <Icon
            className="menu-icon-no-shadow"
            color={color}
            corner="bottom right"
            name={icon}
          />
        </Icon.Group>
        {label}
      </Menu.Item>
    );
  }
};

export default ModeSpecificConnectButton;
