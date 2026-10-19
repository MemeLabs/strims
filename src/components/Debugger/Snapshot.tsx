// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useState } from "react";

import { Int64Value, UInt32Value, UInt64Value } from "../../apis/google/protobuf/wrappers";
import { Metrics } from "../../apis/strims/debug/v1/metrics";
import { hex, jsonReplacer } from "./series";

const BYTES_SHOWN = 8;

const formatLeaf = (value: unknown): string => {
  switch (typeof value) {
    case "bigint":
    case "number":
    case "boolean":
      return value.toString();
    case "string":
      return JSON.stringify(value);
    case "undefined":
      return "unset";
  }
  if (value instanceof Uint8Array) {
    const digits = hex(value.subarray(0, BYTES_SHOWN));
    return value.length > BYTES_SHOWN ? `${digits}… (${value.length} B)` : digits || "empty";
  }
  return String(value);
};

// The snapshot's optional scalars, shown as their values.
const isWrapper = (value: unknown): value is Int64Value | UInt32Value | UInt64Value =>
  value instanceof Int64Value || value instanceof UInt32Value || value instanceof UInt64Value;

const isBranch = (value: unknown): value is object =>
  typeof value === "object" &&
  value !== null &&
  !(value instanceof Uint8Array) &&
  !isWrapper(value);

interface BranchProps {
  name: string;
  value: object;
  defaultOpen?: boolean;
}

// Renders its children only while open, since a snapshot can be large.
const Branch: React.FC<BranchProps> = ({ name, value, defaultOpen = false }) => {
  const [open, setOpen] = useState(defaultOpen);
  const entries = Object.entries(value);
  const summary = Array.isArray(value) ? `[${value.length}]` : "{…}";

  return (
    <details
      className="debugger_snapshot__branch"
      open={open}
      onToggle={(e) => setOpen(e.currentTarget.open)}
    >
      <summary>
        <span className="debugger_snapshot__key">{name}</span> {summary}
      </summary>
      {open &&
        entries.map(([key, child]) =>
          isBranch(child) ? (
            <Branch key={key} name={key} value={child} />
          ) : (
            <div key={key} className="debugger_snapshot__leaf">
              <span className="debugger_snapshot__key">{key}</span>{" "}
              {formatLeaf(isWrapper(child) ? child.value : child)}
            </div>
          )
        )}
    </details>
  );
};

interface SnapshotProps {
  snapshot: Metrics;
}

const Snapshot: React.FC<SnapshotProps> = ({ snapshot }) => {
  const [copied, setCopied] = useState(false);

  const handleCopyClick = () => {
    void navigator.clipboard.writeText(JSON.stringify(snapshot, jsonReplacer, 2)).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1000);
    });
  };

  return (
    <div className="debugger_snapshot">
      <button className="debugger_snapshot__copy" onClick={handleCopyClick}>
        {copied ? "copied" : "copy JSON"}
      </button>
      <Branch name="metrics" value={snapshot} defaultOpen />
    </div>
  );
};

export default Snapshot;
