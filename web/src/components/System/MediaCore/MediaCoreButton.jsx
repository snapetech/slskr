import React, { useRef } from 'react';
import { Button as SemanticButton, Popup } from 'semantic-ui-react';

const getButtonText = (children) => {
  if (typeof children === 'string') {
    return children;
  }

  if (Array.isArray(children)) {
    return children.filter((child) => typeof child === 'string').join(' ').trim();
  }

  return '';
};

const Button = ({
  'aria-label': ariaLabel,
  children,
  onClick,
  title,
  tooltip,
  ...props
}) => {
  const inFlightRef = useRef(false);
  const label = ariaLabel || title || getButtonText(children) || undefined;
  const handleClick = (...args) => {
    if (typeof onClick !== 'function' || inFlightRef.current) {
      return undefined;
    }

    inFlightRef.current = true;
    try {
      const result = onClick(...args);
      if (result && typeof result.then === 'function') {
        return Promise.resolve(result).finally(() => {
          inFlightRef.current = false;
        });
      }
      inFlightRef.current = false;
      return result;
    } catch (error) {
      inFlightRef.current = false;
      throw error;
    }
  };
  const button = (
    <SemanticButton
      aria-label={ariaLabel || label}
      onClick={onClick ? handleClick : undefined}
      title={title}
      {...props}
    >
      {children}
    </SemanticButton>
  );
  const content = tooltip || title || label;

  if (!content) {
    return button;
  }

  return (
    <Popup
      content={content}
      trigger={button}
    />
  );
};

Button.Group = SemanticButton.Group;
Button.Or = SemanticButton.Or;

export default Button;
