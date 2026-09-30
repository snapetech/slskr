import { NavLink } from 'react-router-dom';
import { Dropdown, Icon, Menu } from 'semantic-ui-react';

const NavigationIcon = ({ alert, alertTestId, name }) => (
  <span className="navigation-alert-icon">
    <Icon name={name} />
    {alert && (
      <span
        aria-label="New activity"
        className="navigation-alert-dot"
        data-testid={alertTestId}
        role="status"
      />
    )}
  </span>
);

const AppNavigationPrimary = ({
  isAgent,
  isLegacyProfile,
  isNativeProfile,
  navActivity,
  version,
}) => (
              <div className="navigation-primary">
                {version.isCanary && (
                  <Menu.Item>
                    <Icon
                      color="yellow"
                      name="flask"
                    />
                    Canary
                  </Menu.Item>
                )}
              {isAgent ? (
                <Menu.Item>
                  <Icon name="detective" />
                  Agent Mode
                </Menu.Item>
              ) : (
                isLegacyProfile ? (
                <>
                  <NavLink to="/dashboard">
                    <Menu.Item data-testid="nav-dashboard">
                      <Icon name="chart bar" />
                      Dashboard
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/searches">
                    <Menu.Item data-testid="nav-search">
                      <Icon name="search" />
                      Search
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/downloads">
                    <Menu.Item data-testid="nav-downloads">
                      <Icon name="download" />
                      Downloads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/uploads">
                    <Menu.Item data-testid="nav-uploads">
                      <Icon name="upload" />
                      Uploads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/rooms">
                    <Menu.Item data-testid="nav-rooms">
                      <NavigationIcon
                        alert={navActivity.rooms}
                        alertTestId="nav-rooms-alert"
                        name="comments"
                      />
                      Rooms
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/chat">
                    <Menu.Item data-testid="nav-chat">
                      <NavigationIcon
                        alert={navActivity.chat}
                        alertTestId="nav-chat-alert"
                        name="comment"
                      />
                      Chat
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/users">
                    <Menu.Item data-testid="nav-users">
                      <Icon name="users" />
                      Users
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/browse">
                    <Menu.Item data-testid="nav-browse">
                      <Icon name="folder open" />
                      Browse
                    </Menu.Item>
                  </NavLink>
                </>
                ) : isNativeProfile ? (
                <>
                  <NavLink to="/searches">
                    <Menu.Item data-testid="nav-search">
                      <Icon name="search" />
                      Search
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/discovery-graph">
                    <Menu.Item data-testid="nav-discovery-graph">
                      <Icon name="crosshairs" />
                      Discovery Graph
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/playlist-intake">
                    <Menu.Item data-testid="nav-playlist-intake">
                      <Icon name="list alternate outline" />
                      Playlist Intake
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/wishlist">
                    <Menu.Item data-testid="nav-wishlist">
                      <Icon name="star" />
                      Wishlist
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/lidarr">
                    <Menu.Item data-testid="nav-lidarr">
                      <Icon name="music" />
                      Lidarr
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/downloads">
                    <Menu.Item data-testid="nav-downloads">
                      <Icon name="download" />
                      Downloads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/uploads">
                    <Menu.Item data-testid="nav-uploads">
                      <Icon name="upload" />
                      Uploads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/messages">
                    <Menu.Item data-testid="nav-messages">
                      <NavigationIcon
                        alert={navActivity.rooms || navActivity.chat}
                        alertTestId={
                          navActivity.chat ? 'nav-chat-alert' : 'nav-rooms-alert'
                        }
                        name="comments"
                      />
                      Messages
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/users">
                    <Menu.Item data-testid="nav-users">
                      <Icon name="users" />
                      Users
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/contacts">
                    <Menu.Item data-testid="nav-contacts">
                      <Icon name="address book" />
                      Contacts
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/solid">
                    <Menu.Item data-testid="nav-solid">
                      <Icon name="key" />
                      Solid
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/collections">
                    <Menu.Item data-testid="nav-collections">
                      <Icon name="list" />
                      Collections
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/sharegroups">
                    <Menu.Item data-testid="nav-groups">
                      <Icon name="users" />
                      Share Groups
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/shared">
                    <Menu.Item data-testid="nav-shared-with-me">
                      <Icon name="share" />
                      Shared with Me
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/browse">
                    <Menu.Item data-testid="nav-browse">
                      <Icon name="folder open" />
                      Browse
                    </Menu.Item>
                  </NavLink>
                </>
                ) : (
                <>
                  <NavLink to="/searches">
                    <Menu.Item data-testid="nav-search">
                      <Icon name="search" />
                      Search
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/wishlist">
                    <Menu.Item data-testid="nav-wishlist">
                      <Icon name="star" />
                      Wishlist
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/downloads">
                    <Menu.Item data-testid="nav-downloads">
                      <Icon name="download" />
                      Downloads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/uploads">
                    <Menu.Item data-testid="nav-uploads">
                      <Icon name="upload" />
                      Uploads
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/messages">
                    <Menu.Item data-testid="nav-messages">
                      <NavigationIcon
                        alert={navActivity.rooms || navActivity.chat}
                        alertTestId={
                          navActivity.chat ? 'nav-chat-alert' : 'nav-rooms-alert'
                        }
                        name="comments"
                      />
                      Messages
                    </Menu.Item>
                  </NavLink>
                  <NavLink to="/users">
                    <Menu.Item data-testid="nav-users">
                      <Icon name="users" />
                      Users
                    </Menu.Item>
                  </NavLink>
                  <Dropdown
                    className="navigation-more"
                    data-testid="nav-more"
                    icon={null}
                    item
                    trigger={(
                      <span className="navigation-more-trigger">
                        <Icon name="ellipsis horizontal" />
                        More
                      </span>
                    )}
                  >
                    <Dropdown.Menu>
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-discovery-graph"
                        icon="crosshairs"
                        text="Discovery Graph"
                        to="/discovery-graph"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-playlist-intake"
                        icon="list alternate outline"
                        text="Playlist Intake"
                        to="/playlist-intake"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-contacts"
                        icon="address book"
                        text="Contacts"
                        to="/contacts"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-solid"
                        icon="key"
                        text="Solid"
                        to="/solid"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-collections"
                        icon="list"
                        text="Collections"
                        to="/collections"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-groups"
                        icon="users"
                        text="Share Groups"
                        to="/sharegroups"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-shared-with-me"
                        icon="share"
                        text="Shared with Me"
                        to="/shared"
                      />
                      <Dropdown.Item
                        as={NavLink}
                        data-testid="nav-browse"
                        icon="folder open"
                        text="Browse"
                        to="/browse"
                      />
                    </Dropdown.Menu>
                  </Dropdown>
                </>
                )
              )}
            </div>
);

export default AppNavigationPrimary;
