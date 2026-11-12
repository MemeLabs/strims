// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useState } from "react";

import { PublishNetworkToBootstrapPeerRequest } from "../../../../../apis/strims/network/v1/bootstrap/bootstrap";
import PublishNetworkModal from "../../../../../pages/Settings/Network/PublishNetworkModal";
import BootstrapService, { bootstrapPeers } from "../../../../mocks/bootstrap/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory, Step, choose, click, openSelect, submit } from "./harness";

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

// A bootstrap service that says each time it publishes.
class PublishCounter extends BootstrapService {
  onPublish: (count: number) => void = noop;

  publishNetworkToPeer(req: PublishNetworkToBootstrapPeerRequest) {
    const res = super.publishNetworkToPeer(req);
    this.onPublish(this.published.length);
    return res;
  }
}

// The modal with a peer chosen and Cancel clicked, and a count of its
// publishes and closes below: Cancel closes it (here it stays, the story's
// page ignoring its close) without publishing.
const Cancelled: React.FC = () => {
  const [bootstrap] = useState(() => new PublishCounter());
  const [published, setPublished] = useState(0);
  const [closes, setCloses] = useState(0);
  useEffect(() => void (bootstrap.onPublish = setPublished), []);
  return (
    <SettingsStory
      services={() => ({ bootstrap })}
      element={
        <PublishNetworkModal
          network={fixtures.network(1)}
          onClose={() => setCloses((n) => n + 1)}
        />
      }
      steps={[openSelect(), choose("strims.gg:443"), click("button", "Cancel")]}
    >
      <p className="settings_story__note">
        closes: {closes}, publishes: {published}
      </p>
    </SettingsStory>
  );
};

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
  {
    // Cancel with a peer chosen closes the modal and publishes nothing.
    name: "Cancelled",
    component: Cancelled,
  },
];

export default stories;
