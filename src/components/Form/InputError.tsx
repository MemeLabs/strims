// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./InputError.scss";

import React from "react";
import { FieldError } from "react-hook-form";
import { FiAlertTriangle } from "react-icons/fi";

export interface InputErrorProps {
  error: FieldError | Error | string;
}

const InputError: React.FC<InputErrorProps> = ({ error }) => {
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

export default InputError;
