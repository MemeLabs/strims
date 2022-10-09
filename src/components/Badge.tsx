// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Badge.scss";

import clsx from "clsx";
import React from "react";

export interface BadgeProps {
  count: number;
  max?: number;
  hidden?: boolean;
}

const Badge: React.FC<BadgeProps> = ({ count, max = count, hidden = false }) => (
  <span className={clsx("badge", { "badge--hidden": hidden })}>
    {Math.min(count, max).toLocaleString()}
    {count > max && "+"}
  </span>
);

export default Badge;
