// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import * as bootstrapv1 from "../../../../../apis/strims/network/v1/bootstrap/bootstrap";
import BootstrapService from "../../../../mocks/bootstrap/service";
import { Catalog, Story } from "../../../../storybook";
import { SettingsStory, click, slow, submit, waitFor } from "../Network/harness";

const PATH = "/settings/bootstrap";

class FailingBootstrapService extends BootstrapService {
  getConfig(): never {
    throw new Error("the node isn't answering");
  }
}

class FailingSaveBootstrapService extends BootstrapService {
  setConfig(): never {
    throw new Error("the profile's store is read-only");
  }
}

const stories: Story[] = [
  {
    name: "Enabled",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Disabled",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({
          bootstrap: new BootstrapService({
            config: new bootstrapv1.Config({ enablePublishing: false }),
          }),
        })}
      />
    ),
  },
  {
    name: "Loading",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ bootstrap: slow(new BootstrapService(), ["getConfig"]) })}
      />
    ),
  },
  {
    name: "Error",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ bootstrap: new FailingBootstrapService() })} />
    ),
  },
  {
    name: "Save error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ bootstrap: new FailingSaveBootstrapService() })}
        steps={[submit()]}
      />
    ),
  },
  {
    // Turning publishing off and storing it: the page loads the stored config again.
    name: "Saved",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          click("input[name=enablePublishing]"),
          submit(),
          waitFor(".thing_table__title_bar__title", "Edit Bootstrap Config"),
        ]}
      />
    ),
  },
];

export default stories;

export const catalog: Catalog = {
  covers: ["pages/Settings/Bootstrap/index.tsx"],
};
