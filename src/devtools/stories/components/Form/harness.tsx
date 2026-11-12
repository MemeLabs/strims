// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Form.stories.scss";

import { Base64 } from "js-base64";
import React, { ReactElement, ReactNode, useEffect, useState } from "react";
import { Control, DefaultValues, FieldValues, useForm } from "react-hook-form";

import { ImageValue } from "../../../../components/Form";
import { fixtures } from "../../../storybook";

export interface FormStoryProps<T extends FieldValues> {
  defaultValues?: DefaultValues<T>;
  /** Validates every field on mount, so their rules' errors show untouched. */
  validate?: boolean;
  /** Without the settings pages' form layout (thing_form), as the landing page has it. */
  bare?: boolean;
  children: (control: Control<T>) => ReactNode;
}

// A react-hook-form form around inputs given its control.
export const FormStory = <T extends FieldValues>({
  defaultValues,
  validate,
  bare,
  children,
}: FormStoryProps<T>): ReactElement => {
  const { control, trigger } = useForm<T>({ mode: "onBlur", defaultValues });
  useEffect(() => {
    if (validate) {
      void trigger();
    }
  }, []);
  return (
    <form
      className={bare ? "form_story" : "form_story thing_form"}
      onSubmit={(e) => e.preventDefault()}
    >
      {children(control)}
    </form>
  );
};

// A fixture image as a form's ImageValue.
export const imageValue = async (url: string): Promise<ImageValue> => {
  const data = await fixtures.imageData(url);
  const img = new Image();
  img.src = url;
  await img.decode();
  return {
    data: Base64.fromUint8Array(data),
    type: "image/png",
    height: img.naturalHeight,
    width: img.naturalWidth,
  };
};

// The fixture images as ImageValues; null until they load.
export const useImageValues = (urls: string[]): ImageValue[] | null => {
  const [values, setValues] = useState<ImageValue[]>(null);
  useEffect(() => {
    void Promise.all(urls.map(imageValue)).then(setValues);
  }, []);
  return values;
};
