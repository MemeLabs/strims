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

  return (
    <div
      className={clsx({
        "dropdown": true,
        "dropdown--open": open,
        [baseClassName]: true,
        [`${baseClassName}--open`]: open,
      })}
      ref={ref}
      onClick={() => toggleOpen()}
    >
      <div className={clsx("dropdown__anchor", `${baseClassName}__anchor`)}>{anchor}</div>
      <div className={clsx("dropdown__menu", `${baseClassName}__menu`)}>{items}</div>
    </div>
  );
};

export default Dropdown;
