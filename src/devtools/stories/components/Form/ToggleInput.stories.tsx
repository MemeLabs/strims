// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { ToggleInput } from "../../../../components/Form";
import { Story } from "../../../storybook";
import { FormStory } from "./harness";

interface Values {
  enabled: boolean;
}

const stories: Story[] = [
  {
    name: "Off",
    component: () => (
      <FormStory<Values>>
        {(control) => <ToggleInput control={control} name="enabled" label="Autoseed" />}
      </FormStory>
    ),
  },
  {
    name: "On",
    component: () => (
      <FormStory<Values> defaultValues={{ enabled: true }}>
        {(control) => <ToggleInput control={control} name="enabled" label="Autoseed" />}
      </FormStory>
    ),
  },
  {
    name: "Description",
    component: () => (
      <FormStory<Values> defaultValues={{ enabled: true }}>
        {(control) => (
          <ToggleInput
            control={control}
            name="enabled"
            label="Autoseed"
            description="Seed the streams your networks publish while the app is open."
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <FormStory<Values>>
        {(control) => <ToggleInput control={control} name="enabled" label="Autoseed" disabled />}
      </FormStory>
    ),
  },
  {
    name: "Disabled on",
    component: () => (
      <FormStory<Values> defaultValues={{ enabled: true }}>
        {(control) => <ToggleInput control={control} name="enabled" label="Autoseed" disabled />}
      </FormStory>
    ),
  },
  {
    name: "Error",
    component: () => (
      <FormStory<Values> validate>
        {(control) => (
          <ToggleInput
            control={control}
            name="enabled"
            label="Accept the rules"
            rules={{ validate: (v: boolean) => v || "Accept the network's rules to join it" }}
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Bare",
    component: () => (
      <FormStory<Values> bare defaultValues={{ enabled: true }}>
        {(control) => <ToggleInput control={control} name="enabled" label="Stay logged in" />}
      </FormStory>
    ),
  },
];

export default stories;
