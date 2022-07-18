// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./InternalLink.scss";

import clsx from "clsx";
import React from "react";
import { Link, LinkProps } from "react-router-dom";

const InternalLink: React.FC<LinkProps> = ({ className, ...props }) => (
  <Link className={clsx("internal_link", className)} {...props} />
);

export default InternalLink;
