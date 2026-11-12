// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { useEffect, useState, useSyncExternalStore } from "react";

import { Session } from "./recorder";

// JSON in a script element: "<" escaped so no text in it ends the element.
export const jsonScript = (value: unknown): { __html: string } => ({
  __html: JSON.stringify(value).replace(/</g, "\\u003c"),
});

// The story's recorded errors: a collapsible list, and the same as JSON in
// #storybook-story ({route, state, errors: [{kind, text, allowedBy?}]}) for
// hack/storybook/check.py.
const ErrorList: React.FC<{ session: Session }> = ({ session }) => {
  const snapshot = useSyncExternalStore(session.subscribe, session.getSnapshot);
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
      <script
        type="application/json"
        id="storybook-story"
        dangerouslySetInnerHTML={jsonScript(snapshot)}
      />
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
