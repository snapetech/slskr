import React from 'react';
import { toDisplayError } from '../../lib/errors';
import { Header, Icon, Segment } from 'semantic-ui-react';

const ErrorSegment = ({ caption, error, icon = 'x', suppressPrefix = false }) => {
  const rawCaption = caption ?? error;
  const displayCaption = React.isValidElement(rawCaption)
    || typeof rawCaption === 'string'
    || typeof rawCaption === 'number'
    ? rawCaption
    : toDisplayError(rawCaption);

  return (
    <Segment
      basic
      className="error-segment"
      placeholder
    >
      <Header icon>
        <Icon
          color="red"
          name={icon}
        />
        {!suppressPrefix && 'Error: '}
        {displayCaption}
      </Header>
    </Segment>
  );
};

export default ErrorSegment;
