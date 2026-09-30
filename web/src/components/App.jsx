import 'react-toastify/dist/ReactToastify.css';
import './App.css';
import * as collectionsAPI from '../lib/collections';
import { createApplicationHubConnection } from '../lib/hubFactory';
import { getState as getApplicationState } from '../lib/application';
import { getCurrent as getApplicationOptions } from '../lib/options';
import * as relayAPI from '../lib/relay';
import { connect, disconnect } from '../lib/server';
import * as session from '../lib/session';
import { getLocalStorageItem, setLocalStorageItem } from '../lib/storage';
import { isPassthroughEnabled } from '../lib/token';
import {
  getStoredNetworkEndpointSnapshot,
  getVpnPortForwards,
  getVpnPortSignature,
  hasDismissedVpnPortNotice,
  storeDismissedVpnPortNotice,
  VpnPortChangeNotice,
} from './NetworkEndpointNotice';
export { getStoredNetworkEndpointSnapshot };
import AppNavigationActivity from './AppNavigationActivity';
import AppHeaderMenu from './AppHeaderMenu';
import AppNavigationPrimary from './AppNavigationPrimary';
import AppRouteTable from './AppRouteTable';
import AppContext from './AppContext';
import LoginForm from './LoginForm';
import PlayerBar from './Player/PlayerBar';
import { PlayerProvider } from './Player/PlayerContext';
import ErrorSegment from './Shared/ErrorSegment';
import Footer from './Shared/Footer';
import React, { Component, lazy, Suspense, useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import { ToastContainer } from 'react-toastify';
import {
  Button,
  Form,
  Header,
  Icon,
  Loader,
  Menu,
  Modal,
  Popup,
  Segment,
  Sidebar,
} from 'semantic-ui-react';

const THEME_OPTIONS = [
  { key: 'slskr', text: 'slskr', value: 'slskr' },
  { key: 'classic-dark', text: 'Classic Dark', value: 'classic-dark' },
  { key: 'light', text: 'Light', value: 'light' },
];

const THEME_LABELS = THEME_OPTIONS.reduce(
  (labels, option) => ({ ...labels, [option.value]: option.text }),
  {},
);

const SOULSEEK_CREDENTIAL_STORE_OPTIONS = [
  { key: 'memory', text: 'This session only', value: 'memory' },
  { key: 'os', text: 'OS credential store', value: 'os' },
  { key: 'file', text: 'Local credential file', value: 'file' },
];

const normalizeTheme = (theme) => {
  if (theme === 'light' || theme === 'classic-dark') {
    return theme;
  }

  return 'slskr';
};

const getSemanticTheme = (theme) => (theme === 'light' ? 'light' : 'dark');

const toDisplayError = (error, fallback = 'Request failed') => {
  const value = error?.response?.data ?? error?.message ?? error;
  if (typeof value === 'string' || typeof value === 'number') return String(value);
  if (value && typeof value === 'object') {
    for (const key of ['message', 'error', 'detail', 'title']) {
      if (typeof value[key] === 'string' && value[key].trim()) return value[key];
    }
  }
  return fallback;
};

const setNavigationHeightVariable = (element) => {
  if (!element || typeof document === 'undefined') return;

  const rect = element.getBoundingClientRect();
  const height = Math.ceil(rect.height || element.offsetHeight || 0);
  if (height > 0) {
    document.documentElement.style.setProperty(
      '--slskr-nav-height',
      `${height}px`,
    );
  }
};



const initialState = {
  applicationOptions: {},
  applicationState: {},
  error: false,
  initialized: false,
  login: {
    error: undefined,
    pending: false,
  },
  navActivity: {
    chat: false,
    rooms: false,
  },
  retriesExhausted: false,
  soulseekCredentials: {
    credentialStore: 'memory',
    error: undefined,
    open: false,
    password: '',
    pending: false,
    username: '',
  },
  themeMenuOpen: false,
};

const getRuntimeProfileHint = () => {
  if (typeof document === 'undefined') {
    return undefined;
  }

  const target = document
    .querySelector('meta[name="slskr-runtime-profile"]')
    ?.getAttribute('content');
  return ['legacy', 'native'].includes(target) ? target : undefined;
};



class App extends Component {
  constructor(props) {
    super(props);

    this.runtimeProfileHint = getRuntimeProfileHint();
    this.state = {
      ...initialState,
      applicationState: this.runtimeProfileHint
        ? { runtimeProfile: this.runtimeProfileHint }
        : initialState.applicationState,
    };
    this.applicationHub = undefined;
    this.navigationResizeObserver = undefined;
    this.isMountedFlag = false;
    this.connectionInFlight = false;
    this.loginInFlight = false;
    this.navigationActivity = new AppNavigationActivity({
      getCurrentPath: () => this.getCurrentPath(),
      isAuthenticated: () => this.isAuthenticated(),
      onActivityChange: (nextActivity) => this.setNavigationActivity(nextActivity),
      runtimeProfileHint: this.runtimeProfileHint,
    });
  }

  componentDidMount() {
    this.isMountedFlag = true;
    this.init();
    this.navigationActivity.start();
    document.addEventListener(
      'visibilitychange',
      this.navigationActivity.handleVisibilityChange,
    );
    this.startChromeMeasurement();
    this.syncThemeChrome();
  }

  componentDidUpdate(previousProps, previousState) {
    if (previousProps.location?.pathname !== this.props.location?.pathname) {
      this.navigationActivity.refresh();
    }
    if (
      previousState.initialized !== this.state.initialized ||
      previousState.login !== this.state.login ||
      previousState.theme !== this.state.theme
    ) {
      this.syncThemeChrome();
    }
    this.updateNavigationHeight();
  }

  componentWillUnmount() {
    this.isMountedFlag = false;
    this.navigationActivity.stop();
    if (this.applicationHub) {
      this.applicationHub.stop().catch(() => {});
      this.applicationHub = undefined;
    }

    document.removeEventListener(
      'visibilitychange',
      this.navigationActivity.handleVisibilityChange,
    );

    if (this.navigationResizeObserver) {
      this.navigationResizeObserver.disconnect();
      this.navigationResizeObserver = undefined;
    }
  }

  startChromeMeasurement = () => {
    this.updateNavigationHeight();
    if (typeof window.ResizeObserver !== 'function') {
      return;
    }

    const navigation = document.querySelector('.navigation');
    if (!navigation) {
      return;
    }

    this.navigationResizeObserver = new window.ResizeObserver(
      this.updateNavigationHeight,
    );
    this.navigationResizeObserver.observe(navigation);
  };

  updateNavigationHeight = () => {
    setNavigationHeightVariable(document.querySelector('.navigation'));
  };

  getCurrentPath = () =>
    this.props.location?.pathname || window.location?.pathname || '';

  isAuthenticated = () => session.isLoggedIn() || isPassthroughEnabled();

  setNavigationActivity = (nextActivity) => {
    const currentActivity = this.state.navActivity;
    if (
      currentActivity.chat === nextActivity.chat &&
      currentActivity.rooms === nextActivity.rooms
    ) {
      return;
    }
    this.setState({ navActivity: nextActivity });
  };

  startApplicationHub = () => {
    if (!this.isMountedFlag) return;
    if (this.applicationHub) {
      this.applicationHub.stop().catch(() => {});
    }

    const HUB_START_TIMEOUT_MS = 30000;
    const appHub = createApplicationHubConnection();
    this.applicationHub = appHub;

    appHub.on('state', (state) => {
      if (!this.isMountedFlag || this.applicationHub !== appHub) return;
      this.setState({
        applicationState: this.runtimeProfileHint
          ? { ...state, runtimeProfile: this.runtimeProfileHint }
          : state,
      });
    });

    appHub.on('options', (options) => {
      if (!this.isMountedFlag || this.applicationHub !== appHub) return;
      this.setState({ applicationOptions: options });
    });

    appHub.onreconnecting(() =>
      this.isMountedFlag &&
      this.applicationHub === appHub &&
      this.setState({ error: true, retriesExhausted: false }),
    );
    appHub.onclose(() =>
      this.isMountedFlag &&
      this.applicationHub === appHub &&
      this.setState({ error: true, retriesExhausted: true }),
    );
    appHub.onreconnected(() =>
      this.isMountedFlag &&
      this.applicationHub === appHub &&
      this.setState({ error: false, retriesExhausted: false }),
    );

    const hubStart = appHub.start();
    let hubTimeoutId;
    const hubTimeout = new Promise((_, reject) => {
      hubTimeoutId = setTimeout(
        () => reject(new Error('HubConnectionTimeout')),
        HUB_START_TIMEOUT_MS,
      );
    });

    Promise.race([hubStart, hubTimeout])
      .catch((error) => {
        if (!this.isMountedFlag || this.applicationHub !== appHub) {
          return;
        }

        if (error?.message === 'HubConnectionTimeout') {
          console.warn(
            'Event feed connection timed out during background startup; allowing the UI to continue while WebSocket reconnects.',
          );
          return;
        }

        console.error(error);
        this.setState({ error: true, retriesExhausted: false });
      })
      .finally(() => {
        if (hubTimeoutId) {
          clearTimeout(hubTimeoutId);
        }

        // Prevent unhandled rejections if the timeout wins and the start later faults.
        hubStart.catch(() => {});
      });
  };

  init = async () => {
    this.setState({ initialized: false }, async () => {
      if (!this.isMountedFlag) return;
      const INIT_TOTAL_TIMEOUT_MS = 30000;

      let initTimedOut = false;
      let initTimeoutId;
      try {
        const initTask = (async () => {
          const securityEnabled = await session.getSecurityEnabled();
          if (!this.isMountedFlag) return;

          if (!securityEnabled) {
            console.debug('application security is not enabled, per api call');
            session.enablePassthrough();
          }

          const sessionValid =
            !securityEnabled && this.runtimeProfileHint === 'native'
              ? true
              : await session.check();
          if (!this.isMountedFlag) return;

          if (sessionValid) {
            if (this.runtimeProfileHint === 'native') {
              // The frozen native UI bootstraps from the build and collection
              // contracts.  Its system/application state arrives through the
              // native hubs; requesting the legacy application/options pair
              // here changes the observable API surface and makes a native
              // profile look like slskd to target clients.
              await collectionsAPI.getCollections();
              if (!this.isMountedFlag) return;
              this.setState({
                applicationOptions: {},
                applicationState: { runtimeProfile: 'native' },
              });
            } else if (this.runtimeProfileHint === 'legacy') {
              // The frozen slskd UI receives its application state and
              // options from the application hub.  Keep those legacy REST
              // bootstrap requests out of the compatibility profile so the
              // replacement has the same startup API inventory.
              this.setState({
                applicationOptions: {},
                applicationState: { runtimeProfile: 'legacy' },
              });
            } else {
              const [initialApplicationState, initialOptions] =
                await Promise.all([
                  getApplicationState(),
                  getApplicationOptions(),
                ]);
              if (!this.isMountedFlag) return;
              this.setState({
                applicationOptions: initialOptions || {},
                applicationState: {
                  ...(initialApplicationState || {}),
                },
              });
            }
            this.startApplicationHub();
          }

          const savedTheme = this.getSavedTheme();
          if (!this.isMountedFlag) return;
          if (savedTheme != null) {
            this.setState({ theme: savedTheme });
          }

          this.setState({
            error: false,
          });
        })();

        // Safety timeout so a stalled init doesn't keep the UI on the big loader forever.
        const initTimeout = new Promise((resolve) => {
          initTimeoutId = setTimeout(() => {
            initTimedOut = true;
            resolve();
          }, INIT_TOTAL_TIMEOUT_MS);
        });

        await Promise.race([initTask, initTimeout]);

        // Prevent unhandled rejections if the timeout wins.
        initTask.catch((error) => {
          if (initTimedOut) {
            console.warn('Init completed after timeout.', error);
          }
        });

        if (initTimedOut) {
          console.warn('Init timed out; showing UI (hub/state may reconnect later).');
        }
      } catch (error) {
        if (!initTimedOut && this.isMountedFlag) {
          console.error(error);
          this.setState({ error: true, retriesExhausted: true });
        }
      } finally {
        if (initTimeoutId) {
          clearTimeout(initTimeoutId);
        }
        if (this.isMountedFlag) this.setState({ initialized: true });
      }
    });
  };

  getSavedTheme = () => {
    const savedTheme = getLocalStorageItem('slskr-theme');
    return savedTheme == null ? null : normalizeTheme(savedTheme);
  };

  setTheme = (theme) => {
    const nextTheme = normalizeTheme(theme);
    setLocalStorageItem('slskr-theme', nextTheme);
    this.setState({ theme: nextTheme, themeMenuOpen: false });
  };

  openThemeMenu = () => {
    this.setState({ themeMenuOpen: true });
  };

  closeThemeMenu = () => {
    this.setState({ themeMenuOpen: false });
  };

  openSoulseekCredentials = (server) => {
    const modes = Array.isArray(server?.writableCredentialStoreModes)
      ? server.writableCredentialStoreModes
      : [];
    const defaultCredentialStore = modes.includes(server?.credentialStore)
      ? server.credentialStore
      : 'memory';
    this.setState((previousState) => ({
      soulseekCredentials: {
        ...previousState.soulseekCredentials,
        credentialStore: defaultCredentialStore,
        error: undefined,
        open: true,
      },
    }));
  };

  closeSoulseekCredentials = () => {
    this.setState((previousState) => ({
      soulseekCredentials: {
        ...previousState.soulseekCredentials,
        error: undefined,
        open: false,
        password: '',
        pending: false,
      },
    }));
  };

  updateSoulseekCredential = (field, value) => {
    this.setState((previousState) => ({
      soulseekCredentials: {
        ...previousState.soulseekCredentials,
        [field]: value,
      },
    }));
  };

  updateServerState = (server) => {
    if (!server) return;

    this.setState((previousState) => ({
      applicationState: {
        ...previousState.applicationState,
        server,
      },
    }));
  };

  setServerConnectError = (error) => {
    this.setState((previousState) => ({
      applicationState: {
        ...previousState.applicationState,
        server: {
          ...previousState.applicationState?.server,
          isConnecting: false,
          isLoggingIn: false,
          lastError: toDisplayError(error, 'Unable to connect to Soulseek.'),
        },
      },
    }));
  };

  handleSoulseekConnect = async (server) => {
    if (this.connectionInFlight || !this.isMountedFlag) return;
    const credentialsConfigured =
      server?.credentialsConfigured ||
      server?.runtimeCredentialsConfigured ||
      server?.credentialSource === 'config' ||
      server?.credentialSource === 'runtime';

    if (credentialsConfigured) {
      this.connectionInFlight = true;
      try {
        const response = await connect();
        this.updateServerState(response?.data);
      } catch (error) {
        console.error('Failed to connect to Soulseek:', error);
        this.setServerConnectError(error);
      } finally {
        this.connectionInFlight = false;
      }
      return;
    }

    this.openSoulseekCredentials(server);
  };

  submitSoulseekCredentials = async () => {
    if (this.connectionInFlight || !this.isMountedFlag) return;
    const { soulseekCredentials = {} } = this.state;
    const username = (soulseekCredentials.username || '').trim();
    const password = soulseekCredentials.password || '';

    if (!username || !password) {
      this.setState((previousState) => ({
        soulseekCredentials: {
          ...previousState.soulseekCredentials,
          error: 'Username and password are required.',
        },
      }));
      return;
    }

    this.connectionInFlight = true;
    this.setState((previousState) => ({
      soulseekCredentials: {
        ...previousState.soulseekCredentials,
        error: undefined,
        pending: true,
      },
    }));

    try {
      const response = await connect({
        credentialStore: soulseekCredentials.credentialStore || 'memory',
        password,
        username,
      });
      this.updateServerState(response?.data);
      this.closeSoulseekCredentials();
    } catch (error) {
      this.setState((previousState) => ({
        soulseekCredentials: {
          ...previousState.soulseekCredentials,
          error: toDisplayError(
            error,
            'Unable to connect with those credentials.',
          ),
          password: '',
          pending: false,
        },
      }));
    } finally {
      this.connectionInFlight = false;
    }
  };

  dismissVpnPortNotice = (signature, portForwards) => {
    storeDismissedVpnPortNotice(signature, portForwards);
    this.forceUpdate();
  };

  handleLogin = (username, password) => {
    if (this.loginInFlight || !this.isMountedFlag) return;
    this.loginInFlight = true;
    this.setState(
      (previousState) => ({
        login: { ...previousState.login, error: undefined, pending: true },
      }),
      async () => {
        try {
          await session.login({ password, username });
          this.setState(
            (previousState) => ({
              login: { ...previousState.login, error: false, pending: false },
            }),
            () => this.init(),
          );
        } catch (error) {
          this.setState((previousState) => ({
            login: { ...previousState.login, error, pending: false },
          }));
        } finally {
          this.loginInFlight = false;
        }
      },
    );
  };

  logout = () => {
    session.logout();
    this.setState({ login: { ...initialState.login } });
  };

  withTokenCheck = (component) => {
    return component;
  };

  syncThemeChrome = () => {
    if (
      !this.state.initialized ||
      (!session.isLoggedIn() && !isPassthroughEnabled())
    ) {
      return;
    }
    const theme = normalizeTheme(this.state.theme || this.getSavedTheme() || 'slskr');
    const semanticTheme = getSemanticTheme(theme);
    document.title = 'slskR';
    document.documentElement.classList.remove(
      'classic-dark',
      'dark',
      'light',
      'slskr',
    );
    document.documentElement.classList.add(theme);
    if (semanticTheme === 'dark') {
      document.documentElement.classList.add('dark');
    }
  };

  // eslint-disable-next-line complexity
  render() {
    const {
      applicationOptions = {},
      applicationState = {},
      error,
      initialized,
      login,
      navActivity,
      retriesExhausted,
      soulseekCredentials = {},
      theme = normalizeTheme(this.getSavedTheme() || 'slskr'),
      themeMenuOpen,
    } = this.state;
    const semanticTheme = getSemanticTheme(theme);
    const {
      connectionWatchdog = {},
      pendingReconnect,
      pendingRestart,
      relay = {},
      server,
      shares = {},
      user,
      version = {},
    } = applicationState;
    const { current, isUpdateAvailable, latest } = version;
    const { scanPending: pendingShareRescan } = shares;
    const vpnPortForwards = getVpnPortForwards(applicationState.vpn);
    const vpnPortSignature = getVpnPortSignature(vpnPortForwards);
    const previousNetworkEndpointSnapshot = getStoredNetworkEndpointSnapshot();
    const showVpnPortNotice =
      vpnPortSignature &&
      applicationState.vpn?.isReady &&
      !hasDismissedVpnPortNotice(vpnPortSignature) &&
      previousNetworkEndpointSnapshot?.signature !== vpnPortSignature;

    const { controller, mode } = relay;
    const runtimeProfile = ['legacy', 'native'].includes(
      applicationState.runtimeProfile,
    )
      ? applicationState.runtimeProfile
      : undefined;
    const isLegacyProfile = runtimeProfile === 'legacy';
    const isNativeProfile = runtimeProfile === 'native';

    if (!initialized) {
      return (
        <Loader
          active
          size="big"
        />
      );
    }

    if (!session.isLoggedIn() && !isPassthroughEnabled()) {
      if (error) {
        return (
          <ErrorSegment
            caption={
              <>
                <span>Lost connection to slskr</span>
                <br />
                <span>
                  {retriesExhausted ? 'Refresh to reconnect' : 'Retrying...'}
                </span>
              </>
            }
            icon="attention"
            suppressPrefix
          />
        );
      }

      return (
        <LoginForm
          error={login.error}
          initialized={login.initialized}
          loading={login.pending}
          onLoginAttempt={this.handleLogin}
        />
      );
    }

    const isAgent = mode === 'Agent';

    return (
      <>
        {error && (
          <Segment
            color="red"
            inverted
            style={{
              borderRadius: 0,
              margin: 0,
              padding: '0.75rem 1rem',
            }}
          >
            <Icon name="attention" />
            Lost connection to slskr. {retriesExhausted ? 'Refresh to reconnect.' : 'Retrying...'}
          </Segment>
        )}
        <Modal
          centered
          closeIcon={!soulseekCredentials.pending}
          onClose={this.closeSoulseekCredentials}
          open={soulseekCredentials.open}
          size="mini"
        >
          <Modal.Header>Soulseek Authentication</Modal.Header>
          <Modal.Content>
            <Form
              error={Boolean(soulseekCredentials.error)}
              onSubmit={this.submitSoulseekCredentials}
            >
              <Form.Input
                autoComplete="username"
                disabled={soulseekCredentials.pending}
                label="Username"
                onChange={(_, data) =>
                  this.updateSoulseekCredential('username', data.value)
                }
                value={soulseekCredentials.username}
              />
              <Form.Input
                autoComplete="current-password"
                disabled={soulseekCredentials.pending}
                label="Password"
                onChange={(_, data) =>
                  this.updateSoulseekCredential('password', data.value)
                }
                type="password"
                value={soulseekCredentials.password}
              />
              <Form.Select
                disabled={soulseekCredentials.pending}
                label="Credential storage"
                onChange={(_, data) =>
                  this.updateSoulseekCredential('credentialStore', data.value)
                }
                options={SOULSEEK_CREDENTIAL_STORE_OPTIONS.filter((option) =>
                  (server?.writableCredentialStoreModes || ['memory']).includes(
                    option.value,
                  ),
                )}
                value={soulseekCredentials.credentialStore}
              />
              {soulseekCredentials.error && (
                <Segment
                  color="red"
                  inverted
                >
                  {soulseekCredentials.error}
                </Segment>
              )}
            </Form>
          </Modal.Content>
          <Modal.Actions>
            <Button
              disabled={soulseekCredentials.pending}
              onClick={this.closeSoulseekCredentials}
            >
              Cancel
            </Button>
            <Button
              loading={soulseekCredentials.pending}
              onClick={this.submitSoulseekCredentials}
              primary
            >
              Connect
            </Button>
          </Modal.Actions>
        </Modal>
        <PlayerProvider>
          <Sidebar.Pushable
            as={Segment}
            className="app"
            data-runtime-profile={runtimeProfile || 'unknown'}
          >
            <Sidebar
              animation="overlay"
              as={Menu}
              className="navigation"
              direction="top"
              horizontal="true"
              icon="labeled"
              inverted
              visible
              width="thin"
            >
              <AppNavigationPrimary
                isAgent={isAgent}
                isLegacyProfile={isLegacyProfile}
                isNativeProfile={isNativeProfile}
                navActivity={navActivity}
                version={version}
              />
            <AppHeaderMenu
              connectionWatchdog={connectionWatchdog}
              controller={controller}
              current={current}
              isLoggedIn={session.isLoggedIn()}
              isUpdateAvailable={isUpdateAvailable}
              latest={latest}
              mode={mode}
              onCloseThemeMenu={this.closeThemeMenu}
              onConnect={this.handleSoulseekConnect}
              onLogout={this.logout}
              onOpenThemeMenu={this.openThemeMenu}
              onSetTheme={this.setTheme}
              pendingReconnect={pendingReconnect}
              pendingRestart={pendingRestart}
              pendingShareRescan={pendingShareRescan}
              runtimeProfile={runtimeProfile}
              server={server}
              theme={theme}
              themeMenuOpen={themeMenuOpen}
              themeOptions={THEME_OPTIONS}
              user={user}
            />
            </Sidebar>
            <Sidebar.Pusher className="app-content">
              {showVpnPortNotice && (
                <VpnPortChangeNotice
                  onDismiss={() =>
                    this.dismissVpnPortNotice(vpnPortSignature, vpnPortForwards)
                  }
                  options={applicationOptions}
                  portForwards={vpnPortForwards}
                />
              )}
              <AppContext.Provider
                // Note: Context value object recreated on each render (class component limitation)
                // Deferred: Optimize with useMemo when converting to functional component
                // Deferred until this class component is converted to hooks.
                // eslint-disable-next-line react/jsx-no-constructed-context-values
                value={{ options: applicationOptions, state: applicationState }}
              >
                <Suspense
                  fallback={
                    <Segment
                      basic
                      className="view"
                    >
                      <Loader active />
                    </Segment>
                  }
                >
                  <AppRouteTable
                    applicationOptions={applicationOptions}
                    applicationState={applicationState}
                    isAgent={isAgent}
                    isLegacyProfile={isLegacyProfile}
                    runtimeProfile={runtimeProfile}
                    semanticTheme={semanticTheme}
                    withTokenCheck={this.withTokenCheck}
                  />
                </Suspense>
              </AppContext.Provider>
            </Sidebar.Pusher>
          </Sidebar.Pushable>
          {!isLegacyProfile && (
            <PlayerBar runtimeProfile={runtimeProfile} />
          )}
        </PlayerProvider>
        <ToastContainer
          autoClose={5_000}
          closeOnClick
          draggable={false}
          hideProgressBar={false}
          newestOnTop
          pauseOnFocusLoss
          pauseOnHover
          position="bottom-center"
          rtl={false}
        />
        <Footer runtimeProfile={runtimeProfile} />
      </>
    );
  }
}

const AppWithLocation = (props) => {
  const location = useLocation();
  return (
    <App
      {...props}
      location={location}
    />
  );
};

export { App };
export default AppWithLocation;
