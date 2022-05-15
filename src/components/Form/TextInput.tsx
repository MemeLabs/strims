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

export interface TextInputProps extends ComponentProps<"input"> {
  label: string;
  description?: string;
  type?: "text" | "password" | "number" | "search";
  format?: "text";
}

const TextInput = <T extends FieldValues>({
  label,
  description,
  type,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: TextInputProps & CompatibleUseControllerProps<T, string | number>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    // @ts-ignore
    defaultValue: defaultValue || (type === "number" ? 0 : ""),
    control,
  });

  return (
    <InputLabel
      required={isRequired(rules)}
      text={label}
      description={description}
      inputType="text"
    >
      <input
        {...inputProps}
        {...field}
        onChange={(e) => {
          field.onChange(type === "number" ? parseFloat(e.target.value) : e.target.value);
          inputProps.onChange?.(e);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        type={type}
        className={clsx(className, "input", "input_text")}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export default TextInput;
