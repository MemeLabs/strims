import "./SwipablePanel.scss";

import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import { isEqual } from "lodash";
import React, { useCallback, useContext, useLayoutEffect, useRef, useState } from "react";
import { useToggle } from "react-use";

import useUpdate from "../hooks/useUpdate";
import { DEVICE_TYPE, DeviceType } from "../lib/userAgent";

const HorizontalContext = React.createContext<(open: boolean) => void>(null);
const VerticalContext = React.createContext<(open: boolean) => void>(null);

const TOGGLE_UPDATE_DELAY = 50;
const TOGGLE_TRANSITION_DURATION = 200; // todo sync with scss

export interface DragState {
  closed: boolean;
  closing: boolean;
  dragging: boolean;
  transitioning?: boolean;
}

const getDragState = (open: boolean) => ({
  closed: !open,
  closing: open,
  dragging: false,
});

interface SwipablePaneProps {
  direction: "up" | "down" | "left" | "right";
  className?: string;
  dragThreshold?: number;
  open?: boolean;
  animateInitialState?: boolean;
  handleRef?: EventTarget | React.RefObject<EventTarget>;
  onToggle?: (open: boolean) => void;
  onDragStateChange?: (state: DragState) => void;
  preventScroll?: boolean;
  filterDeviceTypes?: DeviceType[] | null;
}

const SwipablePanel: React.FC<SwipablePaneProps> = ({
  direction,
  children,
  className,
  dragThreshold = 200,
  open = true,
  animateInitialState = false,
  handleRef,
  onToggle,
  onDragStateChange,
  preventScroll = false,
  filterDeviceTypes = [DeviceType.Portable],
}) => {
  if (filterDeviceTypes && !filterDeviceTypes.includes(DEVICE_TYPE)) {
    return <div className={className}>{children}</div>;
  }

  const ref = useRef<HTMLDivElement>(null);
  const vertical = direction === "up" || direction === "down";

  // gestures on nested swipable panels that move on the same axis collide. the
  // prescribed solution for this in use-gesture doesn't work if preventScroll
  // is enabled. without preventScroll gestures on scrollable elements interfere
  // with panel gestures. to solve this panels disable their anscestores when
  // opened.

  // note that because the gestures for open and close collide if nested panels
  // open in opposite directions this isn't supported.

  const Context = vertical ? VerticalContext : HorizontalContext;
  const toggleParentChildOpen = useContext(Context);
  const [childOpen, toggleChildOpen] = useToggle(false);

  const contextValue = useCallback((open: boolean) => {
    toggleParentChildOpen?.(open);
    toggleChildOpen(open);
  }, []);

  const [dragState, setDragState] = useState(() => getDragState(open && !animateInitialState));
  const toggleDragState = (open: boolean) => {
    setDragState(getDragState(open));
    toggleParentChildOpen?.(open);
  };

  useLayoutEffect(() => {
    if (dragState.closed == open) {
      const tid = setTimeout(() => toggleDragState(open), TOGGLE_UPDATE_DELAY);
      return () => clearTimeout(tid);
    }
  }, [open]);

  const [emittedOpen, setEmittedOpen] = useState(open);
  useUpdate(() => onToggle?.(emittedOpen), [emittedOpen]);

  useUpdate(() => {
    if (!dragState.dragging) {
      const tid = setTimeout(() => {
        setEmittedOpen(!dragState.closed);
        onDragStateChange?.({ ...dragState, transitioning: false });
      }, TOGGLE_TRANSITION_DURATION);
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
          m = -mx;
          s = -sx;
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
          (dragState.closing && (s === 1 || m > dragThreshold)) ||
          (!dragState.closing && s !== -1 && m > -dragThreshold);
        setDragOffset(0);
        next = { closed, closing: !closed, dragging: false };
      }
      if (!isEqual(dragState, next)) {
        setDragState(next);
        toggleParentChildOpen?.(!next.closed);
        onDragStateChange?.({ ...next, transitioning: true });
      }
    },
    {
      enabled: !childOpen,
      axis: vertical ? "y" : "x",
      target: handleRef ?? ref,
      eventOptions: {
        passive: false,
      },
      boundToParent: false,
      triggerAllEvents: true,
      preventScroll,
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
      <Context.Provider value={contextValue}>{children}</Context.Provider>
    </div>
  );
};

export default SwipablePanel;
