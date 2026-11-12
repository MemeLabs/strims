// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Dropdown.scss";

import clsx from "clsx";
import React, { useRef } from "react";
import { useToggle } from "react-use";

import useClickAway from "../../hooks/useClickAway";

export interface DropdownProps {
  baseClassName?: string;
  anchor: React.ReactNode;
  items: React.ReactNode | React.ReactNode[];
}

const Dropdown: React.FC<DropdownProps> = ({ baseClassName, anchor, items }) => {
  const ref = useRef<HTMLDivElement>(null);

  const [open, toggleOpen] = useToggle(false);
  useClickAway(ref, () => toggleOpen(false), { events: ["click"] });

  // The base's classes beside the dropdown's own, when there is a base.
  const base = (suffix = "") => baseClassName && `${baseClassName}${suffix}`;

  return (
    <div
      className={clsx("dropdown", base(), open && ["dropdown--open", base("--open")])}
      ref={ref}
      onClick={() => toggleOpen()}
    >
      <div className={clsx("dropdown__anchor", base("__anchor"))}>{anchor}</div>
      <div className={clsx("dropdown__menu", base("__menu"))}>{items}</div>
    </div>
  );
};

export default Dropdown;
