import "./CreatableSelectInput.scss";

import clsx from "clsx";
import React, { ReactElement } from "react";
import { FieldValues, useController } from "react-hook-form";
import { Props as SelectProps } from "react-select";
import CreatableSelect from "react-select/creatable";

import { useLayout } from "../../contexts/Layout";
import { CompatibleUseControllerProps, isRequired } from "./Form";
import InputError from "./InputError";
import InputLabel from "./InputLabel";
import { SelectOption } from "./SelectInput";

export interface CreatableSelectInputProps<T> extends SelectProps<T, true> {
  label: string;
  description?: string;
}

const CreatableSelectInput = <T extends FieldValues>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: CreatableSelectInputProps<T> &
  CompatibleUseControllerProps<T, SelectOption<string>[]>): ReactElement => {
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
      <CreatableSelect
        classNamePrefix="input_select"
        menuPortalTarget={root}
        menuPlacement="auto"
        {...(inputProps as unknown)}
        {...field}
        onChange={(value, action) => {
          field.onChange(value);
          inputProps.onChange?.(value, action);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        isMulti={true}
        className={clsx(className, "input_select")}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export default CreatableSelectInput;
