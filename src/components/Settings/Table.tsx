// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Table.scss";

import clsx from "clsx";
import React, {
  ChangeEvent,
  ComponentProps,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { BsThreeDots } from "react-icons/bs";
import { MdChevronLeft } from "react-icons/md";
import { Link } from "react-router-dom";

import { setRef } from "../../lib/ref";
import Dropdown from "../Dropdown";

export interface TableState {
  checkboxes: Map<string, Set<bigint>>;
  values: Map<string, Set<bigint>>;
}

interface TableActions {
  registerCheckbox(name: string, id: bigint): void;
  unregisterCheckbox(name: string, id: bigint): void;
  setCheckboxValue(name: string, id: bigint, value: boolean): void;
  setAllCheckboxValues(name: string, value: boolean): void;
}

const defaultTableState: TableState = {
  checkboxes: new Map(),
  values: new Map(),
};

const TableContext = React.createContext<[TableState, TableActions]>(null);

interface TableProps {
  children: ReactNode;
}

export const Table = React.forwardRef<TableState, TableProps>(({ children }, ref) => {
  const [state, setState] = useState(defaultTableState);
  setRef(ref, state);

  const registerCheckbox = useCallback((name: string, id: bigint) => {
    setState((prev) => {
      const checkboxes = new Set(prev.checkboxes.get(name)).add(id);
      return { ...prev, checkboxes: new Map(prev.checkboxes).set(name, checkboxes) };
    });
  }, []);

  const unregisterCheckbox = useCallback((name: string, id: bigint) => {
    setState((prev) => {
      const checkboxes = new Set(prev.checkboxes.get(name));
      const values = new Set(prev.values.get(name));
      checkboxes.delete(id);
      values.delete(id);
      return {
        ...prev,
        checkboxes: new Map(prev.checkboxes).set(name, checkboxes),
        values: new Map(prev.values).set(name, values),
      };
    });
  }, []);

  const setCheckboxValue = useCallback((name: string, id: bigint, value: boolean) => {
    setState((prev) => {
      const values = new Set(prev.values.get(name));
      if (value) {
        values.add(id);
      } else {
        values.delete(id);
      }
      return { ...prev, values: new Map(prev.values).set(name, values) };
    });
  }, []);

  const setAllCheckboxValues = useCallback((name: string, value: boolean) => {
    setState((prev) => {
      const values = value ? new Set(prev.checkboxes.get(name)) : new Set<bigint>();
      return { ...prev, values: new Map(prev.values).set(name, values) };
    });
  }, []);

  const value = useMemo<[TableState, TableActions]>(
    () => [state, { registerCheckbox, unregisterCheckbox, setCheckboxValue, setAllCheckboxValues }],
    [state]
  );

  return (
    <table className="thing_table">
      <TableContext.Provider value={value}>{children}</TableContext.Provider>
    </table>
  );
});

interface MenuCellProps {
  children: ReactNode;
}

export const MenuCell: React.FC<MenuCellProps> = ({ children }) => (
  <TableCell className="thing_table__row_menu">
    <Dropdown baseClassName="thing_table_item_dropdown" anchor={<BsThreeDots />} items={children} />
  </TableCell>
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

export interface TableCellProps extends ComponentProps<"td"> {
  truncate?: boolean;
  children?: ReactNode;
}

export const TableCell: React.FC<TableCellProps> = ({
  truncate,
  children,
  className,
  ...props
}) => {
  if (truncate) {
    return (
      <td className={clsx("thing_table__cell", className)} {...props}>
        <div className="thing_table__truncate">
          <span className="thing_table__truncate__reference">{children}</span>
          <span className="thing_table__truncate__display">{children}</span>
        </div>
      </td>
    );
  }

  return (
    <td className={clsx("thing_table__cell", className)} {...props}>
      {children}
    </td>
  );
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

export interface CheckboxCellProps {
  name: string;
  id: bigint;
}

export const CheckboxCell: React.FC<CheckboxCellProps> = ({ name, id }) => {
  const [state, { registerCheckbox, unregisterCheckbox, setCheckboxValue }] =
    useContext(TableContext);

  const handleChange = useCallback((e: ChangeEvent<HTMLInputElement>) => {
    setCheckboxValue(name, id, e.currentTarget.checked);
  }, []);

  useEffect(() => {
    registerCheckbox(name, id);
    return () => unregisterCheckbox(name, id);
  }, [name, id]);

  return (
    <TableCell className="thing_table_checkbox_cell">
      <input
        type="checkbox"
        onChange={handleChange}
        checked={state.values.get(name)?.has(id) ?? false}
        name={name}
        value={id.toString()}
      />
    </TableCell>
  );
};

export interface CheckboxHeaderProps {
  name: string;
}

export const CheckboxHeader: React.FC<CheckboxHeaderProps> = ({ name }) => {
  const [state, { setAllCheckboxValues }] = useContext(TableContext);

  const handleChange = useCallback((e: ChangeEvent<HTMLInputElement>) => {
    setAllCheckboxValues(name, e.currentTarget.checked);
  }, []);

  const checked = useMemo(() => {
    const values = new Set(state.values.get(name));
    const checkboxes = new Set(state.checkboxes.get(name));
    return ![...checkboxes].some((id) => !values.has(id));
  }, [state]);

  return (
    <th className="thing_table_checkbox_cell">
      <input type="checkbox" onChange={handleChange} checked={checked} name={name} />
    </th>
  );
};
