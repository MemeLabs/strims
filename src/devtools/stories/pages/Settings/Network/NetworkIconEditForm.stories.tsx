// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useState } from "react";

import { Image } from "../../../../../apis/strims/type/image";
import NetworkService from "../../../../mocks/network/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory, Step, pngImage, submit, waitFor } from "./harness";

class FailingUpdateNetworkService extends NetworkService {
  updateServerConfig(): never {
    throw new Error("the network's server isn't running");
  }
}

const PATH = "/settings/networks/1/icon";

interface WithIconProps {
  failing?: boolean;
  steps?: Step[];
}

// Network 1's server with an icon (a fixture emote), once its image has loaded.
const WithIcon: React.FC<WithIconProps> = ({ failing, steps }) => {
  const [icon, setIcon] = useState<Image>();
  useEffect(() => {
    void pngImage(fixtures.iconUrls[3]).then(setIcon);
  }, []);
  if (!icon) {
    return null;
  }
  const networks = () => {
    const networks = fixtures.networks(5);
    networks[0].serverConfig.icon = icon;
    return networks;
  };
  const Service = failing ? FailingUpdateNetworkService : NetworkService;
  return (
    <SettingsStory
      path={PATH}
      services={() => ({ network: new Service({ networks: networks() }) })}
      steps={steps}
    />
  );
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => <WithIcon />,
  },
  {
    // An owner network without an icon yet.
    name: "No icon",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    // Saving without an image.
    name: "Invalid",
    component: () => <SettingsStory path={PATH} steps={[submit()]} />,
  },
  {
    // A network the profile is a member of has no server config: the page is blank.
    name: "Member",
    component: () => <SettingsStory path="/settings/networks/2/icon" />,
  },
  {
    // No network 9.
    name: "Error",
    component: () => <SettingsStory path="/settings/networks/9/icon" />,
  },
  {
    name: "Update error",
    component: () => <WithIcon failing steps={[waitFor(".input_image__preview"), submit()]} />,
  },
  {
    // Saving goes back to the network.
    name: "Saved",
    component: () => (
      <WithIcon
        steps={[waitFor(".input_image__preview"), submit(), waitFor("input[name=alias]")]}
      />
    ),
  },
];

export default stories;
