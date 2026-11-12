// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import PublishNetworkModal from "../../../../../pages/Settings/Network/PublishNetworkModal";
import BootstrapService, { bootstrapPeers } from "../../../../mocks/bootstrap/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory, Step, choose, openSelect, submit } from "./harness";

const noop = (): void => undefined;

const Modal: React.FC<{ bootstrap?: () => BootstrapService; steps?: Step[] }> = ({
  bootstrap,
  steps,
}) => (
  <SettingsStory
    services={bootstrap && (() => ({ bootstrap: bootstrap() }))}
    element={<PublishNetworkModal network={fixtures.network(1)} onClose={noop} />}
    steps={steps}
  />
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => <Modal />,
  },
  {
    // The bootstrap peers it can be published to.
    name: "Peers",
    component: () => <Modal steps={[openSelect()]} />,
  },
  {
    name: "Many peers",
    component: () => (
      <Modal
        bootstrap={() => new BootstrapService({ peers: bootstrapPeers(12) })}
        steps={[openSelect()]}
      />
    ),
  },
  {
    name: "No peers",
    component: () => (
      <Modal bootstrap={() => new BootstrapService({ peers: [] })} steps={[openSelect()]} />
    ),
  },
  {
    name: "Chosen",
    component: () => <Modal steps={[openSelect(), choose("strims.gg:443")]} />,
  },
  {
    name: "Required",
    component: () => <Modal steps={[submit()]} />,
  },
];

export default stories;
