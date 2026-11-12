// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { SelectInput, SelectOption } from "../../../../components/Form";
import { Story } from "../../../storybook";
import { FormStory } from "./harness";

const options: SelectOption<string>[] = [
  { label: "Source", value: "source" },
  { label: "1080p", value: "1080" },
  { label: "720p", value: "720" },
  { label: "480p", value: "480" },
  { label: "Audio only", value: "audio" },
];

interface Values {
  quality: SelectOption<string>;
  qualities: SelectOption<string>[];
}

const required = { required: { value: true, message: "Quality is required" } };

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <FormStory<Values>>
        {(control) => (
          <SelectInput control={control} name="quality" label="Quality" options={options} />
        )}
      </FormStory>
    ),
  },
  {
    name: "With value",
    component: () => (
      <FormStory<Values> defaultValues={{ quality: options[2] }}>
        {(control) => (
          <SelectInput control={control} name="quality" label="Quality" options={options} />
        )}
      </FormStory>
    ),
  },
  {
    name: "Description",
    component: () => (
      <FormStory<Values> defaultValues={{ quality: options[0] }}>
        {(control) => (
          <SelectInput
            control={control}
            name="quality"
            label="Quality"
            description="The rendition the player starts with."
            options={options}
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Required",
    component: () => (
      <FormStory<Values>>
        {(control) => (
          <SelectInput
            control={control}
            name="quality"
            label="Quality"
            options={options}
            rules={required}
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Error",
    component: () => (
      <FormStory<Values> validate>
        {(control) => (
          <SelectInput
            control={control}
            name="quality"
            label="Quality"
            options={options}
            rules={required}
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <FormStory<Values> defaultValues={{ quality: options[1] }}>
        {(control) => (
          <SelectInput
            control={control}
            name="quality"
            label="Quality"
            options={options}
            isDisabled
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Multi",
    component: () => (
      <FormStory<Values> defaultValues={{ qualities: options.slice(1, 4) }}>
        {(control) => (
          <SelectInput
            control={control}
            name="qualities"
            label="Qualities"
            options={options}
            isMulti
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Menu open",
    component: () => (
      <FormStory<Values> defaultValues={{ quality: options[2] }}>
        {(control) => (
          <SelectInput
            control={control}
            name="quality"
            label="Quality"
            options={options}
            menuIsOpen
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Loading",
    component: () => (
      <FormStory<Values>>
        {(control) => (
          <SelectInput
            control={control}
            name="quality"
            label="Quality"
            options={[]}
            isLoading
            menuIsOpen
          />
        )}
      </FormStory>
    ),
  },
];

export default stories;
