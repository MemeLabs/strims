// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Button.scss";

import clsx from "clsx";
import React, { ComponentProps, ReactNode } from "react";

interface ButtonSetProps {
  children: ReactNode;
}

export const ButtonSet: React.FC<ButtonSetProps> = ({ children }: ButtonProps) => (
  <div className="input_buttonset">{children}</div>
);

export interface ButtonProps extends ComponentProps<"button"> {
  primary?: boolean;
  borderless?: boolean;
}

const Button: React.FC<ButtonProps> = ({
  children,
  className,
  primary,
  borderless,
  ...inputProps
}: ButtonProps) => (
  <button
    className={clsx("input_button", className, {
      "input_button--primary": primary,
      "input_button--borderless": borderless,
    })}
    {...inputProps}
  >
    {children}
  </button>
);

export default Button;
