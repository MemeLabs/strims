// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../Form/Form.stories.scss";

import React, { useEffect } from "react";
import { FormProvider, useForm } from "react-hook-form";

import {
  ChatStyleSheetFormData,
  ChatStyleSheetInput,
  toStyleSheetFormValue,
} from "../../../../components/Settings/ChatStyleSheet";
import { Story, fixtures } from "../../../storybook";
import { useImageValues } from "../Form/harness";

const scss = `$accent: #ff9158;

.chat__message {
  .emote_ComfyCat {
    filter: drop-shadow(0 0 2px $accent);
  }

  &--highlight {
    border-left: 2px solid rgba($accent, 0.6);
  }
}`;

interface FormProps {
  defaultValues: ChatStyleSheetFormData;
  validate?: boolean;
}

const Form: React.FC<FormProps> = ({ defaultValues, validate }) => {
  const form = useForm<ChatStyleSheetFormData>({ mode: "onBlur", defaultValues });
  useEffect(() => {
    if (validate) {
      void form.trigger();
    }
  }, []);
  return (
    <FormProvider {...form}>
      <form className="form_story thing_form" onSubmit={(e) => e.preventDefault()}>
        <ChatStyleSheetInput />
      </form>
    </FormProvider>
  );
};

const WithAssets: React.FC<{ count: number }> = ({ count }) => {
  const images = useImageValues(fixtures.iconUrls.slice(0, count));
  if (!images) {
    return null;
  }
  const values: ChatStyleSheetFormData = { scss, assetCount: count };
  const names = ["sparkle", "frame", "badge", "glow"];
  images.forEach((image, i) => {
    values[`asset_${i as 0 | 1 | 2 | 3}_name`] = names[i];
    values[`asset_${i as 0 | 1 | 2 | 3}_image`] = image;
  });
  return <Form defaultValues={values} />;
};

const stories: Story[] = [
  {
    name: "Empty",
    component: () => <Form defaultValues={toStyleSheetFormValue()} />,
  },
  {
    name: "SCSS",
    component: () => <Form defaultValues={{ scss, assetCount: 0 }} validate />,
  },
  {
    name: "With assets",
    component: () => <WithAssets count={2} />,
  },
  {
    name: "Asset limit",
    component: () => <WithAssets count={4} />,
  },
  {
    name: "Compile error",
    component: () => (
      <Form
        defaultValues={{ scss: ".chat__message {\n  color: $undefined;\n", assetCount: 0 }}
        validate
      />
    ),
  },
  {
    name: "Asset errors",
    component: () => <Form defaultValues={{ scss, assetCount: 1 }} validate />,
  },
];

export default stories;
