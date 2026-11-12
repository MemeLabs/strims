// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "overlayscrollbars/overlayscrollbars.css";
import "./Scrollbars.scss";

import type { PartialOptions } from "overlayscrollbars";
import { useOverlayScrollbars } from "overlayscrollbars-react";
import React, {
  CSSProperties,
  ComponentProps,
  ReactElement,
  cloneElement,
  forwardRef,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
} from "react";

import { useStableCallback } from "../hooks/useStableCallback";

export interface ScrollbarsHandle {
  /** The element the view fills and the scrollbars are drawn over. */
  readonly container: HTMLDivElement | null;
  /** The scrolling element. */
  readonly view: HTMLElement | null;
  scrollToTop(): void;
  getScrollTop(): number;
}

export interface ScrollbarsProps extends Omit<ComponentProps<"div">, "onScroll" | "ref"> {
  /** Shows the scrollbars only while the view scrolls (and the pointer is on them). */
  autoHide?: boolean;
  /** Lets the view scroll sideways too. */
  scrollX?: boolean;
  /** The scrolling element, from its style; a div by default. */
  renderView?: (props: { style: CSSProperties }) => ReactElement;
  onScroll?: (e: Event) => void;
  /** A scroll starts, and stops once the view hasn't moved for SCROLL_STOP_MS. */
  onScrollStart?: () => void;
  onScrollStop?: () => void;
}

const SCROLL_STOP_MS = 100;

const CONTAINER_STYLE: CSSProperties = {
  position: "relative",
  overflow: "hidden",
  width: "100%",
  height: "100%",
};

// The view fills the container and scrolls: OverlayScrollbars, once it takes
// the view over, sets the overflow its properties carry.
const VIEW_STYLE: CSSProperties = {
  position: "absolute",
  top: 0,
  left: 0,
  right: 0,
  bottom: 0,
  overflowX: "var(--os-viewport-overflow-x, scroll)" as CSSProperties["overflowX"],
  overflowY: "var(--os-viewport-overflow-y, scroll)" as CSSProperties["overflowY"],
};

const defaultRenderView = (props: { style: CSSProperties }) => <div {...props} />;

// A view that scrolls inside its container with thin overlay scrollbars
// (OverlayScrollbars) in place of the native ones.
const Scrollbars = forwardRef<ScrollbarsHandle, ScrollbarsProps>(
  (
    {
      autoHide = false,
      scrollX = true,
      renderView = defaultRenderView,
      onScroll,
      onScrollStart,
      onScrollStop,
      style,
      children,
      ...props
    },
    ref
  ) => {
    const container = useRef<HTMLDivElement>(null);
    const view = useRef<HTMLElement>(null);

    const options = useMemo<PartialOptions>(
      () => ({
        overflow: { x: scrollX ? "scroll" : "hidden" },
        scrollbars: {
          theme: "os-theme-strims",
          autoHide: autoHide ? "scroll" : "never",
          autoHideDelay: 1000,
          clickScroll: "instant",
        },
      }),
      [autoHide, scrollX]
    );
    // Where the native scrollbars overlay the content and take no room
    // (phones, macOS) the view keeps them.
    const [initialize] = useOverlayScrollbars({ options });
    useLayoutEffect(() => {
      initialize({
        target: container.current,
        elements: { viewport: view.current, padding: false, content: false },
        cancel: { nativeScrollbarsOverlaid: true },
      });
    }, []);

    const handleScroll = useStableCallback((e: Event) => onScroll?.(e));
    const handleScrollStart = useStableCallback(() => onScrollStart?.());
    const handleScrollStop = useStableCallback(() => onScrollStop?.());
    useEffect(() => {
      const el = view.current;
      let stop = 0;
      const listener = (e: Event) => {
        handleScroll(e);
        if (stop) {
          clearTimeout(stop);
        } else {
          handleScrollStart();
        }
        stop = window.setTimeout(() => {
          stop = 0;
          handleScrollStop();
        }, SCROLL_STOP_MS);
      };
      el.addEventListener("scroll", listener);
      return () => {
        el.removeEventListener("scroll", listener);
        clearTimeout(stop);
      };
    }, []);

    useImperativeHandle(
      ref,
      () => ({
        get container() {
          return container.current;
        },
        get view() {
          return view.current;
        },
        scrollToTop: () => {
          if (view.current) {
            view.current.scrollTop = 0;
          }
        },
        getScrollTop: () => view.current?.scrollTop ?? 0,
      }),
      []
    );

    return (
      <div {...props} ref={container} style={{ ...CONTAINER_STYLE, ...style }}>
        {cloneElement(renderView({ style: VIEW_STYLE }), { ref: view }, children)}
      </div>
    );
  }
);

Scrollbars.displayName = "Scrollbars";

export default Scrollbars;
