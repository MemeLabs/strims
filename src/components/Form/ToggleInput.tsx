import "./ToggleInput.scss";

import clsx from "clsx";
import React, { ComponentProps, ReactElement } from "react";
import { FieldValues, useController } from "react-hook-form";

import { CompatibleUseControllerProps } from "./Form";
import InputError from "./InputError";
import InputLabel from "./InputLabel";

export interface ToggleInputProps extends ComponentProps<"input"> {
  label: string;
  description?: string;
}

const ToggleInput = <T extends FieldValues>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: ToggleInputProps & CompatibleUseControllerProps<T, boolean>): ReactElement => {
  const {
    field: { value, ...field },
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    // @ts-ignore
    defaultValue: defaultValue || false,
    control,
  });

  return (
    <InputLabel text={label} description={description} inputType="toggle">
      <input
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
        checked={value}
        className="input input_toggle"
        type="checkbox"
      />
      <div className={clsx(className, "input_toggle__switch")}>
        <div className="input_toggle__switch__track" />
      </div>
      <InputError error={error} />
    </InputLabel>
  );
};

export default ToggleInput;
