import './System.css';
import AdminPolicies from './AdminPolicies';
import { Switch } from '../Shared';
import AutomationCenter from './AutomationCenter';
import Bridge from './Bridge';
import Data from './Data';
import Events from './Events';
import ExperienceSettings from './ExperienceSettings';
import Files from './Files';
import Info from './Info';
import Integrations from './Integrations';
import Jobs from './Jobs';
import LibraryHealth from './LibraryHealth';
import Logs from './Logs';
import MediaCore from './MediaCore';
import Mesh from './Mesh';
import Metrics from './Metrics';
import Network from './Network';
import Options from './Options';
import QuarantineJury from './QuarantineJury';
import Security from './Security';
import Shares from './Shares';
import SourceProviders from './SourceProviders';
import SwarmAnalytics from './SwarmAnalytics';
import React, { useEffect, useRef } from 'react';
import { Navigate, useNavigate, useParams } from 'react-router-dom';
import { Icon, Menu, Segment, Tab } from 'semantic-ui-react';

// Six named groups instead of one 22-item flat tab strip. Order here is the
// order sections appear in; a pane's `section` field below assigns it to one.
const SECTIONS = [
  { icon: 'home', key: 'overview', title: 'Overview' },
  { icon: 'share alternate', key: 'network', title: 'Network & Mesh' },
  { icon: 'shield alternate', key: 'security', title: 'Security & Trust' },
  { icon: 'magic', key: 'automation', title: 'Automation & Jobs' },
  { icon: 'heartbeat', key: 'diagnostics', title: 'Diagnostics' },
  { icon: 'options', key: 'advanced', title: 'Advanced' },
];

const LEGACY_SYSTEM_ROUTES = [
  'info',
  'options',
  'shares',
  'files',
  'data',
  'events',
  'logs',
];

