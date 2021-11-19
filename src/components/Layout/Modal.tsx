import "./Modal.scss";

import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import { isEqual } from "lodash";
import React, { useLayoutEffect, useRef, useState } from "react";
import { BsArrowBarDown } from "react-icons/bs";

import { ContentState } from "../../contexts/Layout";
import useUpdates from "../../hooks/useUpdates";

const DRAG_THRESHOLD = 200;

const initialShowContent = {
  closed: true,
  closing: false,
  dragging: false,
};

interface ModalProps {
  title?: string;
  className?: string;
  defaultOpen?: boolean;
  onClose?: () => void;
}

const Modal: React.FC<ModalProps> = ({
  title,
  className,
  defaultOpen = true,
  children,
  onClose,
}) => {
  const modal = useRef<HTMLDivElement>(null);
  const header = useRef<HTMLDivElement>(null);

  const [showContent, setShowContent] = useState(initialShowContent);

  const toggleShowContent = (open: boolean) =>
    setShowContent({
      closed: !open,
      closing: open,
      dragging: false,
    });

  useLayoutEffect(() => {
    if (defaultOpen) {
      const rafId = window.requestAnimationFrame(() => toggleShowContent(defaultOpen));
      return () => window.cancelAnimationFrame(rafId);
    }
  }, []);

  useUpdates(() => {
    if (showContent.closed) {
      const tid = setTimeout(() => onClose?.(), 200);
      return () => clearTimeout(tid);
    }
  }, [showContent.closed]);

  const setDragOffset = (v: number) =>
    modal.current?.style.setProperty("--layout-drag-offset", `${v}px`);

  useDrag(
    ({ movement: [, my], swipe: [, sy], dragging }) => {
      let next: ContentState;
      if (dragging) {
        if (showContent.closing) {
          setDragOffset(Math.max(my, 0));
          next = { closed: false, closing: true, dragging: true };
        } else {
          setDragOffset(Math.max(-my, 0));
          next = { closed: my >= -10, closing: false, dragging: true };
        }
      } else {
        const closed =
          (showContent.closing && (sy === 1 || my > DRAG_THRESHOLD)) ||
          (!showContent.closing && sy !== -1 && my > -DRAG_THRESHOLD);
        setDragOffset(0);
        next = { closed, closing: !closed, dragging: false };
      }
      if (!isEqual(showContent, next)) {
        setShowContent(next);
      }
    },
    {
      target: header,
      eventOptions: {
        capture: true,
        passive: false,
      },
    }
  );

  return (
    <div
      ref={modal}
      className={clsx(className, {
        "modal": true,
        "modal--open": !showContent.closed,
        "modal--dragging": showContent.dragging,
        "modal--closing": showContent.closing,
      })}
    >
      <div ref={header} className="modal__header">
        {title && <div className="modal__header__title">{title}</div>}
        <button className="modal__header__close" onClick={() => toggleShowContent(false)}>
          <BsArrowBarDown className="modal__header__close__icon" />
        </button>
      </div>
      <div className="modal__body">{children}</div>
    </div>
  );
};

export default Modal;
