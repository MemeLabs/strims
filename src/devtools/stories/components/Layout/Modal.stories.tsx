// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Modal.stories.scss";

import React, { ReactNode } from "react";

import Modal from "../../../../components/Layout/Modal";
import { WithRootRefProps, withLayoutContext } from "../../../../contexts/Layout";
import { Story } from "../../../storybook";

// A layout root of the story's own, which the modal portals into: it is the
// modal's viewport (see Modal.stories.scss).
const Viewport = withLayoutContext<{ children: ReactNode }>(
  ({ rootRef, children }: { children: ReactNode } & WithRootRefProps) => (
    <div ref={rootRef} className="modal_story">
      {children}
    </div>
  )
);

const Paragraphs: React.FC<{ count: number }> = ({ count }) => (
  <div className="modal_story__body">
    {Array.from({ length: count }, (_, i) => (
      <p key={i}>
        {i + 1}. Settings, search and the other full-screen panels of the mobile layout open in a
        modal that slides up from the bottom of the screen and closes when it is swiped back down or
        its header&apos;s button is pressed.
      </p>
    ))}
  </div>
);

const noop = (): void => undefined;

const stories: Story[] = [
  {
    name: "Title",
    component: () => (
      <Viewport>
        <Modal title="Settings" onClose={noop}>
          <Paragraphs count={2} />
        </Modal>
      </Viewport>
    ),
  },
  {
    name: "No header",
    component: () => (
      <Viewport>
        <Modal onClose={noop}>
          <Paragraphs count={2} />
        </Modal>
      </Viewport>
    ),
  },
  {
    name: "Header without title",
    component: () => (
      <Viewport>
        <Modal showHeader onClose={noop}>
          <Paragraphs count={2} />
        </Modal>
      </Viewport>
    ),
  },
  {
    // One line, cut short with an ellipsis before the close button.
    name: "Long title",
    component: () => (
      <Viewport>
        <Modal
          title="A title long enough that it runs past the width of a phone-sized header, so it ends in an ellipsis on one line"
          onClose={noop}
        >
          <Paragraphs count={1} />
        </Modal>
      </Viewport>
    ),
  },
  {
    name: "Long content",
    component: () => (
      <Viewport>
        <Modal title="Settings" onClose={noop}>
          <Paragraphs count={40} />
        </Modal>
      </Viewport>
    ),
  },
];

export default stories;
