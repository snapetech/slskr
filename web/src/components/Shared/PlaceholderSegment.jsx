import React from 'react';
import { Header, Icon, Segment } from 'semantic-ui-react';

const PlaceholderSegment = ({
  caption,
  headingAs = 'h3',
  icon,
  size,
  ...rest
}) => {
  const className =
    size === 'small' ? 'placeholder-segment-small' : 'placeholder-segment';

  return (
    <Segment
      basic
      className={className}
      placeholder
      {...rest}
    >
      <Header
        as={headingAs}
        icon
      >
        <Icon name={icon} />
        {caption}
      </Header>
    </Segment>
  );
};

export default PlaceholderSegment;
