// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import Status from "../../../watch/Status";
import { Story } from "../../storybook";
import { WatchRoot } from "./streams";

const noop = (): void => undefined;

const stories: Story[] = [
  {
    name: "Spinner",
    component: () => (
      <WatchRoot>
        <Status />
      </WatchRoot>
    ),
  },
  {
    name: "Message",
    component: () => (
      <WatchRoot>
        <Status message="No streams are live" />
      </WatchRoot>
    ),
  },
  {
    name: "Action",
    component: () => (
      <WatchRoot>
        <Status
          message="Couldn't join this stream's network."
          action={{ label: "Retry", onClick: noop }}
        />
      </WatchRoot>
    ),
  },
  {
    name: "Long text",
    component: () => (
      <WatchRoot>
        <Status
          message={
            "This stream isn't open to visitors. The network that carries it takes members by " +
            "invitation only, and its owner hasn't offered one to this page's node."
          }
          action={{ label: "Back", onClick: noop }}
        />
      </WatchRoot>
    ),
  },
];

export default stories;
