import "./SwipablePanel.scss";

import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import { isEqual } from "lodash";
import React, { useLayoutEffect, useRef, useState } from "react";

import useUpdates from "../hooks/useUpdates";
import { DEVICE_TYPE, DeviceType } from "../lib/userAgent";

const DRAG_THRESHOLD = 200;

export interface DragState {
  closed: boolean;
  closing: boolean;
  dragging: boolean;
}

const initialDragState: DragState = {
  closed: true,
  closing: false,
  dragging: false,
};

interface SwipablePaneProps {
  direction: "up" | "down" | "left" | "right";
  className?: string;
  open?: boolean;
  animateInitialState?: boolean;
  handleRef?: EventTarget | React.RefObject<EventTarget>;
  onToggle?: (open: boolean) => void;
  onDragStateChange?: (state: DragState) => void;
}

const SwipablePanel: React.FC<SwipablePaneProps> = ({
  direction,
  children,
  className,
  open = true,
  animateInitialState = false,
  onToggle,
  onDragStateChange,
}) => {
  if (DEVICE_TYPE !== DeviceType.Portable) {
    return <div className={className}>{children}</div>;
  }

  const ref = useRef<HTMLDivElement>(null);
  const [dragState, setDragState] = useState(initialDragState);

  const toggleDragState = (open: boolean) =>
    setDragState({
      closed: !open,
      closing: open,
      dragging: false,
    });

  useLayoutEffect(() => {
    if (!animateInitialState) {
      toggleDragState(open);
    }
  }, []);

  useLayoutEffect(() => {
    const rafId = window.requestAnimationFrame(() => toggleDragState(open));
    return () => window.cancelAnimationFrame(rafId);
  }, [open]);

  useUpdates(() => {
    if (!dragState.dragging) {
      const tid = setTimeout(() => onToggle?.(!dragState.closed), 200);
      return () => clearTimeout(tid);
    }
  }, [dragState]);

  const setDragOffset = (v: number) =>
    ref.current?.style.setProperty("--swipable-drag-offset", `${v}px`);

  useLayoutEffect(() => setDragOffset(0), []);

  useDrag(
    ({ movement: [mx, my], swipe: [sx, sy], dragging }) => {
      let m = 0;
      let s = 0;
      switch (direction) {
        case "up":
          m = my;
          s = sy;
          break;
        case "down":
          m = -my;
          s = -sy;
          break;
        case "left":
          m = mx;
          s = sx;
          break;
        case "right":
          m = mx;
          s = sx;
      }

      let next: DragState;
      if (dragging) {
        if (dragState.closing) {
          setDragOffset(Math.max(m, 0));
          next = { closed: false, closing: true, dragging: true };
        } else {
          setDragOffset(Math.max(-m, 0));
          next = { closed: m >= -10, closing: false, dragging: true };
        }
      } else {
        const closed =
          (dragState.closing && (s === 1 || m > DRAG_THRESHOLD)) ||
          (!dragState.closing && s !== -1 && m > -DRAG_THRESHOLD);
        setDragOffset(0);
        next = { closed, closing: !closed, dragging: false };
      }
      if (!isEqual(dragState, next)) {
        setDragState(next);
        onDragStateChange?.(next);
      }
    },
    {
      target: ref,
      eventOptions: {
        capture: true,
        passive: false,
      },
    }
  );

  return (
    <div
      ref={ref}
      className={clsx(className, {
        "swipable": true,
        [`swipable--${direction}`]: true,
        "swipable--open": !dragState.closed,
        "swipable--dragging": dragState.dragging,
        "swipable--closing": dragState.closing,
      })}
    >
      {children}
    </div>
  );
};

export default SwipablePanel;
