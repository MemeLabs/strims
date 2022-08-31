// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./ContextMenu.scss";

import clsx from "clsx";
import React, { ReactNode, useCallback, useEffect, useRef, useState } from "react";
import usePortal from "use-portal";

import { useLayout } from "../contexts/Layout";
import useClickAway, { suppressClickAway } from "../hooks/useClickAway";
import useSize from "../hooks/useSize";

interface MenuProps {
  onClose: () => void;
  x: number;
  y: number;
  children: ReactNode;
}

const MenuPortal: React.FC<MenuProps> = ({ children, onClose, x, y }) => {
  const { root } = useLayout();
  const { Portal } = usePortal({ target: root });

  const ref = useRef<HTMLDivElement>(null);
  const size = useSize(ref);
  useClickAway(ref, onClose);

  return (
    <Portal>
      <div
        ref={ref}
        className={clsx({
          "context_menu": true,
          "context_menu--open": size?.width > 0,
          "context_menu--flip_x": x + size?.width > window.innerWidth,
          "context_menu--flip_y": y + size?.height > window.innerHeight,
        })}
        style={{
          "--context-menu-x": `${x}px`,
          "--context-menu-y": `${y}px`,
        }}
        {...suppressClickAway()}
      >
        {children}
      </div>
    </Portal>
  );
};

interface ContextMenuProps {
  children: ReactNode;
}

export const useContextMenu = () => {
  const [{ isOpen, ...position }, setState] = useState({ isOpen: false, x: 0, y: 0 });

  const openMenu = useCallback(
    (e: React.MouseEvent) => setState({ isOpen: true, x: e.pageX, y: e.pageY }),
    []
  );

  const closeMenu = useCallback(() => setState({ isOpen: false, x: 0, y: 0 }), []);

  const Menu: React.FC<ContextMenuProps> = useCallback(
    ({ children }) =>
      isOpen && (
        <MenuPortal onClose={closeMenu} {...position}>
          {children}
        </MenuPortal>
      ),
    [isOpen]
  );

  return {
    isOpen,
    openMenu,
    closeMenu,
    Menu,
  };
};

type MenuItemProps<T extends keyof JSX.IntrinsicElements> = React.ComponentProps<T> & {
  component?: T;
  disabled?: boolean;
};

export const MenuItem = <T extends keyof JSX.IntrinsicElements>({
  component,
  className,
  children,
  ...props
}: MenuItemProps<T>) =>
  React.createElement(
    component ?? "button",
    {
      className: clsx(className, {
        "context_menu__item": true,
        "context_menu__item--disabled": props.disabled,
      }),
      ...props,
    },
    children
  );
