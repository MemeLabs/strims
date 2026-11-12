// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { CreatableSelectInput, SelectOption } from "../../../../components/Form";
import { Story } from "../../../storybook";
import { FormStory } from "./harness";

const tag = (value: string): SelectOption<string> => ({ label: value, value });

const tags = ["speedrun", "any%", "retro", "comfy", "music", "late night"].map(tag);

interface Values {
  tags: SelectOption<string>[];
}

const required = { required: { value: true, message: "Add at least one tag" } };

const stories: Story[] = [
  {
    name: "Empty",
    component: () => (
      <FormStory<Values>>
        {(control) => <CreatableSelectInput control={control} name="tags" label="Tags" />}
      </FormStory>
    ),
  },
  {
    name: "With values",
    component: () => (
      <FormStory<Values> defaultValues={{ tags: tags.slice(0, 3) }}>
        {(control) => <CreatableSelectInput control={control} name="tags" label="Tags" />}
      </FormStory>
    ),
  },
  {
    name: "Description",
    component: () => (
      <FormStory<Values> defaultValues={{ tags: tags.slice(0, 2) }}>
        {(control) => (
          <CreatableSelectInput
            control={control}
            name="tags"
            label="Tags"
            description="Type a tag and press enter to add it."
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
          <CreatableSelectInput control={control} name="tags" label="Tags" rules={required} />
        )}
      </FormStory>
    ),
  },
  {
    name: "Error",
    component: () => (
      <FormStory<Values> validate>
        {(control) => (
          <CreatableSelectInput control={control} name="tags" label="Tags" rules={required} />
        )}
      </FormStory>
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <FormStory<Values> defaultValues={{ tags: tags.slice(0, 3) }}>
        {(control) => (
          <CreatableSelectInput control={control} name="tags" label="Tags" isDisabled />
        )}
      </FormStory>
    ),
  },
  {
    name: "Many values",
    component: () => (
      <FormStory<Values>
        defaultValues={{
          tags: [
            ...tags,
            ...["a tag long enough to take a row of its own", "x", "y", "z"].map(tag),
          ],
        }}
      >
        {(control) => <CreatableSelectInput control={control} name="tags" label="Tags" />}
      </FormStory>
    ),
  },
  {
    name: "Menu open",
    component: () => (
      <FormStory<Values> defaultValues={{ tags: tags.slice(0, 2) }}>
        {(control) => (
          <CreatableSelectInput
            control={control}
            name="tags"
            label="Tags"
            options={tags}
            menuIsOpen
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Create option",
    component: () => (
      <FormStory<Values> defaultValues={{ tags: tags.slice(0, 2) }}>
        {(control) => (
          <CreatableSelectInput
            control={control}
            name="tags"
            label="Tags"
            options={tags}
            inputValue="speed"
            menuIsOpen
          />
        )}
      </FormStory>
    ),
  },
];

export default stories;
