import { Icon, Item } from 'semantic-ui-react';

const ImagePlaceholder = () => (
  <div className="users-picture-placeholder ui small image">
    <Icon
      name="camera"
      size="big"
    />
  </div>
);

const Presence = ({ presence }) => {
  const label = typeof presence === 'string' && presence.trim() ? presence : 'Unknown';
  const colors = {
    Away: 'yellow',
    Online: 'green',
  };

  return (
    <span className="users-presence" aria-label={`Presence: ${label}`}>
      <Icon
        aria-hidden="true"
        color={colors[label] || 'grey'}
        name="circle"
      />
      <span>{label}</span>
    </span>
  );
};

const FreeUploadSlot = ({ hasFreeUploadSlot }) => {
  if (typeof hasFreeUploadSlot !== 'boolean') {
    return <span className="users-free-upload-slot users-free-upload-slot-unknown">Unknown</span>;
  }

  const available = hasFreeUploadSlot;
  return (
    <span className="users-free-upload-slot">
      <Icon
        aria-hidden="true"
        color={available ? 'green' : 'red'}
        name={available ? 'check' : 'close'}
      />
      {available ? 'Available' : 'Unavailable'}
    </span>
  );
};

const User = ({
  address,
  description,
  hasFreeUploadSlot,
  hasPicture,
  picture,
  port,
  presence,
  queueLength,
  uploadSlots,
  username,
}) => {
  const details = [
    ['Upload slots', uploadSlots],
    ['Queue length', queueLength],
    ['IP address', address],
    ['Port', port],
  ].filter(([, value]) => value != null && String(value).trim() && String(value) !== 'Unknown');

  return (
    <Item>
      {hasPicture ? (
        <Item.Image
          size="small"
          src={`data:image;base64,${picture}`}
        />
      ) : (
        <ImagePlaceholder />
      )}

      <Item.Content>
        <Item.Header as="h2">{username}</Item.Header>
        <Item.Meta as="div">
          <div className="users-profile-status">
            <span className="users-profile-status-item">
              <span className="users-profile-status-label">Presence</span>
              <Presence presence={presence} />
            </span>
            <span className="users-profile-status-item">
              <span className="users-profile-status-label">Free upload slot</span>
              <FreeUploadSlot hasFreeUploadSlot={hasFreeUploadSlot} />
            </span>
          </div>
        </Item.Meta>
        {details.length > 0 ? (
          <dl className="users-network-details">
            {details.map(([label, value]) => (
              <div className="users-network-detail" key={label}>
                <dt>{label}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
        ) : (
          <p className="users-network-details-empty">Live network details are currently unavailable.</p>
        )}
        <Item.Description>{description || 'No profile description provided.'}</Item.Description>
      </Item.Content>
    </Item>
  );
};

export default User;
