import clsx from "clsx";
import * as React from "react";
import { FiAlertTriangle } from "react-icons/fi";

export const InputLabel = ({
  children,
  required,
  text,
}: {
  children: any,
  required: boolean,
  text: string,
}) => {
  const labelClass = clsx({
    "input_label": true,
    "input_label--required": required,
  });

  return (
    <label className={labelClass}>
      <span className="input_label__text">{text}</span>
      {children}
    </label>
  );
};

export const InputError = ({ error }: {error: any}) => {
  if (!error) {
    return null;
  }

  let message = "Invalid value";
  if (typeof error === "string") {
    message = error;
  } else if (error.message) {
    message = error.message;
  }

  return (
    <span className="input_error">
      <FiAlertTriangle className="input_error__icon" />
      <span className="input_error__text">{message}</span>
    </span>
  );
};

export const TextInput = ({
  error,
  inputRef,
  label,
  name,
  placeholder,
  required,
  type,
  disabled,
  defaultValue,
}: {
  error?: any,
  inputRef: React.Ref<HTMLInputElement>,
  label: string,
  name: string,
  placeholder: string,
  required?: boolean,
  type?: "text" | "password",
  disabled?: boolean,
  defaultValue?: string,
}) => {
  return (
    <InputLabel required={required} text={label}>
      <input
        className="input input_text"
        name={name}
        placeholder={placeholder}
        ref={inputRef}
        type={type}
        disabled={disabled}
        defaultValue={defaultValue}
      />
      <InputError error={error} />
    </InputLabel>
  );
};
