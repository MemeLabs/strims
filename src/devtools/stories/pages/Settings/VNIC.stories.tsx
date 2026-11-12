// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useState } from "react";

import * as vnicv1 from "../../../../apis/strims/vnic/v1/vnic";
import VNICService from "../../../mocks/vnic/service";
import { Story } from "../../../storybook";
import { SettingsStory, fill, slow, submit, waitFor } from "./Network/harness";

const PATH = "/settings/vnic";

class FailingVNICService extends VNICService {
  getConfig(): never {
    throw new Error("the node isn't answering");
  }
}

class FailingSaveVNICService extends VNICService {
  setConfig(): never {
    throw new Error("the profile's store is read-only");
  }
}

const config = (maxUploadBytesPerSecond: number, maxPeers: number) =>
  new vnicv1.Config({ maxUploadBytesPerSecond: BigInt(maxUploadBytesPerSecond), maxPeers });

// A VNIC service that says what each setConfig stores.
class StoringVNICService extends VNICService {
  onSet: (config: vnicv1.Config) => void = () => undefined;

  setConfig(req: vnicv1.SetConfigRequest): vnicv1.SetConfigResponse {
    const res = super.setConfig(req);
    this.onSet(this.config);
    return res;
  }
}

// `rate` typed and stored, and the bytes per second the node got below the page.
const StoredRate: React.FC<{ rate: string }> = ({ rate }) => {
  const [vnic] = useState(() => new StoringVNICService());
  const [stored, setStored] = useState<string>("nothing yet");
  useEffect(() => {
    vnic.onSet = (c) => setStored(`${c.maxUploadBytesPerSecond} bytes/s`);
  }, []);
  return (
    <SettingsStory
      path={PATH}
      services={() => ({ vnic })}
      steps={[fill("maxUploadBytesPerSecond", rate), submit()]}
    >
      <p className="settings_story__note">
        {rate} stored as {stored}
      </p>
    </SettingsStory>
  );
};

const stories: Story[] = [
  {
    // 50 Mbit/s up, 25 peers.
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    // A rate no unit divides but bytes.
    name: "Odd rate",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ vnic: new VNICService(config(1234567, 8)) })} />
    ),
  },
  {
    name: "Unset",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ vnic: new VNICService(config(0, 0)) })} />
    ),
  },
  {
    name: "Loading",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ vnic: slow(new VNICService(), ["getConfig"]) })}
      />
    ),
  },
  {
    name: "Error",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ vnic: new FailingVNICService() })} />
    ),
  },
  {
    name: "Invalid",
    component: () => (
      <SettingsStory path={PATH} steps={[fill("maxUploadBytesPerSecond", "fast"), submit()]} />
    ),
  },
  {
    // Two decimal points: not a number.
    name: "Malformed rate",
    component: () => (
      <SettingsStory path={PATH} steps={[fill("maxUploadBytesPerSecond", "1.2.3mbps"), submit()]} />
    ),
  },
  {
    // 1.5 Mbit/s is 187,500 bytes a second.
    name: "Decimal rate",
    component: () => <StoredRate rate="1.5mbps" />,
  },
  {
    // Thousands grouped with commas: 1,000 Kbit/s is 125,000 bytes a second.
    name: "Grouped rate",
    component: () => <StoredRate rate="1,000Kbps" />,
  },
  {
    // A peer budget must be a whole number of peers.
    name: "Invalid peers",
    component: () => <SettingsStory path={PATH} steps={[fill("maxPeers", "2.5"), submit()]} />,
  },
  {
    name: "Negative peers",
    component: () => <SettingsStory path={PATH} steps={[fill("maxPeers", "-3"), submit()]} />,
  },
  {
    name: "Save error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ vnic: new FailingSaveVNICService() })}
        steps={[submit()]}
      />
    ),
  },
  {
    // Storing 100 Mbit/s: the page loads the stored config again.
    name: "Saved",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          fill("maxUploadBytesPerSecond", "100mbps"),
          submit(),
          waitFor("input[name=maxUploadBytesPerSecond]"),
        ]}
      />
    ),
  },
];

export default stories;
