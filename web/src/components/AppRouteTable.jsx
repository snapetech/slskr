import { lazy, useEffect } from 'react';
import { Navigate, Route, Routes, useLocation } from 'react-router-dom';
import ErrorSegment from './Shared/ErrorSegment';

const Browse = lazy(() => import('./Browse/Browse'));
const Chat = lazy(() => import('./Chat/Chat'));
const Collections = lazy(() => import('./Collections/Collections'));
const Contacts = lazy(() => import('./Contacts/Contacts'));
const CompatibilityDashboard = lazy(() => import('./CompatibilityDashboard'));
const DiscoveryGraphAtlasPage = lazy(() =>
  import('./Search/DiscoveryGraphAtlasPage'));
const Messaging = lazy(() => import('./Messaging/Messaging'));
const PlaylistIntake = lazy(() => import('./PlaylistIntake/PlaylistIntake'));
const Rooms = lazy(() => import('./Rooms/Rooms'));
const Searches = lazy(() => import('./Search/Searches'));
const ShareGroups = lazy(() => import('./ShareGroups/ShareGroups'));
const SharedWithMe = lazy(() => import('./Shares/SharedWithMe'));
const SolidSettings = lazy(() => import('./Solid/SolidSettings'));
const System = lazy(() => import('./System/System'));
const Transfers = lazy(() => import('./Transfers/Transfers'));
const Users = lazy(() => import('./Users/Users'));
const Wishlist = lazy(() => import('./Wishlist/Wishlist'));

const RouteMissRedirect = () => {
  const location = useLocation();

  if (typeof window !== 'undefined') {
    window.routeMissPath = location.pathname;
  }

  useEffect(() => {
    if (typeof window === 'undefined') return undefined;

    const timeout = window.setTimeout(() => {
      const element = document.querySelector('[data-testid="route-miss"]');
      if (element) {
        window.routeMissElement = element.textContent;
      }
    }, 100);

    return () => window.clearTimeout(timeout);
  }, [location.pathname]);

  console.error('[Router] Route miss for:', location.pathname);

  return (
    <>
      <div
        data-testid="route-miss"
        style={{
          background: 'red',
          color: 'white',
          left: 0,
          padding: '20px',
          position: 'fixed',
          top: 0,
          zIndex: 9_999,
        }}
      >
        Route miss: {location.pathname}
      </div>
      <Navigate replace to="/searches" />
    </>
  );
};

