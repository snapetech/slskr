import React from 'react';
import { createPortal } from 'react-dom';

const numberWords = {
  1: 'one',
  2: 'two',
  3: 'three',
  4: 'four',
  5: 'five',
  6: 'six',
  7: 'seven',
  8: 'eight',
  9: 'nine',
  10: 'ten',
  11: 'eleven',
  12: 'twelve',
  13: 'thirteen',
  14: 'fourteen',
  15: 'fifteen',
  16: 'sixteen',
};

const semanticOnlyProps = new Set([
  'action',
  'actionPosition',
  'active',
  'activeIndex',
  'allowAdditions',
  'as',
  'attached',
  'basic',
  'borderless',
  'button',
  'cancelButton',
  'celled',
  'centered',
  'children',
  'circular',
  'clearing',
  'closeIcon',
  'closeOnDimmerClick',
  'closeOnDocumentClick',
  'closeOnEscape',
  'collapsing',
  'color',
  'columns',
  'compact',
  'confirmButton',
  'content',
  'defaultActiveIndex',
  'description',
  'definition',
  'disabled',
  'dimmer',
  'direction',
  'divided',
  'error',
  'extra',
  'fitted',
  'fixed',
  'floated',
  'fluid',
  'header',
  'hidden',
  'horizontal',
  'icon',
  'image',
  'info',
  'inline',
  'input',
  'inverted',
  'items',
  'itemsPerRow',
  'label',
  'labelPosition',
  'loading',
  'menu',
  'menuItem',
  'meta',
  'mountNode',
  'negative',
  'onDismiss',
  'onClose',
  'onOpen',
  'options',
  'padded',
  'panes',
  'placeholder',
  'pointing',
  'position',
  'positive',
  'primary',
  'progress',
  'raised',
  'relaxed',
  'render',
  'renderActiveOnly',
  'required',
  'scrolling',
  'search',
  'secondary',
  'selectable',
  'selection',
  'simple',
  'singleLine',
  'size',
  'sorted',
  'stackable',
  'structured',
  'success',
  'tabular',
  'tertiary',
  'text',
  'textAlign',
  'toggle',
  'trigger',
  'upward',
  'unstackable',
  'vertical',
  'verticalAlign',
  'visible',
  'warning',
  'width',
  'widths',
]);

const booleanClassProps = {
  active: 'active',
  basic: 'basic',
  borderless: 'borderless',
  celled: 'celled',
  centered: 'centered',
  circular: 'circular',
  clearing: 'clearing',
  collapsing: 'collapsing',
  compact: 'compact',
  disabled: 'disabled',
  divided: 'divided',
  error: 'error',
  definition: 'definition',
  fitted: 'fitted',
  fixed: 'fixed',
  fluid: 'fluid',
  hidden: 'hidden',
  horizontal: 'horizontal',
  icon: 'icon',
  info: 'info',
  inline: 'inline',
  inverted: 'inverted',
  loading: 'loading',
  negative: 'negative',
  padded: 'padded',
  placeholder: 'placeholder',
  pointing: 'pointing',
  positive: 'positive',
  primary: 'primary',
  raised: 'raised',
  relaxed: 'relaxed',
  required: 'required',
  scrolling: 'scrolling',
  secondary: 'secondary',
  selectable: 'selectable',
  selection: 'selection',
  simple: 'simple',
  stackable: 'stackable',
  structured: 'structured',
  success: 'success',
  tabular: 'tabular',
  tertiary: 'tertiary',
  text: 'text',
  toggle: 'toggle',
  unstackable: 'unstackable',
  vertical: 'vertical',
  visible: 'visible',
  warning: 'warning',
};

const passthroughInputProps = new Set([
  'accept',
  'aria-activedescendant',
  'aria-autocomplete',
  'aria-controls',
  'aria-describedby',
  'aria-expanded',
  'aria-haspopup',
  'aria-invalid',
  'aria-label',
  'aria-labelledby',
  'autoComplete',
  'autoFocus',
  'checked',
  'defaultChecked',
  'defaultValue',
  'disabled',
  'id',
  'list',
  'max',
  'maxLength',
  'min',
  'minLength',
  'multiple',
  'name',
  'onBlur',
  'onChange',
  'onFocus',
  'onInput',
  'onInvalid',
  'onKeyDown',
  'onKeyPress',
  'onKeyUp',
  'pattern',
  'placeholder',
  'readOnly',
  'required',
  'rows',
  'step',
  'title',
  'type',
  'value',
]);

