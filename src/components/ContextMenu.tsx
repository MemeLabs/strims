// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./ContextMenu.scss";

import clsx from "clsx";
import React, { ReactNode, useCallback, useMemo, useRef, useState } from "react";
import usePortal from "use-portal";

import { useLayout } from "../contexts/Layout";
import useClickAway from "../hooks/useClickAway";
import { useStableCallback } from "../hooks/useStableCallback";

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
  useClickAway(ref, onClose);

  // because menus are rendered at the react root poitner events targeted here
  // are not filtered out by useClickAway. this prevents interacting with menus
  // in modals that unmount when they lose focus. to prevent this we stop events
  // before they bubble up to the document.
  const stopEventPropagation = useStableCallback((e: React.MouseEvent | React.TouchEvent) => {
    e.stopPropagation();
  });

  return (
    <Portal>
      <div
        ref={ref}
        className="context_menu"
        style={{
          "--context-menu-x": `${x}px`,
          "--context-menu-y": `${y}px`,
        }}
        onMouseDown={stopEventPropagation}
        onClick={stopEventPropagation}
        onTouchStart={stopEventPropagation}
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

  const Menu: React.FC<ContextMenuProps> = useMemo(() => {
    return isOpen
      ? ({ children }) => (
          <MenuPortal onClose={closeMenu} {...position}>
            {children}
          </MenuPortal>
        )
      : () => null;
  }, [isOpen]);

  return {
    isOpen,
    openMenu,
    closeMenu,
    Menu,
  };
};

export const MenuItem: React.FC<React.ComponentProps<"button">> = ({
  children,
  className,
  ...props
}) => (
  <button
    className={clsx(className, {
      "context_menu__item": true,
      "context_menu__item--disabled": props.disabled,
    })}
    {...props}
  >
    {children}
  </button>
);
