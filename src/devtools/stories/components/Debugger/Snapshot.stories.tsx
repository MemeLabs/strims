// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../components/Debugger/Debugger.scss";

import React, { useEffect, useMemo, useRef } from "react";

import Portlet from "../../../../components/Debugger/Portlet";
import Snapshot from "../../../../components/Debugger/Snapshot";
import { latest } from "../../../../components/Debugger/series";
import { Story } from "../../../storybook";
import { metricsHistory } from "./metrics";
import { usePortletPlacement } from "./portlet";

// Opens the branches at these paths of keys below the root, outermost
// first, as a viewer clicking them would.
const useOpenBranches = (root: React.RefObject<HTMLElement>, paths: string[][]) =>
  useEffect(() => {
    const open = (path: string[]) => {
      let details = root.current?.querySelector<HTMLDetailsElement>("details");
      for (const key of path) {
        details = Array.from(
          details?.querySelectorAll<HTMLDetailsElement>(":scope > details") ?? []
        ).find(
          (d) => d.querySelector(":scope > summary > .debugger_snapshot__key")?.textContent === key
        );
        if (!details) {
          return false;
        }
        if (!details.open) {
          details.open = true;
          return false;
        }
      }
      return true;
    };
    // Each branch renders its children once its toggle event says it opened.
    let frame = 0;
    const step = () => {
      if (!paths.every(open)) {
        frame = requestAnimationFrame(step);
      }
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, []);

const SnapshotStory: React.FC<{ open?: string[][] }> = ({ open = [] }) => {
  const snapshot = useMemo(() => latest(metricsHistory({ count: 2 })), []);
  const ref = useRef<HTMLDivElement>(null);
  usePortletPlacement(() => ref.current, { width: 560, height: 520 });
  useOpenBranches(ref, open);
  return (
    <Portlet isOpen onClose={() => undefined}>
      <div ref={ref}>
        <Snapshot snapshot={snapshot} />
      </div>
    </Portlet>
  );
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SnapshotStory />,
  },
  {
    name: "Expanded",
    component: () => (
      <SnapshotStory
        open={[
          ["memory"],
          ["node", "links", "0"],
          ["node", "links", "0", "metrics"],
          ["debug", "readers", "0"],
        ]}
      />
    ),
  },
];

export default stories;
