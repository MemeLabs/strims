// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { TextInput } from "../../../../components/Form";
import { validNamePattern } from "../../../../lib/validation";
import { Story } from "../../../storybook";
import { FormStory } from "./harness";

interface Values {
  name: string;
  password: string;
  count: number;
  query: string;
}

const nameRules = {
  required: { value: true, message: "Name is required" },
  pattern: { value: validNamePattern, message: "Name contains invalid characters" },
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <FormStory<Values>>
        {(control) => (
          <TextInput
            control={control}
            name="name"
            label="Profile name"
            placeholder="Enter a profile name"
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "With value",
    component: () => (
      <FormStory<Values> defaultValues={{ name: "test_user" }}>
        {(control) => <TextInput control={control} name="name" label="Profile name" />}
      </FormStory>
    ),
  },
  {
    name: "Description",
    component: () => (
      <FormStory<Values> defaultValues={{ name: "test_user" }}>
        {(control) => (
          <TextInput
            control={control}
            name="name"
            label="Profile name"
            description="3 to 20 letters, digits or underscores. Other members see it beside your messages."
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
          <TextInput
            control={control}
            name="name"
            label="Profile name"
            placeholder="Enter a profile name"
            rules={nameRules}
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
          <TextInput
            control={control}
            name="name"
            label="Profile name"
            placeholder="Enter a profile name"
            rules={nameRules}
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Pattern error",
    component: () => (
      <FormStory<Values> defaultValues={{ name: "not a valid name!" }} validate>
        {(control) => (
          <TextInput control={control} name="name" label="Profile name" rules={nameRules} />
        )}
      </FormStory>
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <FormStory<Values> defaultValues={{ name: "test_user" }}>
        {(control) => <TextInput control={control} name="name" label="Profile name" disabled />}
      </FormStory>
    ),
  },
  {
    name: "Password",
    component: () => (
      <FormStory<Values> defaultValues={{ password: "hunter2hunter2" }}>
        {(control) => (
          <TextInput
            control={control}
            name="password"
            label="Password"
            type="password"
            autoComplete="off"
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Number",
    component: () => (
      <FormStory<Values> defaultValues={{ count: 1024 }}>
        {(control) => (
          <TextInput control={control} name="count" label="Bitrate (kbps)" type="number" />
        )}
      </FormStory>
    ),
  },
  {
    name: "Disabled number",
    component: () => (
      <FormStory<Values> defaultValues={{ count: 1024 }}>
        {(control) => (
          <TextInput control={control} name="count" label="Bitrate (kbps)" type="number" disabled />
        )}
      </FormStory>
    ),
  },
  {
    name: "Search",
    component: () => (
      <FormStory<Values> defaultValues={{ query: "speedrun" }}>
        {(control) => <TextInput control={control} name="query" label="Search" type="search" />}
      </FormStory>
    ),
  },
  {
    name: "Format",
    component: () => (
      <FormStory<Values> defaultValues={{ name: "plain text format" }}>
        {(control) => <TextInput control={control} name="name" label="Title" format="text" />}
      </FormStory>
    ),
  },
  {
    name: "Long text",
    component: () => (
      <FormStory<Values>
        defaultValues={{
          name: "A very long value that runs well past the width of the input so its overflow shows",
        }}
      >
        {(control) => (
          <TextInput
            control={control}
            name="name"
            label="A label long enough that it wraps across more than one line of its column"
          />
        )}
      </FormStory>
    ),
  },
];

export default stories;
