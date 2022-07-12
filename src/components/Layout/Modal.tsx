// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Modal.scss";

import clsx from "clsx";
import React, { ReactNode, useRef } from "react";
import { BsArrowBarDown } from "react-icons/bs";
import { useToggle } from "react-use";
import usePortal from "use-portal";

import { useLayout } from "../../contexts/Layout";
import useUpdate from "../../hooks/useUpdate";
import SwipablePanel, { DragState } from "../SwipablePanel";

interface ModalProps {
  title?: ReactNode;
  showHeader?: boolean;
  className?: string;
  open?: boolean;
  onClose?: () => void;
  children: ReactNode;
}

const Modal: React.FC<ModalProps> = ({
  title = "",
  showHeader = title !== "",
  className,
  open = true,
  children,
  onClose,
}) => {
  const header = useRef<HTMLDivElement>(null);

  const [modalOpen, toggleModalOpen] = useToggle(open);
  useUpdate(() => toggleModalOpen(open), [open]);

  const layout = useLayout();
  const { Portal } = usePortal({ target: layout.root });

  const handleToggle = (open: boolean) => {
    if (!open) {
      onClose?.();
    }
  };

  const handleDragStateChange = (state: DragState) =>
    layout.toggleModalOpen(!state.closed && !state.transitioning);

  return (
    <Portal>
      <SwipablePanel
        className={clsx("modal", className)}
        direction="up"
        open={modalOpen}
        onToggle={handleToggle}
        onDragStateChange={handleDragStateChange}
        animateInitialState={true}
        handleRef={showHeader ? header : null}
      >
        {showHeader && (
          <div ref={header} className="modal__header">
            {title && <div className="modal__header__title">{title}</div>}
            <button className="modal__header__close" onClick={() => toggleModalOpen(false)}>
              <BsArrowBarDown className="modal__header__close__icon" />
            </button>
          </div>
        )}
        <div className="modal__body">{children}</div>
      </SwipablePanel>
    </Portal>
  );
};

export default Modal;
