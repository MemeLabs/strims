// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./SelectInput.scss";

import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { ReactElement, useEffect, useState } from "react";
import { FieldValues, useController } from "react-hook-form";
import Select, { PropsValue, Props as SelectProps } from "react-select";

import { useCall } from "../../contexts/FrontendApi";
import { useLayout } from "../../contexts/Layout";
import { certificateRoot } from "../../lib/certificate";
import { CompatibleUseControllerProps } from "./Form";
import { isRequired } from "./Form";
import InputError from "./InputError";
import InputLabel from "./InputLabel";

export interface SelectOption<T> {
  label: string;
  value: T;
}

export interface NetworkSelectInputProps<M extends boolean> extends SelectProps<string, M> {
  label: string;
  description?: string;
  onChange?: (value: Value<M>) => void;
}

type Value<M extends boolean> = M extends true ? string[] : string;

const NetworkSelectInput = <T extends FieldValues, M extends boolean>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: NetworkSelectInputProps<M> & CompatibleUseControllerProps<T, Value<M>>): ReactElement => {
  const {
    field: { value: controlValue, ...field },
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    defaultValue,
    control,
  });

  const [options, setOptions] = useState<SelectOption<string>[]>();
  useCall("network", "list", {
    onComplete: (res) =>
      setOptions(
        res.networks.map((n) => {
          const certRoot = certificateRoot(n.certificate);
          return {
            value: Base64.fromUint8Array(certRoot.key),
            label: certRoot.subject,
          };
        })
      ),
  });

  const [value, setValue] = useState<PropsValue<SelectOption<string>>>(null);
  useEffect(() => {
    if (controlValue && options) {
      if (Array.isArray(controlValue)) {
        setValue(options.filter(({ value }) => (controlValue as string[]).includes(value)));
      } else {
        setValue(options.find(({ value }) => value === controlValue));
      }
    }
  }, [controlValue, options]);

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
        {...(inputProps as unknown)}
        {...field}
        value={value}
        options={options}
        onChange={(option) => {
          const value = (Array.isArray(option)
            ? (option as SelectOption<string>[]).map(({ value }) => value)
            : option.value) as unknown as Value<M>;
          field.onChange(value);
          inputProps.onChange?.(value);
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

export default NetworkSelectInput;