const cx = (...parts) => parts
  .flat(Infinity)
  .filter((part) => part !== false && part != null && part !== '')
  .join(' ');

const toWord = (value) => numberWords[value] || value;

const childrenOrContent = (children, content) =>
  children === undefined ? content : children;

const valueClass = (value, suffix) => {
  if (value === true) return suffix;
  if (value === false || value == null) return null;
  return `${value} ${suffix}`;
};

const widthClass = (width) => {
  if (!width) return null;
  return `${toWord(width)} wide`;
};

const countClass = (value, suffix) => {
  if (!value) return null;
  return `${toWord(value)} ${suffix}`;
};

const commonClasses = (props) => [
  props.size,
  props.color,
  valueClass(props.attached, 'attached'),
  valueClass(props.floated, 'floated'),
  props.textAlign ? `${props.textAlign} aligned` : null,
  props.verticalAlign ? `${props.verticalAlign} aligned` : null,
  props.singleLine ? 'single line' : null,
  props.position,
  ...Object.entries(booleanClassProps).map(([prop, className]) =>
    props[prop] === true
      ? className
      : ['basic', 'padded', 'pointing', 'relaxed'].includes(prop) && props[prop]
        ? `${props[prop]} ${className}`
        : null),
];

const cleanProps = (props, additionalOmit = []) => {
  const omitted = new Set([...semanticOnlyProps, ...additionalOmit]);
  return Object.fromEntries(
    Object.entries(props).filter(([key, value]) =>
      value !== undefined && !omitted.has(key)),
  );
};

const cleanInputProps = (props, additionalOmit = []) => {
  const omitted = new Set(additionalOmit);
  return Object.fromEntries(
    Object.entries(props).filter(([key, value]) =>
      value !== undefined &&
      !omitted.has(key) &&
      (passthroughInputProps.has(key) ||
        key.startsWith('aria-') ||
        key.startsWith('data-'))),
  );
};

const callAll = (...handlers) => (event, data) => {
  handlers.forEach((handler) => {
    if (handler) handler(event, data);
  });
};

const cloneTrigger = (trigger, props) => {
  if (!React.isValidElement(trigger)) return trigger;
  return React.cloneElement(trigger, {
    ...props,
    onClick: callAll(trigger.props.onClick, props.onClick),
  });
};

const createComponent = (baseClass, defaultAs = 'div', classFactory = () => null) =>
  React.forwardRef((props, ref) => {
    const {
      as: Component = defaultAs,
      children,
      className,
      content,
      ...rest
    } = props;
    const componentProps = cleanProps(rest);

    return (
      <Component
        {...componentProps}
        className={cx(baseClass, commonClasses(props), classFactory(props), className)}
        ref={ref}
      >
        {childrenOrContent(children, content)}
      </Component>
    );
  });

export const Icon = React.forwardRef((props, ref) => {
  const {
    as: Component = 'i',
    children,
    className,
    content,
    name,
    ...rest
  } = props;

  return (
    <Component
      {...cleanProps(rest)}
      aria-hidden={props['aria-label'] ? undefined : true}
      className={cx(name, commonClasses(props), 'icon', className)}
      ref={ref}
    >
      {childrenOrContent(children, content)}
    </Component>
  );
});
Icon.Group = createComponent('icons', 'span');

export const ButtonGroup = createComponent('ui buttons');

