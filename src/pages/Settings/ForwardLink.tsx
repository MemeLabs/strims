// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { MdChevronRight } from "react-icons/md";
import { Link } from "react-router-dom";

interface ForwardLinkProps {
  to: string;
  title: string;
  description?: string;
}

const ForwardLink: React.FC<ForwardLinkProps> = ({ to, title, description }) => (
  <Link className="input_label input_label--button" to={to}>
    <div className="input_label__body">
      <div>{title}</div>
      {description && <div>{description}</div>}
    </div>
    <MdChevronRight size="28" />
  </Link>
);

export default ForwardLink;
