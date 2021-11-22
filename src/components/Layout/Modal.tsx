import "./Modal.scss";

import clsx from "clsx";
import React, { ReactNode, useRef } from "react";
import { BsArrowBarDown } from "react-icons/bs";
import { useToggle } from "react-use";

import useUpdates from "../../hooks/useUpdates";
import SwipablePanel from "../SwipablePanel";

interface ModalProps {
  title?: ReactNode;
  showHeader?: boolean;
  className?: string;
  open?: boolean;
  onClose?: () => void;
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
  useUpdates(() => toggleModalOpen(open), [open]);

  const handleToggle = (open: boolean) => {
    if (!open) {
      onClose?.();
    }
  };

  return (
    <SwipablePanel
      className={clsx("modal", className)}
      direction="up"
      open={modalOpen}
      onToggle={handleToggle}
      animateInitialState={true}
      handleRef={showHeader && header}
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
  );
};

export default Modal;