export const Button = React.forwardRef((props, ref) => {
  const {
    as,
    children,
    className,
    content,
    disabled,
    href,
    icon,
    label,
    labelPosition,
    loading,
    onClick,
    type,
    ...rest
  } = props;
  const Component = as || (href ? 'a' : 'button');
  const body = childrenOrContent(children, content);
  const buttonProps = cleanProps(rest);
  const renderedIcon = typeof icon === 'string' ? <Icon name={icon} /> : null;
  const renderedLabel = label ? <Label>{label}</Label> : null;

  if (Component === 'button' && !buttonProps.type) {
    buttonProps.type = type || 'button';
  }
  if (href) buttonProps.href = href;
  if (Component === 'button') buttonProps.disabled = disabled || loading;
  if (disabled && Component !== 'button') buttonProps['aria-disabled'] = true;

  return (
    <Component
      {...buttonProps}
      className={cx(
        'ui',
        commonClasses({ ...props, disabled, loading }),
        labelPosition ? `${labelPosition} labeled` : null,
        'button',
        className,
      )}
      onClick={disabled || loading ? undefined : onClick}
      ref={ref}
    >
      {labelPosition === 'left' ? renderedLabel : null}
      {renderedIcon}
      {body}
      {labelPosition !== 'left' ? renderedLabel : null}
    </Component>
  );
});
Button.Group = ButtonGroup;
Button.Or = ({ children, className, content, text, ...props }) => (
  <div
    {...cleanProps(props)}
    className={cx('or', className)}
    data-text={text}
  >
    {childrenOrContent(children, content)}
  </div>
);

export const Label = createComponent('ui label', 'span');
Label.Detail = createComponent('detail', 'span');
Label.Group = createComponent('ui labels');

export const Header = createComponent('ui header', 'h3');
Header.Content = createComponent('content');
Header.Subheader = createComponent('sub header');

export const Segment = createComponent('ui segment');
Segment.Group = createComponent('ui segments');
Segment.Inline = createComponent('inline');

export const Divider = createComponent('ui divider', 'div');
export const Container = createComponent('ui container');

export const Grid = createComponent('ui grid', 'div', (props) => [
  countClass(props.columns, 'column'),
  props.divided ? 'divided' : null,
]);
Grid.Column = createComponent('column', 'div', (props) => widthClass(props.width));
Grid.Row = createComponent('row', 'div', (props) => countClass(props.columns, 'column'));

export const Card = createComponent('ui card');
Card.Group = createComponent('ui cards', 'div', (props) =>
  countClass(props.itemsPerRow, 'cards'));
Card.Content = createComponent('content', 'div', (props) => props.extra ? 'extra' : null);
Card.Header = createComponent('header');
Card.Meta = createComponent('meta');
Card.Description = createComponent('description');

export const List = createComponent('ui list');
List.Item = createComponent('item');
List.Content = createComponent('content');
List.Header = createComponent('header');
List.Description = createComponent('description');
List.List = createComponent('list');
List.Icon = Icon;

export const Item = createComponent('item');
Item.Group = createComponent('ui items');
Item.Content = createComponent('content');
Item.Header = createComponent('header');
Item.Meta = createComponent('meta');
Item.Description = createComponent('description');
Item.Image = React.forwardRef((props, ref) => {
  const { className, size, src, ...rest } = props;
  return (
    <img
      {...cleanProps(rest)}
      className={cx('ui', size, 'image', className)}
      ref={ref}
      src={src}
    />
  );
});

export const Message = React.forwardRef((props, ref) => {
  const {
    as: Component = 'div',
    children,
    className,
    content,
    header,
    onDismiss,
    ...rest
  } = props;

  return (
    <Component
      {...cleanProps(rest)}
      className={cx('ui message', commonClasses(props), className)}
      ref={ref}
    >
      {onDismiss ? (
        <i
          className="close icon"
          onClick={onDismiss}
          role="button"
        />
      ) : null}
      {header ? <Message.Header>{header}</Message.Header> : null}
      {childrenOrContent(children, content)}
    </Component>
  );
});
Message.Content = createComponent('content');
Message.Header = createComponent('header');
Message.List = ({ children, className, content, items = [], ...props }) => (
  <ul
    {...cleanProps(props)}
    className={cx('list', className)}
  >
    {childrenOrContent(children, content) ||
      items.map((item) => (
        <li key={typeof item === 'string' ? item : item.key || item.content || item}>
          {typeof item === 'object' && item != null ? item.content || item.text : item}
        </li>
      ))}
  </ul>
);

