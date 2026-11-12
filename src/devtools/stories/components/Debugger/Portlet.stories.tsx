// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Debugger.stories.scss";

import React, { useRef } from "react";

import Portlet, { usePortletSize } from "../../../../components/Debugger/Portlet";
import { Story } from "../../../storybook";
import { PortletPlacement, usePortletPlacement } from "./portlet";

const Content: React.FC<{ contentRef: React.Ref<HTMLDivElement> }> = ({ contentRef }) => {
  const { width, height } = usePortletSize();
  return (
    <div ref={contentRef} className="debugger_story__portlet_content">
      <p>
        content area {width} × {height}
      </p>
      <p>
        Drag the header to move the window; drag an edge or a corner to resize it. The close button
        is in the header&apos;s right corner.
      </p>
    </div>
  );
};

const PortletStory: React.FC<PortletPlacement> = (placement) => {
  const ref = useRef<HTMLDivElement>(null);
  usePortletPlacement(() => ref.current, placement);
  return (
    <Portlet isOpen onClose={() => undefined}>
      <Content contentRef={ref} />
    </Portlet>
  );
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => <PortletStory />,
  },
  {
    name: "Resized",
    component: () => <PortletStory width={720} height={520} />,
  },
];

export default stories;
