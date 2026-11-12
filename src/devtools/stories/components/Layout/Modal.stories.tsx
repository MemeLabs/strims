// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Modal.stories.scss";

import React from "react";

import Modal from "../../../../components/Layout/Modal";
import { Story } from "../../../storybook";

// The modal portals into the layout's root, the story's frame, and fills
// the viewport (see Modal.stories.scss for the desktop frame).

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
      <Modal title="Settings" onClose={noop}>
        <Paragraphs count={2} />
      </Modal>
    ),
  },
  {
    name: "No header",
    component: () => (
      <Modal onClose={noop}>
        <Paragraphs count={2} />
      </Modal>
    ),
  },
  {
    name: "Header without title",
    component: () => (
      <Modal showHeader onClose={noop}>
        <Paragraphs count={2} />
      </Modal>
    ),
  },
  {
    // One line, cut short with an ellipsis before the close button.
    name: "Long title",
    component: () => (
      <Modal
        title="A title long enough that it runs past the width of a phone-sized header, so it ends in an ellipsis on one line"
        onClose={noop}
      >
        <Paragraphs count={1} />
      </Modal>
    ),
  },
  {
    name: "Long content",
    component: () => (
      <Modal title="Settings" onClose={noop}>
        <Paragraphs count={40} />
      </Modal>
    ),
  },
];

export default stories;
