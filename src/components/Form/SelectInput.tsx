// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./SelectInput.scss";

import clsx from "clsx";
import React, { ReactElement } from "react";
import { FieldValues, useController } from "react-hook-form";
import Select, { Props as SelectProps } from "react-select";

import { useLayout } from "../../contexts/Layout";
import { CompatibleUseControllerProps } from "./Form";
import { isRequired } from "./Form";
import InputError from "./InputError";
import InputLabel from "./InputLabel";

export interface SelectOption<T> {
  label: string;
  value: T;
}

export interface SelectInputProps<T extends SelectOption<any>, M extends boolean>
  extends SelectProps<T, M> {
  label: string;
  description?: string;
}

const SelectInput = <T extends FieldValues, F extends SelectOption<any>, M extends boolean>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: SelectInputProps<F, M> &
  CompatibleUseControllerProps<T, M extends true ? F[] : F>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    defaultValue,
    control,
  });

  const { root } = useLayout();

  return (
    <InputLabel
      required={isRequired(rules)}
      text={label}
      description={description}
      inputType="select"
    >
      <Select
        classNamePrefix="input_select"
        menuPortalTarget={root}
        menuPlacement="auto"
        {...inputProps}
        {...field}
        onChange={(value, action) => {
          field.onChange(value);
          inputProps.onChange?.(value, action);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        className={clsx(className, "input_select")}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export default SelectInput;
