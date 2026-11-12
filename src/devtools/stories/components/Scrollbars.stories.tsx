// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React, { useEffect, useRef } from "react";

import Scrollbars, { ScrollbarsHandle } from "../../../components/Scrollbars";
import { Story } from "../../storybook";

interface BoxProps {
  lines: number;
  autoHide?: boolean;
  /** Scrolls the view this far down once it mounts. */
  scrollTop?: number;
}

// A small view of numbered lines.
const Box: React.FC<BoxProps> = ({ lines, autoHide, scrollTop }) => {
  const ref = useRef<ScrollbarsHandle>(null);
  useEffect(() => {
    if (scrollTop) {
      ref.current.view.scrollTop = scrollTop;
    }
  }, []);
  return (
    <div className="core_story">
      <div className="core_story__viewport">
        <Scrollbars ref={ref} autoHide={autoHide}>
          <div className="core_story__panel">
            {Array.from({ length: lines }, (_, i) => (
              <p key={i}>Line {i + 1}</p>
            ))}
          </div>
        </Scrollbars>
      </div>
    </div>
  );
};

const stories: Story[] = [
  {
    // The thumb shows only while the view scrolls.
    name: "Auto hide",
    component: () => <Box lines={40} autoHide />,
  },
  {
    name: "Default",
    component: () => <Box lines={40} />,
  },
  {
    name: "Scrolled",
    component: () => <Box lines={40} scrollTop={400} />,
  },
  {
    // Nothing to scroll: no thumb.
    name: "Short",
    component: () => <Box lines={3} />,
  },
];

export default stories;
