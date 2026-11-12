// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { TextAreaInput } from "../../../../components/Form";
import { Story } from "../../../storybook";
import { FormStory } from "./harness";

interface Values {
  description: string;
}

const required = { required: { value: true, message: "Description is required" } };

const longText = [
  "Every Saturday we watch a film nobody in the group has seen, picked by whoever lost last week's quiz.",
  "Chat is open during the film, spoilers in the spoiler room only. Subtitles stay on; the host decides when we pause for snacks.",
  "Links to the next film's trailer and the schedule are pinned in the network's chat, and the archive of past picks lives on the network's directory page, with ratings from everyone who stayed to the end.",
  "averyveryverylongwordwithnospacesthatcannotwrapanywhereandsoshowshowthetextareahandlesoverflowingwordsinsteadoflines",
].join("\n\n");

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <FormStory<Values>>
        {(control) => (
          <TextAreaInput
            control={control}
            name="description"
            label="Description"
            placeholder="Describe the channel"
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "With value",
    component: () => (
      <FormStory<Values> defaultValues={{ description: "Film night, Saturdays at 8." }}>
        {(control) => <TextAreaInput control={control} name="description" label="Description" />}
      </FormStory>
    ),
  },
  {
    name: "Description",
    component: () => (
      <FormStory<Values> defaultValues={{ description: "Film night, Saturdays at 8." }}>
        {(control) => (
          <TextAreaInput
            control={control}
            name="description"
            label="Description"
            description="Shown under the channel's title in the directory."
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
          <TextAreaInput
            control={control}
            name="description"
            label="Description"
            placeholder="Describe the channel"
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
          <TextAreaInput
            control={control}
            name="description"
            label="Description"
            placeholder="Describe the channel"
            rules={required}
          />
        )}
      </FormStory>
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <FormStory<Values> defaultValues={{ description: "Film night, Saturdays at 8." }}>
        {(control) => (
          <TextAreaInput control={control} name="description" label="Description" disabled />
        )}
      </FormStory>
    ),
  },
  {
    name: "Long text",
    component: () => (
      <FormStory<Values> defaultValues={{ description: longText }}>
        {(control) => (
          <TextAreaInput control={control} name="description" label="Description" rows={8} />
        )}
      </FormStory>
    ),
  },
];

export default stories;
