// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The grammar adds itself to the global Prism the import above sets.
import "prismjs/components/prism-sql";

import Prism from "prismjs";
import React, { useRef } from "react";

export interface SqlEditorProps {
  value: string;
  onChange: (sql: string) => void;
  /** Ctrl+Enter or Cmd+Enter. */
  onRun?: () => void;
  readOnly?: boolean;
  rows?: number;
}

/** SQL, highlighted by Prism's grammar under a transparent textarea that edits it. */
const SqlEditor: React.FC<SqlEditorProps> = ({ value, onChange, onRun, readOnly, rows = 14 }) => {
  const pre = useRef<HTMLPreElement>(null);
  // Prism escapes the text it highlights; the trailing newline keeps the last line's height.
  const html = Prism.highlight(value, Prism.languages.sql, "sql") + "\n";

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      onRun?.();
    } else if (e.key === "Tab" && !readOnly) {
      e.preventDefault();
      const t = e.currentTarget;
      const { selectionStart: s, selectionEnd: end } = t;
      onChange(value.slice(0, s) + "    " + value.slice(end));
      requestAnimationFrame(() => t.setSelectionRange(s + 4, s + 4));
    }
  };

  const onScroll = (e: React.UIEvent<HTMLTextAreaElement>) => {
    if (pre.current) {
      pre.current.scrollTop = e.currentTarget.scrollTop;
      pre.current.scrollLeft = e.currentTarget.scrollLeft;
    }
  };

  return (
    <div className="console__editor" style={{ height: `${rows * 1.45 + 1}em` }}>
      <pre ref={pre} className="console__editor-code language-sql" aria-hidden>
        <code dangerouslySetInnerHTML={{ __html: html }} />
      </pre>
      <textarea
        className="console__editor-input"
        value={value}
        spellCheck={false}
        readOnly={readOnly}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={onKeyDown}
        onScroll={onScroll}
      />
    </div>
  );
};

export default SqlEditor;
