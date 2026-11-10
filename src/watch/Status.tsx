// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { MdLoop } from "react-icons/md";

interface StatusProps {
  /** None shows a spinner. */
  message?: string;
  action?: { label: string; onClick: () => void };
}

/** A full-window line of status, and what to do about it. */
const Status: React.FC<StatusProps> = ({ message, action }) => (
  <div className="watch_status">
    {message ? (
      <span className="watch_status__message">{message}</span>
    ) : (
      <MdLoop className="watch_status__spinner" />
    )}
    {action && (
      <button className="watch_status__action" onClick={action.onClick}>
        {action.label}
      </button>
    )}
  </div>
);

export default Status;