export const Statistic = createComponent('ui statistic');
Statistic.Group = createComponent('ui statistics', 'div', (props) =>
  props.widths ? countClass(props.widths, 'statistics') : null);
Statistic.Value = createComponent('value');
Statistic.Label = createComponent('label');

export const Table = createComponent('ui table', 'table');
Table.Header = createComponent('', 'thead');
Table.Body = createComponent('', 'tbody');
Table.Footer = createComponent('', 'tfoot');
Table.Row = createComponent('', 'tr');
Table.HeaderCell = createComponent('', 'th', (props) => [
  widthClass(props.width),
  props.sorted,
]);
Table.Cell = createComponent('', 'td', (props) => widthClass(props.width));

export const Menu = createComponent('ui menu');
Menu.Item = React.forwardRef((props, ref) => {
  const {
    as: Component = props.href ? 'a' : 'div',
    children,
    className,
    content,
    disabled,
    href,
    icon,
    name,
    onClick,
    ...rest
  } = props;
  const menuProps = cleanProps(rest);
  if (href) menuProps.href = href;
  if (onClick && Component === 'div') menuProps.role = menuProps.role || 'button';
  if (name) menuProps.name = name;

  return (
    <Component
      {...menuProps}
      aria-disabled={disabled || undefined}
      className={cx('item', commonClasses(props), className)}
      onClick={disabled ? undefined : onClick}
      ref={ref}
    >
      {icon ? <Icon name={icon} /> : null}
      {childrenOrContent(children, content)}
    </Component>
  );
});

const handleTextInputChange = (props) => (event) => {
  if (props.onChange) {
    props.onChange(event, {
      ...props,
      value: event.target.value,
    });
  }
};

const renderInputAction = (action) => {
  if (!action) return null;
  if (React.isValidElement(action)) return action;
  if (typeof action === 'object') {
    return <Button {...action} />;
  }
  return action;
};

export const Input = React.forwardRef((props, ref) => {
  const {
    action,
    children,
    className,
    disabled,
    icon,
    input,
    label,
    labelPosition,
    loading,
    onChange,
    ...rest
  } = props;
  const innerInputRef = React.useRef(null);
  React.useImperativeHandle(ref, () => ({
    blur: () => innerInputRef.current?.blur(),
    focus: () => innerInputRef.current?.focus(),
    inputRef: innerInputRef,
  }), []);

  const hasCustomInput = React.isValidElement(input);
  const wrapperTestId = hasCustomInput ? undefined : rest['data-testid'];
  const inputProps = cleanInputProps({ ...rest, disabled, onChange }, [
    'children',
    'className',
    wrapperTestId ? 'data-testid' : undefined,
  ]);
  const handleChange = handleTextInputChange({ ...props, onChange });
  const inputElement = hasCustomInput
    ? React.cloneElement(input, {
      ...inputProps,
      ...input.props,
      disabled: disabled || input.props.disabled,
      onChange: callAll(input.props.onChange, handleChange),
      ref: innerInputRef,
    })
    : (
      <input
        {...input}
        {...inputProps}
        onChange={handleChange}
        ref={innerInputRef}
      />
    );

  return (
    <div
      data-testid={wrapperTestId}
      className={cx(
        'ui',
        commonClasses({ ...props, disabled, loading }),
        labelPosition ? `${labelPosition} labeled` : null,
        action ? 'action' : null,
        'input',
        className,
      )}
    >
      {label && labelPosition === 'left' ? <Label>{label}</Label> : null}
      {inputElement}
      {icon ? <Icon name={typeof icon === 'string' ? icon : undefined} /> : null}
      {children}
      {renderInputAction(action)}
      {label && labelPosition !== 'left' ? <Label>{label}</Label> : null}
    </div>
  );
});

