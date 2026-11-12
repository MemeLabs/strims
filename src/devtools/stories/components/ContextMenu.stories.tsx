// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React, { useLayoutEffect, useRef } from "react";

import { MenuItem, useContextMenu } from "../../../components/ContextMenu";
import { Story } from "../../storybook";

interface HarnessProps {
  /** Where the target sits: the story's top left, or its bottom right corner. */
  corner?: boolean;
  /** Opens the menu at the target's bottom right corner rather than its top left. */
  openAtEnd?: boolean;
}

// A target that opens its context menu on mount, as a right click on it
// would: the menu at a point of the target's.
const Harness: React.FC<HarnessProps> = ({ corner, openAtEnd }) => {
  const ref = useRef<HTMLDivElement>(null);
  const { isOpen, openMenu, closeMenu, Menu } = useContextMenu();

  // The menu's position: fixed coordinates are the event's page ones.
  useLayoutEffect(() => {
    const rect = ref.current.getBoundingClientRect();
    const [x, y] = openAtEnd ? [rect.right, rect.bottom] : [rect.left + 24, rect.top + 24];
    openMenu({ pageX: x, pageY: y } as React.MouseEvent);
  }, []);

  return (
    <div className="core_story core_story--fill">
      <div
        ref={ref}
        className={
          corner ? "core_story__context_target core_story__corner" : "core_story__context_target"
        }
        onContextMenu={(e) => {
          e.preventDefault();
          openMenu(e);
        }}
      >
        Right click for the menu ({isOpen ? "open" : "closed"})
      </div>
      <Menu>
        <MenuItem onClick={closeMenu}>Mark as read</MenuItem>
        <MenuItem onClick={closeMenu}>Mute room</MenuItem>
        <MenuItem disabled onClick={closeMenu}>
          Delete thread
        </MenuItem>
        <MenuItem component="a" href="https://strims.gg" target="_blank" rel="noreferrer">
          Open in a new tab
        </MenuItem>
      </Menu>
    </div>
  );
};

const stories: Story[] = [
  {
    name: "Open",
    component: () => <Harness />,
  },
  {
    name: "Flipped",
    component: () => <Harness corner openAtEnd />,
  },
];

export default stories;
