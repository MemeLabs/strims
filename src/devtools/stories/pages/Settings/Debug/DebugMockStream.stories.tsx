// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import DebugService, { mockStream } from "../../../../mocks/debug/service";
import { StaticNetwork, Story, fixtures } from "../../../../storybook";
import { SettingsStory, SettingsStoryProps } from "../Layout.harness";

const PATH = "/settings/debug/mock-stream";

// The page names streams' networks from NetworkContext: fixture networks 1..5.
const Context: React.FC<Omit<SettingsStoryProps, "path">> = (props) => (
  <StaticNetwork>
    <SettingsStory path={PATH} {...props} />
  </StaticNetwork>
);

const stories: Story[] = [
  {
    name: "Empty",
    component: () => <Context />,
  },
  {
    // Listed and unlisted streams, with and without a timeout; the last in
    // a network the node isn't in, named by its key.
    name: "Running",
    component: () => (
      <Context
        services={() => ({
          debug: new DebugService({
            mockStreams: [
              ...[1, 2, 3, 4].map((i) => mockStream(i)),
              mockStream(5, { networkKey: fixtures.key("network:gone") }),
            ],
          }),
        })}
      />
    ),
  },
  {
    // Submitted without a network.
    name: "Invalid",
    component: () => <Context submit />,
  },
];

export default stories;