const AppRouteTable = ({
  applicationOptions,
  applicationState,
  isAgent,
  isLegacyProfile,
  runtimeProfile,
  semanticTheme,
  withTokenCheck,
}) => (
isAgent ? (
                  <Routes>
                  <Route
                    path="/system"
                    element={
                      withTokenCheck(
                        <System
                          options={applicationOptions}
                          state={applicationState}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/system/:tab"
                    element={
                      withTokenCheck(
                        <System
                          options={applicationOptions}
                          state={applicationState}
                        />,
                      )
                    }
                  />
                  <Route
                    path="*"
                    element={<Navigate replace to="/system" />}
                  />
                  </Routes>
                  ) : (
                  <Routes>
                  <Route
                    path="/"
                    element={
                      <Navigate
                        replace
                        to={isLegacyProfile ? '/dashboard' : '/searches'}
                      />
                    }
                  />
                  <Route
                    path="/dashboard"
                    element={
                      isLegacyProfile ? (
                        withTokenCheck(
                          <CompatibilityDashboard
                            runtimeProfile={runtimeProfile}
                            server={applicationState.server}
                          />,
                        )
                      ) : (
                        <Navigate replace to="/searches" />
                      )
                    }
                  />
                  <Route
                    path="/lidarr"
                    element={<Navigate replace to="/system/integrations" />}
                  />
                  <Route
                    path="/collections"
                    element={(() => {
                      // This should log if route matches
                      if (typeof window !== 'undefined') {
                        window.routeMatchedCollections = true;
                        console.log(
                          '[Router] /collections route matched!',
                          '/collections',
                        );
                      }

                      try {
                        const result = withTokenCheck(
                          <div className="view">
                            <Collections />
                          </div>,
                        );
                        console.log(
                          '[Router] Collections rendered successfully',
                        );
                        return result;
                      } catch (renderError) {
                        console.error(
                          '[Router] Error rendering Collections:',
                          renderError,
                        );
                        // Return error UI instead of crashing
                        return (
                          <div className="view">
                            <ErrorSegment
                              caption={`Error loading Collections: ${renderError.message}`}
                            />
                          </div>
                        );
                      }
                    })()}
                  />
                  <Route
                    path="/solid"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <SolidSettings />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/discovery-graph"
                    element={
                      withTokenCheck(
                        <DiscoveryGraphAtlasPage
                          server={applicationState.server}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/playlist-intake"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <PlaylistIntake />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/searches"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <Searches
                            runtimeProfile={runtimeProfile}
                            server={applicationState.server}
                          />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/searches/:id"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <Searches
                            runtimeProfile={runtimeProfile}
                            server={applicationState.server}
                          />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/wishlist"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <Wishlist />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/browse"
                    element={withTokenCheck(
                      <Browse runtimeProfile={runtimeProfile} />,
                    )}
                  />
                  <Route
                    path="/users"
                    element={withTokenCheck(<Users />)}
                  />
                  <Route
                    path="/contacts"
                    element={withTokenCheck(<Contacts />)}
                  />
                  <Route
                    path="/sharegroups"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <ShareGroups />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/shared"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <SharedWithMe />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/chat"
                    element={
                      withTokenCheck(
                        isLegacyProfile ? (
                          <Chat
                            runtimeProfile={runtimeProfile}
                            state={applicationState}
                          />
                        ) : (
                          <Messaging
                            initialKind="chat"
                            state={applicationState}
                          />
                        ),
                      )
                    }
                  />
                  <Route
                    path="/pods"
                    element={
                      withTokenCheck(
                          <Messaging
                            runtimeProfile={runtimeProfile}
                            initialKind="pod"
                          state={applicationState}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/pods/:podId"
                    element={<Navigate replace to="/messages" />}
                  />
                  <Route
                    path="/pods/:podId/channels/:channelId"
                    element={<Navigate replace to="/messages" />}
                  />
                  <Route
                    path="/rooms"
                    element={
                      withTokenCheck(
                        isLegacyProfile ? (
                          <Rooms runtimeProfile={runtimeProfile} />
                        ) : (
                          <Messaging
                            runtimeProfile={runtimeProfile}
                            initialKind="room"
                            state={applicationState}
                          />
                        ),
                      )
                    }
                  />
                  <Route
                    path="/messages"
                    element={
                      isLegacyProfile ? (
                        <Navigate replace to="/chat" />
                      ) : withTokenCheck(
                        <Messaging
                          runtimeProfile={runtimeProfile}
                          initialKind="mixed"
                          state={applicationState}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/uploads"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <Transfers
                            runtimeProfile={runtimeProfile}
                            direction="upload"
                          />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/downloads"
                    element={
                      withTokenCheck(
                        <div className="view">
                          <Transfers
                            runtimeProfile={runtimeProfile}
                            direction="download"
                            server={applicationState.server}
                          />
                        </div>,
                      )
                    }
                  />
                  <Route
                    path="/system"
                    element={
                      withTokenCheck(
                        <System
                          runtimeProfile={runtimeProfile}
                          options={applicationOptions}
                          state={applicationState}
                          theme={semanticTheme}
                        />,
                      )
                    }
                  />
                  <Route
                    path="/system/:tab"
                    element={
                      withTokenCheck(
                        <System
                          runtimeProfile={runtimeProfile}
                          options={applicationOptions}
                          state={applicationState}
                          theme={semanticTheme}
                        />,
                      )
                    }
                  />
                  <Route
                    path="*"
                    element={<RouteMissRedirect />}
                  />
                  </Routes>
                  )
);

export default AppRouteTable;
