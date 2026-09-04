import './TrafficTicker.css';
import { createTransfersHubConnection } from '../../lib/hubFactory';
import { toDisplayError } from '../../lib/errors';
import React, { useEffect, useState } from 'react';
import { Icon, List, Popup } from 'semantic-ui-react';

const MAX_ACTIVITIES = 50; // Keep last 50 activities

const normalizeActivity = (activity) => {
  if (!activity || typeof activity !== 'object' || Array.isArray(activity)) {
    return null;
  }

  const numeric = (value, fallback = 0) => {
    const number = Number(value);
    return Number.isFinite(number) ? number : fallback;
  };

  return {
    ...activity,
    averageSpeed: Math.max(0, numeric(activity.averageSpeed)),
    direction: activity.direction === 'Upload' ? 'Upload' : 'Download',
    filename: typeof activity.filename === 'string' ? activity.filename : 'unknown file',
    percentComplete: Math.max(0, Math.min(100, numeric(activity.percentComplete))),
    size: Math.max(0, numeric(activity.size)),
    state: typeof activity.state === 'string' ? activity.state : 'Unknown',
    timestamp: activity.timestamp || new Date().toISOString(),
    username: typeof activity.username === 'string' ? activity.username : 'unknown user',
  };
};

const getActivityIcon = (direction, state) => {
  state = typeof state === 'string' ? state : '';
  const isCompleted = state.includes('Completed');
  const isFailed =
    state.includes('Errored') ||
    state.includes('Cancelled') ||
    state.includes('TimedOut') ||
    state.includes('Rejected');

  if (direction === 'Download') {
    if (isCompleted) {
      return isFailed ? 'x' : 'checkmark';
    }

    return 'download';
  } else {
    if (isCompleted) {
      return isFailed ? 'x' : 'checkmark';
    }

    return 'upload';
  }
};

const getActivityColor = (direction, state) => {
  state = typeof state === 'string' ? state : '';
  const isCompleted = state.includes('Completed');
  const isFailed =
    state.includes('Errored') ||
    state.includes('Cancelled') ||
    state.includes('TimedOut') ||
    state.includes('Rejected');

  if (isFailed) return 'red';
  if (isCompleted) return 'green';
  if (direction === 'Download') return 'blue';
  return 'teal';
};

const formatActivity = (activity) => {
  const {
    averageSpeed,
    direction,
    filename,
    percentComplete,
    size,
    state,
    username,
  } = activity;
  const fileName = (typeof filename === 'string' ? filename : 'unknown file')
    .split('\\').pop().split('/').pop();
  const safeState = typeof state === 'string' ? state : 'Unknown';
  const safeDirection = direction === 'Download' ? 'Download' : 'Upload';
  const safeUsername = typeof username === 'string' ? username : 'unknown user';

  if (safeState.includes('Completed')) {
    const speedText =
      averageSpeed > 0 ? ` @ ${formatBytes(averageSpeed)}/s` : '';
    return `${safeDirection === 'Download' ? '↓' : '↑'} ${safeUsername}/${fileName} ${safeState}${speedText}`;
  }

  if (safeState.includes('InProgress') && percentComplete > 0) {
    return `${safeDirection === 'Download' ? '↓' : '↑'} ${safeUsername}/${fileName} ${percentComplete.toFixed(1)}%`;
  }

  return `${safeDirection === 'Download' ? '↓' : '↑'} ${safeUsername}/${fileName} ${safeState}`;
};

const formatBytes = (bytes) => {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
  const k = 1_024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(sizes.length - 1, Math.floor(Math.log(bytes) / Math.log(k)));
  return (
    Number.parseFloat((bytes / k ** index).toFixed(1)) + ' ' + sizes[index]
  );
};

const TrafficTicker = () => {
  const [activities, setActivities] = useState([]);
  const [connected, setConnected] = useState(false);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    const transfersHub = createTransfersHubConnection();
    let active = true;

    transfersHub.on('activity', (activity) => {
      const normalizedActivity = normalizeActivity(activity);
      if (!active || !normalizedActivity) return;
      setActivities((previous) => [
        normalizedActivity,
        ...previous.slice(0, MAX_ACTIVITIES - 1),
      ]);
    });

    transfersHub.onreconnecting(() => setConnected(false));
    transfersHub.onreconnected(() => setConnected(true));
    transfersHub.onclose(() => setConnected(false));

    transfersHub.start()
      .then(() => {
        if (active) setConnected(true);
      })
      .catch((error) => {
        if (active) {
          console.warn('Transfer activity feed failed to start:', toDisplayError(error));
          setConnected(false);
        }
      });

    return () => {
      active = false;
      transfersHub.stop().catch(() => {});
    };
  }, []);

  const visibleActivities = expanded ? activities : activities.slice(0, 10);
  const hasMore = activities.length > 10;

  return (
    <div className="traffic-ticker">
      <div className="traffic-ticker-header">
        <Icon name="exchange" />
        <span>Transfer Activity</span>
        <Icon
          color={connected ? 'green' : 'red'}
          name={connected ? 'wifi' : 'remove'}
          size="small"
          title={
            connected
              ? 'Connected to transfer feed'
              : 'Disconnected from transfer feed'
          }
        />
      </div>

      <div className="traffic-ticker-content">
        {activities.length === 0 ? (
          <div className="traffic-ticker-empty">
            <Icon name="clock outline" />
            <span>No recent activity</span>
          </div>
        ) : (
          <List
            divided
            relaxed
          >
            {visibleActivities.map((activity, index) => (
              <List.Item key={`${activity.timestamp}-${index}`}>
                <List.Content>
                  <div className="traffic-activity">
                    <Icon
                      color={getActivityColor(
                        activity.direction,
                        activity.state,
                      )}
                      name={getActivityIcon(activity.direction, activity.state)}
                      size="small"
                    />
                    <Popup
                      content={`${new Date(activity.timestamp).toLocaleTimeString()} - ${activity.direction} ${activity.state}`}
                      position="top left"
                      trigger={
                        <span className="traffic-activity-text">
                          {formatActivity(activity)}
                        </span>
                      }
                    />
                  </div>
                </List.Content>
              </List.Item>
            ))}
          </List>
        )}

        {hasMore && (
          <div className="traffic-ticker-toggle">
            <button
              className="traffic-ticker-toggle-button"
              onClick={() => setExpanded(!expanded)}
            >
              {expanded ? 'Show Less' : `Show ${activities.length - 10} More`}
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default TrafficTicker;