export const TextArea = React.forwardRef((props, ref) => {
  const {
    className,
    disabled,
    onChange,
    ...rest
  } = props;
  const textAreaProps = cleanInputProps({ ...rest, disabled, onChange }, ['type']);

  return (
    <textarea
      {...textAreaProps}
      className={className}
      onChange={handleTextInputChange({ ...props, onChange })}
      ref={ref}
    />
  );
});

const optionValue = (value) => value == null ? '' : String(value);

const mapSelectedValue = (options, selectedValue) => {
  const option = options.find((candidate) =>
    optionValue(candidate.value) === selectedValue);
  return option ? option.value : selectedValue;
};

const renderOptions = (options, placeholder, onOptionClick) => (
  <>
    {placeholder ? (
      <option
        disabled
        value=""
      >
        {placeholder}
      </option>
    ) : null}
    {options.map((option) => (
      <option
        disabled={option.disabled}
        key={option.key ?? optionValue(option.value)}
        onClick={(event) => onOptionClick(event, option.value)}
        value={optionValue(option.value)}
      >
        {option.text ?? option.content ?? option.value}
      </option>
    ))}
  </>
);

export const Dropdown = React.forwardRef((props, ref) => {
  const {
    children,
    className,
    content,
    disabled,
    icon,
    multiple,
    onChange,
    options = [],
    placeholder,
    text,
    trigger,
    value,
    ...rest
  } = props;

  if (options.length > 0) {
    const selectProps = cleanInputProps({ ...rest, disabled, multiple, value }, ['type']);
    const emitChange = (event, nextValue) => {
      if (onChange) onChange(event, { ...props, value: nextValue });
    };
    const handleChange = (event) => {
      const nextValue = multiple
        ? Array.from(event.target.selectedOptions).map((option) =>
          mapSelectedValue(options, option.value))
        : mapSelectedValue(options, event.target.value);
      emitChange(event, nextValue);
    };
    const handleOptionClick = (event, nextValue) => {
      if (!multiple) emitChange(event, nextValue);
    };

    return (
      <select
        {...selectProps}
        className={cx('ui', commonClasses(props), 'dropdown', className)}
        onChange={handleChange}
        ref={ref}
        role={selectProps.role || 'listbox'}
        value={multiple ? (value || []).map(optionValue) : optionValue(value)}
      >
        {renderOptions(options, placeholder, handleOptionClick)}
      </select>
    );
  }

  return (
    <div
      {...cleanProps(rest)}
      className={cx('ui dropdown', commonClasses(props), className)}
      ref={ref}
    >
      {trigger}
      {icon ? <Icon name={typeof icon === 'string' ? icon : 'dropdown'} /> : null}
      {text || childrenOrContent(children, content)}
    </div>
  );
});
Dropdown.Menu = createComponent('menu');
Dropdown.Item = React.forwardRef((props, ref) => {
  const {
    as: Component = 'div',
    children,
    className,
    content,
    disabled,
    icon,
    onClick,
    text,
    ...rest
  } = props;

  return (
    <Component
      {...cleanProps(rest)}
      aria-disabled={disabled || undefined}
      className={cx('item', commonClasses(props), className)}
      onClick={disabled ? undefined : onClick}
      ref={ref}
      role={Component === 'div' && onClick ? 'button' : undefined}
    >
      {icon ? <Icon name={icon} /> : null}
      {text || childrenOrContent(children, content)}
    </Component>
  );
});
Dropdown.Header = React.forwardRef((props, ref) => {
  const {
    children,
    className,
    content,
    icon,
    ...rest
  } = props;
  return (
    <div
      {...cleanProps(rest)}
      className={cx('header', className)}
      ref={ref}
    >
      {icon ? <Icon name={icon} /> : null}
      {childrenOrContent(children, content)}
    </div>
  );
});
Dropdown.Divider = createComponent('divider');

