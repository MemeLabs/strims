// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Table.scss";

import clsx from "clsx";
import React, { ReactNode } from "react";
import { BsThreeDots } from "react-icons/bs";
import { MdChevronLeft } from "react-icons/md";
import { Link } from "react-router-dom";

import Dropdown from "../Dropdown";

interface TableProps {
  children: ReactNode;
}

export const Table: React.FC<TableProps> = ({ children }) => (
  <table className="thing_table">{children}</table>
);

interface MenuCellProps {
  children: ReactNode;
}

export const MenuCell: React.FC<MenuCellProps> = ({ children }) => (
  <td className="thing_table__row_menu">
    <Dropdown baseClassName="thing_table_item_dropdown" anchor={<BsThreeDots />} items={children} />
  </td>
);

export interface TableTitleBarProps {
  label: string;
  backLink?: string;
  children?: ReactNode;
}

export const TableTitleBar: React.FC<TableTitleBarProps> = ({ label, backLink, children }) => (
  <div className="thing_table__title_bar">
    <div className="thing_table__title_bar__header">
      {backLink && (
        <Link to={backLink} className="thing_table__title_bar__back">
          <MdChevronLeft size={24} />
        </Link>
      )}
      <h2 className="thing_table__title_bar__title">{label}</h2>
    </div>
    <div className="thing_table__title_bar__controls">{children}</div>
  </div>
);

export interface TableCellProps {
  truncate?: boolean;
  children: ReactNode;
}

export const TableCell: React.FC<TableCellProps> = ({ truncate, children }) => {
  if (truncate) {
    return (
      <td>
        <div className="thing_table__truncate">
          <span className="thing_table__truncate__reference">{children}</span>
          <span className="thing_table__truncate__display">{children}</span>
        </div>
      </td>
    );
  }

  return <td>{children}</td>;
};

export interface TableMenuProps {
  label: string;
  children: ReactNode;
}

export const TableMenu: React.FC<TableMenuProps> = ({ label = "Create", children }) => (
  <Dropdown baseClassName="thing_table_dropdown" anchor={label} items={children} />
);

export interface MenuItemProps {
  label: string;
  className?: string;
  onClick: () => void;
}

export const MenuItem: React.FC<MenuItemProps> = ({ label, className, onClick }) => (
  <button onClick={onClick} className={clsx("thing_table_item_dropdown__button", className)}>
    {label}
  </button>
);

export interface MenuLinkProps {
  label: string;
  className?: string;
  to: string;
}

export const MenuLink: React.FC<MenuLinkProps> = ({ label, className, to }) => (
  <Link to={to} className={clsx("thing_table_item_dropdown__button", className)}>
    {label}
  </Link>
);
