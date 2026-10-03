import ModeSpecificConnectButton from './ModeSpecificConnectButton';
import { NavLink } from 'react-router-dom';
import {
  Button,
  Header,
  Icon,
  Menu,
  Modal,
  Popup,
} from 'semantic-ui-react';

const SLSKR_RELEASES_URL = 'https://github.com/snapetech/slskr/releases';

const AppHeaderMenu = ({
  connectionWatchdog,
  controller,
  current,
  isLoggedIn,
  isUpdateAvailable,
  latest,
  mode,
  onCloseThemeMenu,
  onConnect,
  onLogout,
  onOpenThemeMenu,
  onSetTheme,
  pendingReconnect,
  pendingRestart,
  pendingShareRescan,
  runtimeProfile,
  server,
  theme,
  themeMenuOpen,
  themeOptions,
  user,
}) => (
            <Menu
              aria-label="Session controls"
              className="right"
              inverted
              role="navigation"
            >
              <ModeSpecificConnectButton
                runtimeProfile={runtimeProfile}
                connectionWatchdog={connectionWatchdog}
                controller={controller}
                mode={mode}
                onConnect={onConnect}
                pendingReconnect={pendingReconnect}
                server={server}
                user={user}
              />
              <Popup
                basic
                className="theme-picker-popup"
                on="click"
                onClose={onCloseThemeMenu}
                onOpen={onOpenThemeMenu}
                open={themeMenuOpen}
                pinned
                position="bottom right"
                trigger={(
                  <Menu.Item
                    className={`theme-menu ${themeMenuOpen ? 'visible' : ''}`}
                    data-testid="theme-menu"
                    role={runtimeProfile ? 'presentation' : undefined}
                    title="Choose the web UI color theme"
                  >
                    <Icon name="paint brush" />
                    <span className="theme-menu-label">Theme</span>
                  </Menu.Item>
                )}
              >
                <Menu
                  className="theme-picker-menu"
                  vertical
                >
                  {themeOptions.map((option) => (
                    <Menu.Item
                      active={theme === option.value}
                      data-testid={`theme-option-${option.value}`}
                      key={option.value}
                      onClick={() => onSetTheme(option.value)}
                    >
                      <Icon name="theme" />
                      {option.text}
                    </Menu.Item>
                  ))}
                </Menu>
              </Popup>
              {(pendingReconnect || pendingRestart || pendingShareRescan) && (
                <Menu.Item position="right">
                  <Icon.Group className="menu-icon-group">
                    <NavLink to="/system/info">
                      <Icon
                        color="yellow"
                        name="exclamation circle"
                      />
                    </NavLink>
                  </Icon.Group>
                  Pending Action
                </Menu.Item>
              )}
              {isUpdateAvailable && (
                <Modal
                  centered
                  closeIcon
                  size="mini"
                  trigger={
                    <Menu.Item position="right">
                      <Icon.Group className="menu-icon-group">
                        <Icon
                          color="yellow"
                          name="bullhorn"
                        />
                      </Icon.Group>
                      New Version!
                    </Menu.Item>
                  }
                >
                  <Modal.Header>New Version!</Modal.Header>
                  <Modal.Content>
                    <p>
                      You are currently running version{' '}
                      <strong>{current}</strong>
                      while version <strong>{latest}</strong> is available.
                    </p>
                  </Modal.Content>
                  <Modal.Actions>
                    <Button
                      fluid
                      href={SLSKR_RELEASES_URL}
                      primary
                      style={{ marginLeft: 0 }}
                    >
                      See Release Notes
                    </Button>
                  </Modal.Actions>
                </Modal>
              )}
              <NavLink to="/system">
                <Menu.Item data-testid="nav-system">
                  <Icon name="cogs" />
                  System
                </Menu.Item>
              </NavLink>
              {isLoggedIn && (
                <Modal
                  actions={[
                    'Cancel',
                    {
                      content: 'Log Out',
                      key: 'done',
                      negative: true,
                      onClick: onLogout,
                    },
                  ]}
                  centered
                  content="Are you sure you want to log out?"
                  header={
                    <Header
                      content="Confirm Log Out"
                      icon="sign-out"
                    />
                  }
                  size="mini"
                  trigger={
                    <Menu.Item data-testid="logout">
                      <Icon name="sign-out" />
                      Log Out
                    </Menu.Item>
                  }
                />
              )}
            </Menu>
);

export default AppHeaderMenu;
