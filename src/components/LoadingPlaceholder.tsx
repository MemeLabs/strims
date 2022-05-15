// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./LoadingPlaceholder.scss";

import clsx from "clsx";
import React from "react";

import { withTheme } from "./Theme";

interface LoadingPlaceholderProps {
  className: string;
}

const LoadingPlaceholder: React.FC<LoadingPlaceholderProps> = ({ className }) => (
  <div className={clsx(className, "loading_placeholder")}>loading</div>
);

export default withTheme(LoadingPlaceholder);
