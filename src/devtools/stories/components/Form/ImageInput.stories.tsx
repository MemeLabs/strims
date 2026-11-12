// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, useEffect, useRef } from "react";

import {
  ImageInput,
  ImageInputPlaceholder,
  ImageInputPreview,
  ImageValue,
  InputLabel,
} from "../../../../components/Form";
import { Story, fixtures } from "../../../storybook";
import { FormStory, useImageValues } from "./harness";

interface Values {
  image: ImageValue;
}

const required = { required: { value: true, message: "Image is required" } };

const Field: React.FC<{ children: ReactNode }> = ({ children }) => (
  <InputLabel text="Image" component="div">
    {children}
  </InputLabel>
);

const WithImage: React.FC<{ classNameBase?: string }> = ({ classNameBase }) => {
  const values = useImageValues([fixtures.iconUrls[0]]);
  if (!values) {
    return null;
  }
  return (
    <FormStory<Values> defaultValues={{ image: values[0] }}>
      {(control) => (
        <Field>
          <ImageInput control={control} name="image" classNameBase={classNameBase} />
        </Field>
      )}
    </FormStory>
  );
};

// Gives the dropzone's file input a file over its maxSize, as a drop would.
const DropTooLarge: React.FC<{ children: ReactNode }> = ({ children }) => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const input = ref.current.querySelector<HTMLInputElement>("input[type=file]");
    const transfer = new DataTransfer();
    transfer.items.add(new File([new Uint8Array(600 * 1024)], "large.png", { type: "image/png" }));
    input.files = transfer.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }, []);
  return <div ref={ref}>{children}</div>;
};

const stories: Story[] = [
  {
    name: "Placeholder",
    component: () => (
      <FormStory<Values>>
        {(control) => (
          <Field>
            <ImageInput control={control} name="image" />
          </Field>
        )}
      </FormStory>
    ),
  },
  {
    name: "Preview",
    component: () => <WithImage />,
  },
  {
    name: "Avatar placeholder",
    component: () => (
      <FormStory<Values>>
        {(control) => (
          <Field>
            <ImageInput control={control} name="image" classNameBase="input_avatar" />
          </Field>
        )}
      </FormStory>
    ),
  },
  {
    name: "Avatar preview",
    component: () => <WithImage classNameBase="input_avatar" />,
  },
  {
    name: "Required",
    component: () => (
      <FormStory<Values> validate>
        {(control) => (
          <Field>
            <ImageInput control={control} name="image" rules={required} />
          </Field>
        )}
      </FormStory>
    ),
  },
  {
    name: "Max size",
    component: () => (
      <DropTooLarge>
        <FormStory<Values>>
          {(control) => (
            <Field>
              <ImageInput control={control} name="image" />
            </Field>
          )}
        </FormStory>
      </DropTooLarge>
    ),
  },
  {
    name: "Parts",
    component: () => (
      <div className="form_story form_story__row">
        <div className="input_image">
          <ImageInputPlaceholder classNameBase="input_image" />
        </div>
        <div className="input_image">
          <ImageInputPreview src={fixtures.iconUrls[1]} classNameBase="input_image" />
        </div>
        <div className="input_avatar">
          <ImageInputPlaceholder classNameBase="input_avatar" />
        </div>
        <div className="input_avatar">
          <ImageInputPreview src={fixtures.iconUrls[2]} classNameBase="input_avatar" />
        </div>
      </div>
    ),
  },
];

export default stories;