const CheckboxInput = React.forwardRef((props, ref) => {
  const {
    checked,
    className,
    disabled,
    label,
    onChange,
    radio,
    toggle,
    type,
    value,
    ...rest
  } = props;
  const inputType = type || (radio ? 'radio' : 'checkbox');
  const handleChange = (event) => {
    if (onChange) {
      onChange(event, {
        ...props,
        checked: event.target.checked,
        value,
      });
    }
  };
  const inputProps = cleanInputProps({
    ...rest,
    checked,
    disabled,
    onChange: handleChange,
    type: inputType,
    value,
  });

  return (
    <div className={cx('ui', toggle ? 'toggle' : null, radio ? 'radio' : null, 'checkbox', commonClasses(props), className)}>
      <input
        {...inputProps}
        ref={ref}
      />
      {label ? <label>{label}</label> : null}
    </div>
  );
});

export const Checkbox = CheckboxInput;
export const Radio = React.forwardRef((props, ref) => (
  <CheckboxInput
    {...props}
    radio
    ref={ref}
    type="radio"
  />
));

export const Form = React.forwardRef((props, ref) => {
  const {
    as: Component = 'form',
    children,
    className,
    content,
    ...rest
  } = props;

  return (
    <Component
      {...cleanProps(rest)}
      className={cx('ui form', commonClasses(props), className)}
      ref={ref}
    >
      {childrenOrContent(children, content)}
    </Component>
  );
});

Form.Group = createComponent('fields', 'div', (props) => [
  props.inline ? 'inline' : null,
  props.widths ? `${toWord(props.widths)} fields` : null,
]);

Form.Field = React.forwardRef((props, ref) => {
  const {
    as: Component = 'div',
    children,
    className,
    content,
    label,
    ...rest
  } = props;

  return (
    <Component
      {...cleanProps(rest)}
      className={cx('field', commonClasses(props), widthClass(props.width), className)}
      ref={ref}
    >
      {label ? <label>{label}</label> : null}
      {childrenOrContent(children, content)}
    </Component>
  );
});

Form.Input = React.forwardRef((props, ref) => {
  const {
    label,
    width,
    ...rest
  } = props;
  return (
    <Form.Field
      error={props.error}
      required={props.required}
      width={width}
    >
      {label ? <label>{label}</label> : null}
      <Input
        {...rest}
        ref={ref}
      />
    </Form.Field>
  );
});

Form.TextArea = React.forwardRef((props, ref) => {
  const {
    label,
    width,
    ...rest
  } = props;
  return (
    <Form.Field
      error={props.error}
      required={props.required}
      width={width}
    >
      {label ? <label>{label}</label> : null}
      <TextArea
        {...rest}
        ref={ref}
      />
    </Form.Field>
  );
});

Form.Select = React.forwardRef((props, ref) => {
  const {
    label,
    width,
    ...rest
  } = props;
  return (
    <Form.Field
      error={props.error}
      required={props.required}
      width={width}
    >
      {label ? <label>{label}</label> : null}
      <Dropdown
        {...rest}
        ref={ref}
      />
    </Form.Field>
  );
});

Form.Dropdown = Form.Select;

Form.Checkbox = React.forwardRef((props, ref) => (
  <Form.Field
    error={props.error}
    required={props.required}
    width={props.width}
  >
    <Checkbox
      {...props}
      ref={ref}
    />
  </Form.Field>
));

export const Modal = React.forwardRef((props, ref) => {
  const {
    children,
    className,
    closeIcon,
    content,
    onClose,
    onOpen,
    open,
    trigger,
    ...rest
  } = props;

  const triggerElement = trigger
    ? cloneTrigger(trigger, {
      onClick: (event) => {
        if (onOpen) onOpen(event, props);
      },
    })
    : null;

  if (!open) {
    return triggerElement;
  }

  const modal = (
    <div className="ui active visible dimmer modals page transition">
      <div
        {...cleanProps(rest)}
        className={cx('ui active visible modal', commonClasses(props), className)}
        ref={ref}
      >
        {closeIcon ? (
          <button
            aria-label="Close"
            className="close icon"
            onClick={(event) => onClose && onClose(event, props)}
            type="button"
          />
        ) : null}
        {childrenOrContent(children, content)}
      </div>
    </div>
  );

  return (
    <>
      {triggerElement}
      {modal}
    </>
  );
});
Modal.Header = createComponent('header');
Modal.Content = createComponent('content', 'div', (props) => props.scrolling ? 'scrolling' : null);
Modal.Actions = createComponent('actions');

