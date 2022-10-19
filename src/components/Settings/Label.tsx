// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Label.scss";

import clsx from "clsx";
import React, { ReactNode } from "react";

interface labelProps {
  children: ReactNode;
  className?: string;
}

const Label: React.FC<labelProps> = ({ children, className }) => (
  <span className={clsx("label", className)}>{children}</span>
);

export default Label;