const System = ({ runtimeProfile, options = {}, state = {}, theme }) => {
  const navigate = useNavigate();
  const { tab } = useParams();
  const systemRef = useRef(null);

  const panes = [
    {
      menuItem: (
        <Menu.Item key="info">
          <Switch
            pending={
              ((state?.pendingRestart ?? false) ||
                (state?.pendingReconnect ?? false)) && (
                <Icon
                  color="yellow"
                  name="exclamation circle"
                />
              )
            }
          >
            <Icon name="info circle" />
          </Switch>
          Info
        </Menu.Item>
      ),
      render: () => (
        <Tab.Pane>
          <Info
            runtimeProfile={runtimeProfile}
            options={options}
            state={state}
            theme={theme}
          />
        </Tab.Pane>
      ),
      route: 'info',
      section: 'overview',
    },
    {
      menuItem: (
        <Menu.Item key="network">
          <Icon
            color="blue"
            name="sitemap"
          />
          Network
        </Menu.Item>
      ),
      render: () => (
        <Tab.Pane>
          <Network
            options={options}
            state={state}
            theme={theme}
          />
        </Tab.Pane>
      ),
      route: 'network',
      section: 'overview',
    },
    {
      menuItem: (
        <Menu.Item key="shares">
          <Switch
            scanPending={
              (state?.shares?.scanPending ?? false) && (
                <Icon
                  color="yellow"
                  name="exclamation circle"
                />
              )
            }
          >
            <Icon name="share external" />
          </Switch>
          Shares
        </Menu.Item>
      ),
      render: () => (
        <Tab.Pane>
          <Shares
            state={state.shares}
            theme={theme}
          />
        </Tab.Pane>
      ),
      route: 'shares',
      section: 'overview',
    },
    {
      menuItem: {
        content: 'Mesh',
        icon: 'share alternate',
        key: 'mesh',
      },
      render: () => (
        <Tab.Pane>
          <Mesh runtimeProfile={runtimeProfile} />
        </Tab.Pane>
      ),
      route: 'mesh',
      section: 'network',
    },
    {
      menuItem: {
        content: 'Bridge',
        icon: 'exchange',
        key: 'bridge',
      },
      render: () => (
        <Tab.Pane>
          <Bridge />
        </Tab.Pane>
      ),
      route: 'bridge',
      section: 'network',
    },
    {
      menuItem: {
        content: 'MediaCore',
        icon: 'music',
        key: 'mediacore',
      },
      render: () => (
        <Tab.Pane>
          <MediaCore />
        </Tab.Pane>
      ),
      route: 'mediacore',
      section: 'network',
    },
    {
      menuItem: {
        content: 'Source Providers',
        icon: 'random',
        key: 'source-providers',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <SourceProviders />
        </Tab.Pane>
      ),
      route: 'source-providers',
      section: 'network',
    },
    {
      menuItem: {
        content: 'Swarm Analytics',
        icon: 'chart line',
        key: 'swarm-analytics',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <SwarmAnalytics />
        </Tab.Pane>
      ),
      route: 'swarm-analytics',
      section: 'network',
    },
    {
      menuItem: {
        content: 'Security',
        icon: 'shield alternate',
        key: 'security',
      },
      render: () => (
        <Tab.Pane>
          <Security />
        </Tab.Pane>
      ),
      route: 'security',
      section: 'security',
    },
    {
      menuItem: {
        content: 'Policies',
        icon: 'sliders horizontal',
        key: 'policies',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <AdminPolicies options={options} />
        </Tab.Pane>
      ),
      route: 'policies',
      section: 'security',
    },
    {
      menuItem: {
        content: 'Quarantine Jury',
        icon: 'shield',
        key: 'quarantine-jury',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <QuarantineJury />
        </Tab.Pane>
      ),
      route: 'quarantine-jury',
      section: 'security',
    },
    {
      menuItem: {
        content: 'Jobs',
        icon: 'tasks',
        key: 'jobs',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <Jobs />
        </Tab.Pane>
      ),
      route: 'jobs',
      section: 'automation',
    },
    {
      menuItem: {
        content: 'Automations',
        icon: 'magic',
        key: 'automations',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <AutomationCenter />
        </Tab.Pane>
      ),
      route: 'automations',
      section: 'automation',
    },
    {
      menuItem: {
        content: 'Events',
        icon: 'calendar check',
        key: 'events',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <Events />
        </Tab.Pane>
      ),
      route: 'events',
      section: 'automation',
    },
    {
      menuItem: {
        content: 'Library Health',
        icon: 'heartbeat',
        key: 'library-health',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <LibraryHealth />
        </Tab.Pane>
      ),
      route: 'library-health',
      section: 'diagnostics',
    },
    {
      menuItem: {
        content: 'Files',
        icon: 'folder open',
        key: 'files',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <Files
            options={options}
            theme={theme}
          />
        </Tab.Pane>
      ),
      route: 'files',
      section: 'diagnostics',
    },
    {
      menuItem: {
        content: 'Data',
        icon: 'database',
        key: 'data',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <Data theme={theme} />
        </Tab.Pane>
      ),
      route: 'data',
      section: 'diagnostics',
    },
    {
      menuItem: {
        content: 'Logs',
        icon: 'file outline',
        key: 'logs',
      },
      render: () => (
        <Tab.Pane>
          <Logs />
        </Tab.Pane>
      ),
      route: 'logs',
      section: 'diagnostics',
    },
    {
      menuItem: {
        content: 'Metrics',
        icon: 'chart bar',
        key: 'metrics',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <Metrics />
        </Tab.Pane>
      ),
      route: 'metrics',
      section: 'diagnostics',
    },
    {
      menuItem: {
        content: 'Experience',
        icon: 'compass',
        key: 'experience',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <ExperienceSettings />
        </Tab.Pane>
      ),
      route: 'experience',
      section: 'advanced',
    },
    {
      menuItem: {
        content: 'Integrations',
        icon: 'plug',
        key: 'integrations',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <Integrations
            options={options}
            state={state}
          />
        </Tab.Pane>
      ),
      route: 'integrations',
      section: 'advanced',
    },
    {
      menuItem: {
        content: 'Options',
        icon: 'options',
        key: 'options',
      },
      render: () => (
        <Tab.Pane className="full-height">
          <Options
            options={options}
            theme={theme}
          />
        </Tab.Pane>
      ),
      route: 'options',
      section: 'advanced',
    },
  ];

  // The two controller profiles have established flat System tab surfaces.
  // Keep the grouped layout for the general slskR surface, but preserve the
  // target profile contracts so a drop-in replacement does not add a second
  // navigation layer or change the visible control inventory.
  const profilePanes = runtimeProfile === 'legacy'
    ? panes.filter((pane) => LEGACY_SYSTEM_ROUTES.includes(pane.route))
    : panes;

  const activeIndex = panes.findIndex((pane) => pane.route === tab);

  useEffect(() => {
    const activeItem = systemRef.current?.querySelector(
      '.ui.tabular.menu .active.item',
    );
    activeItem?.scrollIntoView?.({ block: 'nearest', inline: 'nearest' });
  }, [tab]);

  useEffect(() => {
    if (!runtimeProfile) {
      return;
    }

    systemRef.current
      ?.querySelectorAll('.ui.tabular.menu [role="button"]')
      .forEach((item) => item.setAttribute('role', 'tab'));
  }, [tab, runtimeProfile]);

  if (tab === undefined) {
    return <Navigate replace to={`/system/${profilePanes[0].route}`} />;
  }

  if (runtimeProfile === 'legacy' || runtimeProfile === 'native') {
    const profileActiveIndex = profilePanes.findIndex(
      (pane) => pane.route === tab,
    );
    const activeProfileIndex = profileActiveIndex > -1
      ? profileActiveIndex
      : 0;

    return (
      <div className="system" ref={systemRef}>
        <Segment raised>
          <Tab
            activeIndex={activeProfileIndex}
            onTabChange={(_event, { activeIndex: newIndex }) =>
              navigate(`/system/${profilePanes[newIndex].route}`)}
            panes={profilePanes}
            renderActiveOnly
          />
        </Segment>
      </div>
    );
  }

  const activePane = activeIndex > -1 ? panes[activeIndex] : panes[0];
  const sectionPanes = panes.filter(
    (pane) => pane.section === activePane.section,
  );
  const sectionActiveIndex = sectionPanes.findIndex(
    (pane) => pane.route === activePane.route,
  );

  const onSectionTabChange = (_event, { activeIndex: newIndex }) => {
    navigate(`/system/${sectionPanes[newIndex].route}`);
  };

  const onSectionSelect = (sectionKey) => {
    const firstRoute = panes.find((pane) => pane.section === sectionKey)?.route;
    if (firstRoute) {
      navigate(`/system/${firstRoute}`);
    }
  };

  return (
    <div className="system" ref={systemRef}>
      <Menu
        className="system-section-menu"
        pointing
        secondary
      >
        {SECTIONS.map((section) => (
          <Menu.Item
            active={activePane.section === section.key}
            key={section.key}
            onClick={() => onSectionSelect(section.key)}
          >
            <Icon name={section.icon} />
            {section.title}
          </Menu.Item>
        ))}
      </Menu>
      <Segment raised>
        <Tab
          activeIndex={sectionActiveIndex > -1 ? sectionActiveIndex : 0}
          onTabChange={onSectionTabChange}
          panes={sectionPanes}
          renderActiveOnly={Boolean(runtimeProfile)}
        />
      </Segment>
    </div>
  );
};

export default System;