const renderConfirmButton = (button, fallbackText, onClick, positive) => {
  if (React.isValidElement(button)) {
    return React.cloneElement(button, {
      onClick: callAll(button.props.onClick, onClick),
    });
  }
  if (typeof button === 'object' && button != null) {
    return (
      <Button
        {...button}
        onClick={onClick}
        positive={positive}
      />
    );
  }
  return (
    <Button
      onClick={onClick}
      positive={positive}
    >
      {button || fallbackText}
    </Button>
  );
};

export const Confirm = ({
  cancelButton,
  confirmButton,
  content,
  header,
  onCancel,
  onConfirm,
  open,
  ...props
}) => (
  <Modal
    {...props}
    onClose={onCancel}
    open={open}
    size="tiny"
  >
    {header ? <Modal.Header>{header}</Modal.Header> : null}
    <Modal.Content>{content}</Modal.Content>
    <Modal.Actions>
      {renderConfirmButton(cancelButton, 'Cancel', onCancel, false)}
      {renderConfirmButton(confirmButton, 'OK', onConfirm, true)}
    </Modal.Actions>
  </Modal>
);

export const Popup = ({
  basic,
  children,
  className,
  content,
  on,
  onClose,
  onOpen,
  open,
  position,
  trigger,
}) => {
  const [internalOpen, setInternalOpen] = React.useState(false);
  const isOpen = open ?? internalOpen;
  const popupContent = content === undefined && trigger ? children : content;
  const triggerElement = trigger || (content === undefined ? children : null);
  const title = typeof content === 'string' ? content : undefined;
  const handleTriggerClick = (event) => {
    if (on !== 'click' && open === undefined && !onOpen && !onClose) return;
    if (isOpen) {
      if (onClose) onClose(event);
      if (open === undefined) setInternalOpen(false);
      return;
    }
    if (onOpen) onOpen(event);
    if (open === undefined) setInternalOpen(true);
  };
  const renderedTrigger = React.isValidElement(triggerElement)
    ? React.cloneElement(triggerElement, {
      onClick: callAll(triggerElement.props.onClick, handleTriggerClick),
      title: triggerElement.props.title || title,
    })
    : triggerElement;

  if (!isOpen) return renderedTrigger;

  return (
    <>
      {renderedTrigger}
      <div
        className={cx('ui visible popup', basic ? 'basic' : null, position, className)}
        role="tooltip"
      >
        {popupContent}
      </div>
    </>
  );
};

export const Portal = ({ children, mountNode, open }) => {
  if (!open) return null;
  const target = mountNode || (typeof document !== 'undefined' ? document.body : null);
  if (!target) return children;
  return createPortal(children, target);
};

export const Ref = ({ children, innerRef }) => {
  if (!React.isValidElement(children)) return children;
  return React.cloneElement(children, { ref: innerRef });
};

export const Sidebar = createComponent('ui sidebar');
Sidebar.Pushable = createComponent('pushable');
Sidebar.Pusher = createComponent('pusher');

export const Dimmer = ({ active, children, className, ...props }) => {
  if (!active) return null;
  return (
    <div
      {...cleanProps(props)}
      className={cx('ui active dimmer', commonClasses({ ...props, active }), className)}
    >
      {children}
    </div>
  );
};

export const Loader = ({ active = true, children, className, content, ...props }) => {
  if (!active) return null;
  return (
    <div
      {...cleanProps(props)}
      className={cx('ui active loader', commonClasses({ ...props, active }), className)}
    >
      {childrenOrContent(children, content)}
    </div>
  );
};

export const Progress = ({
  children,
  className,
  percent = 0,
  progress,
  value,
  total,
  ...props
}) => {
  const computedPercent = percent || (total ? Math.round((value / total) * 100) : 0);
  const displayProgress = progress === true ? `${computedPercent}%` : progress;

  return (
    <div
      {...cleanProps(props)}
      aria-valuemax={100}
      aria-valuemin={0}
      aria-valuenow={computedPercent}
      className={cx('ui progress', commonClasses(props), className)}
      role="progressbar"
    >
      <div
        className="bar"
        style={{ width: `${Math.max(0, Math.min(100, computedPercent))}%` }}
      >
        {displayProgress ? <div className="progress">{displayProgress}</div> : null}
      </div>
      {children ? <div className="label">{children}</div> : null}
    </div>
  );
};

