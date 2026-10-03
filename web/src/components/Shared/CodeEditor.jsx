import {
  defaultHighlightStyle,
  StreamLanguage,
  syntaxHighlighting,
} from '@codemirror/language';
import { yaml } from '@codemirror/legacy-modes/mode/yaml';
import { EditorView } from '@codemirror/view';
import CodeMirror from '@uiw/react-codemirror';
import React from 'react';

const CodeEditor = ({
  ariaLabel,
  editable,
  onChange = () => {},
  theme,
  value,
  ...rest
}) => {
  const cspNonce = typeof document === 'undefined'
    ? ''
    : document.querySelector('meta[name="csp-nonce"]')?.getAttribute('content') || '';
  const contentAttributes = {
    ...(ariaLabel ? { 'aria-label': ariaLabel } : {}),
    ...(editable === false ? { 'aria-readonly': 'true', tabindex: '0' } : {}),
  };

  return (
    <CodeMirror
      extensions={[
        StreamLanguage.define(yaml),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        EditorView.contentAttributes.of(contentAttributes),
        ...(cspNonce ? [EditorView.cspNonce.of(cspNonce)] : []),
      ]}
      editable={editable}
      onChange={(newValue) => onChange(newValue)}
      theme={theme}
      value={value}
      {...rest}
    />
  );
};

export default CodeEditor;
