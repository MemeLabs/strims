// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { MdChevronLeft } from "react-icons/md";
import { Link } from "react-router-dom";

interface BackLinkProps {
  to: string;
  title: string;
  description: string;
}

const BackLink: React.FC<BackLinkProps> = ({ to, title, description }) => (
  <Link className="input_label input_label--button" to={to}>
    <MdChevronLeft size="28" />
    <div className="input_label__body">
      <div>{title}</div>
      <div>{description}</div>
    </div>
  </Link>
);

export default BackLink;