export const Pagination = ({
  activePage = 1,
  boundaryRange,
  className,
  onPageChange,
  siblingRange,
  totalPages = 1,
  ...props
}) => {
  const pageCount = Math.max(1, Number(totalPages) || 1);
  const pages = Array.from({ length: pageCount }, (_, index) => index + 1);
  const selectPage = (event, nextPage) => {
    if (onPageChange) onPageChange(event, { ...props, activePage: nextPage });
  };

  return (
    <div
      {...cleanProps(props)}
      className={cx('ui pagination menu', className)}
    >
      {pages.map((page) => (
        <button
          className={cx('item', page === activePage ? 'active' : null)}
          key={page}
          onClick={(event) => selectPage(event, page)}
          type="button"
        >
          {page}
        </button>
      ))}
    </div>
  );
};

export const Flag = ({ className, name, ...props }) => (
  <i
    {...cleanProps(props)}
    className={cx(name, 'flag', className)}
  />
);

const getMenuItem = (menuItem, index) => {
  if (React.isValidElement(menuItem)) {
    return {
      content: menuItem,
      key: menuItem.key || index,
    };
  }
  if (typeof menuItem === 'object' && menuItem != null) {
    return {
      content: menuItem.content || menuItem.text || menuItem.name || menuItem.key,
      icon: menuItem.icon,
      key: menuItem.key || menuItem.content || index,
    };
  }
  return {
    content: menuItem,
    key: menuItem || index,
  };
};

export const Tab = ({
  activeIndex,
  className,
  defaultActiveIndex = 0,
  menu,
  onTabChange,
  panes = [],
  renderActiveOnly = true,
  ...props
}) => {
  const [internalActiveIndex, setInternalActiveIndex] = React.useState(defaultActiveIndex);
  const selectedIndex = activeIndex ?? internalActiveIndex;
  const selectPane = (event, nextIndex) => {
    setInternalActiveIndex(nextIndex);
    if (onTabChange) onTabChange(event, { ...props, activeIndex: nextIndex });
  };
  const renderedPanes = renderActiveOnly
    ? panes.filter((_, index) => index === selectedIndex)
    : panes;

  return (
    <div className={cx('ui tabular wrapper', className)}>
      <Menu
        {...menu}
        className={cx(menu?.className)}
        tabular
      >
        {panes.map((pane, index) => {
          const item = getMenuItem(pane.menuItem, index);
          return (
            <Menu.Item
              active={index === selectedIndex}
              icon={item.icon}
              key={item.key}
              onClick={(event) => selectPane(event, index)}
            >
              {item.content}
            </Menu.Item>
          );
        })}
      </Menu>
      {renderedPanes.map((pane, index) => {
        const originalIndex = renderActiveOnly ? selectedIndex : index;
        const rendered = pane.render ? pane.render() : pane.children || pane.content;
        return React.isValidElement(rendered)
          ? React.cloneElement(rendered, {
            active: originalIndex === selectedIndex,
            key: pane.menuItem?.key || originalIndex,
          })
          : rendered;
      })}
    </div>
  );
};
Tab.Pane = createComponent('ui tab segment', 'div', (props) => props.active ? 'active' : null);

export default {
  Button,
  ButtonGroup,
  Card,
  Checkbox,
  Confirm,
  Container,
  Dimmer,
  Divider,
  Dropdown,
  Flag,
  Form,
  Grid,
  Header,
  Icon,
  Input,
  Item,
  Label,
  List,
  Loader,
  Menu,
  Message,
  Modal,
  Pagination,
  Popup,
  Portal,
  Progress,
  Radio,
  Ref,
  Segment,
  Sidebar,
  Statistic,
  Tab,
  Table,
  TextArea,
};
