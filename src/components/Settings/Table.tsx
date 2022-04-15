import "./Table.scss";

import clsx from "clsx";
import React from "react";
import { BsChevronLeft, BsThreeDots } from "react-icons/bs";
import { Link } from "react-router-dom";

import Dropdown from "../Dropdown";

export const Table: React.FC = ({ children }) => <table className="thing_table">{children}</table>;

export const MenuCell: React.FC = ({ children }) => (
  <td className="thing_table__row_menu">
    <Dropdown baseClassName="thing_table_item_dropdown" anchor={<BsThreeDots />} items={children} />
  </td>
);

export interface TableTitleBarProps {
  label: string;
  backLink?: string;
}

export const TableTitleBar: React.FC<TableTitleBarProps> = ({ label, backLink, children }) => (
  <div className="thing_table__title_bar">
    <div className="thing_table__title_bar__header">
      {backLink && (
        <Link to={backLink} className="thing_table__title_bar__back">
          <BsChevronLeft />
        </Link>
      )}
      <h2 className="thing_table__title_bar__title">{label}</h2>
    </div>
    <div className="thing_table__title_bar__controls">{children}</div>
  </div>
);

export interface TableCellProps {
  truncate?: boolean;
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
