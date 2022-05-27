// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./ContextMenu.scss";

import React from "react";
import usePortal from "react-useportal";

export interface ContextMenuProps {}

const ContextMenu: React.FC<ContextMenuProps> = ({ children }) => {
  const { isOpen, openPortal, closePortal, Portal } = usePortal();

  return (
    <>
      {isOpen && (
        <Portal>
          <div className="context_menu">{children}</div>
        </Portal>
      )}
    </>
  );
};

export default ContextMenu;
