// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../../components/Layout/NetworkNav/NetworkNav.scss";
import "../Layout.stories.scss";

import React from "react";

import NetworkGem from "../../../../../components/Layout/NetworkNav/NetworkGem";
import { Story, fixtures } from "../../../../storybook";
import { useNetworkIcons } from "../shell";

const PEER_COUNTS = [0, 1, 42, 500, 501, 123456];

// A gem per peer count, without icons (the network's initial on a colour)
// or with them.
const Gems: React.FC<{ icons?: boolean }> = ({ icons }) => {
  const bundles = useNetworkIcons(icons ? PEER_COUNTS.length : 0);
  if (icons && !bundles) {
    return null;
  }
  return (
    <div className="layout_story__gems">
      {PEER_COUNTS.map((peerCount, i) => (
        <figure key={i}>
          <NetworkGem
            network={fixtures.network(i + 1)}
            peerCount={peerCount}
            icon={bundles?.get(BigInt(i + 1))?.icon}
          />
          <figcaption>
            {peerCount.toLocaleString()} {peerCount === 1 ? "peer" : "peers"}
          </figcaption>
        </figure>
      ))}
    </div>
  );
};

const stories: Story[] = [
  {
    name: "Without icon",
    component: () => <Gems />,
  },
  {
    name: "With icon",
    component: () => <Gems icons />,
  },
];

export default stories;
