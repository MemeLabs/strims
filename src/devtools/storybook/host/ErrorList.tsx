// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { useEffect, useState } from "react";

import type { SessionSnapshot } from "./recorder";

// JSON in a script element: "<" escaped so no text in it ends the element.
export const jsonScript = (value: unknown): { __html: string } => ({
  __html: JSON.stringify(value).replace(/</g, "\\u003c"),
});

// A story's recorder snapshot as JSON in #storybook-story ({route, state,
// errors: [{kind, text, allowedBy?}]}), in its frame document for
// hack/storybook/check.py and on the page beside the error list.
export const StoryJson: React.FC<{ snapshot: SessionSnapshot }> = ({ snapshot }) => (
  <script
    type="application/json"
    id="storybook-story"
    dangerouslySetInnerHTML={jsonScript(snapshot)}
  />
);

// The story's recorded errors, as its frame posted them: a collapsible list.
const ErrorList: React.FC<{ snapshot: SessionSnapshot }> = ({ snapshot }) => {
  const { errors, state } = snapshot;
  const failing = errors.filter((e) => !e.allowedBy).length;
  const [open, setOpen] = useState(false);
  useEffect(() => setOpen(state === "error"), [state === "error"]);

  return (
    <div
      className={clsx("storybook_errors", {
        "storybook_errors--failing": state === "error",
        "storybook_errors--empty": !errors.length,
      })}
    >
      <StoryJson snapshot={snapshot} />
      {errors.length > 0 && (
        <>
          <button className="storybook_errors__summary" onClick={() => setOpen(!open)}>
            {open ? "▾" : "▸"} {failing} error{failing === 1 ? "" : "s"}
            {errors.length > failing && `, ${errors.length - failing} allowed`}
          </button>
          {open && (
            <ol className="storybook_errors__list">
              {errors.map((e, i) => (
                <li
                  key={i}
                  className={clsx("storybook_errors__item", {
                    "storybook_errors__item--allowed": e.allowedBy,
                  })}
                >
                  <span className="storybook_errors__kind">{e.kind}</span>
                  {e.allowedBy && <span className="storybook_errors__allowed">{e.allowedBy}</span>}
                  <pre>{e.text}</pre>
                </li>
              ))}
            </ol>
          )}
        </>
      )}
    </div>
  );
};

export default ErrorList;
