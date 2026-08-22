import {
  defaultHighlightStyle,
  StreamLanguage,
  syntaxHighlighting,
} from '@codemirror/language';
import { yaml } from '@codemirror/legacy-modes/mode/yaml';
import { EditorView } from '@codemirror/view';
import CodeMirror from '@uiw/react-codemirror';
import React from 'react';

const CodeEditor = ({ onChange = () => {}, theme, value, ...rest }) => {
  const cspNonce = typeof document === 'undefined'
    ? ''
    : document.querySelector('meta[name="csp-nonce"]')?.getAttribute('content') || '';

  return (
    <CodeMirror
      extensions={[
        StreamLanguage.define(yaml),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        ...(cspNonce ? [EditorView.cspNonce.of(cspNonce)] : []),
      ]}
      onChange={(newValue) => onChange(newValue)}
      theme={theme}
      value={value}
      {...rest}
    />
  );
};

export default CodeEditor;
