// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Search.scss";

import clsx from "clsx";
import React, { ComponentProps } from "react";
import { FiSearch } from "react-icons/fi";

const Search: React.FC<ComponentProps<"input">> = ({ className, ...props }) => (
  <label className={clsx("settings_search", className)}>
    <input className="settings_search__input" type="search" {...props} placeholder="Search" />
    <FiSearch className="settings_search__icon" />
  </label>
);

export default Search;
