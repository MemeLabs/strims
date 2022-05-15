// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./TextInput.scss";

import clsx from "clsx";
import React, { ComponentProps, ReactElement } from "react";
import { FieldValues, useController } from "react-hook-form";

import { CompatibleUseControllerProps } from "./Form";
import { isRequired } from "./Form";
import InputError from "./InputError";
import InputLabel from "./InputLabel";

export interface TextAreaInputProps extends ComponentProps<"textarea"> {
  label: string;
  description?: string;
}

const TextAreaInput = <T extends FieldValues>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: TextAreaInputProps & CompatibleUseControllerProps<T, string>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    // @ts-ignore
    defaultValue: defaultValue || "",
    control,
  });

  return (
    <InputLabel
      required={isRequired(rules)}
      text={label}
      description={description}
      inputType="textarea"
    >
      <textarea
        {...inputProps}
        {...field}
        onChange={(e) => {
          field.onChange(e);
          inputProps.onChange?.(e);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        className={clsx(className, "input", "input_textarea")}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export default TextAreaInput;
