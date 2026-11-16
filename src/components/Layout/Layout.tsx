// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Layout.scss";

import clsx from "clsx";
import React, { ReactNode, RefCallback, useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useLocation } from "react-router";

import { useLayout, withLayoutContext } from "../../contexts/Layout";
import { globalHotkeyOptions } from "../../lib/hotkeys";
import { DEVICE_TYPE, DeviceType, IS_IOS, IS_PWA } from "../../lib/userAgent";
import NotificationToast from "../Notification/Toast";
import { withTheme } from "../Theme";
import Footer from "./Footer";
import Header from "./Header";
import { BackSwipeGuard } from "./backSwipeGuard";
import { layoutShapeClassNames, layoutStateClassNames, screenOrientation } from "./shape";

export type Breakpoint = "sm" | "md" | "lg" | "xl";

const FORCE_FIXED_SIZE = DEVICE_TYPE !== DeviceType.PC;

interface ViewportShape {
  height: number;
  width: number;
  orientation: number;
  safariViewportBug: boolean;
  useFixedSize: boolean;
}

const getViewportShape = (prev?: ViewportShape): ViewportShape => {
  let height = window.visualViewport?.height ?? window.innerHeight;
  let width = window.visualViewport?.width ?? window.innerWidth;
  const orientation = screenOrientation();

  // in ios if the on screen keyboard was opened and closed with the app in pwa
  // fullscreen mode with landscape orientation the VisualViewport api returns
  // erronious small heights in portrait mode. this bug clears after the next
  // resize event in portrait mode.
  let safariViewportBug = prev?.safariViewportBug ?? false;
  let useFixedSize = true;
  if (IS_PWA && IS_IOS) {
    if (prev?.orientation !== 0 && orientation !== 0 && prev?.height !== height) {
      safariViewportBug = prev?.height > height;
    }
    if (prev?.orientation === 0 && orientation === 0 && prev?.height !== height) {
      safariViewportBug = false;
    }

    if (safariViewportBug && orientation === 0) {
      useFixedSize = false;
      height = window.innerHeight;
      width = window.innerWidth;
    }
  }

  return { height, width, orientation, safariViewportBug, useFixedSize };
};

// iOS's touch events also carry a pinch's scale.
interface ScaledTouchEvent extends TouchEvent {
  scale?: number;
}

// Whether `target` is in a field a tap focuses: an input, a textarea, a
// select or editable text.
const inField = (target: EventTarget): boolean =>
  target instanceof Element &&
  !!target.closest("input, textarea, select, [contenteditable]:not([contenteditable='false'])");

interface LayoutProps {
  className: string;
  rootRef: RefCallback<HTMLElement>;
  children: ReactNode;
}

const Layout: React.FC<LayoutProps> = ({ className, rootRef, children }) => {
  const [viewportShape, setViewportShape] = useState(getViewportShape);

  useEffect(() => {
    if (DEVICE_TYPE !== DeviceType.Portable) {
      return;
    }

    // Swipe history navigation is off near the page's left edge, so a swipe
    // of a panel there (the nav, the swapped drawer's strip) doesn't go
    // back; swipes beginning past the edge of the screen are unaffected.
    // The guard cancels those touches' starts, and gives a tap the click the
    // browser then doesn't.
    const guard = new BackSwipeGuard();

    const handleTouchStart = (event: TouchEvent) => {
      const field = inField(event.target);
      if (guard.start(event.changedTouches[0], event.touches.length, field)) {
        event.preventDefault();
      }
    };

    const handleTouchEnd = (event: TouchEvent) => {
      const tap = guard.end(event.changedTouches);
      if (!tap) {
        return;
      }

      const click = new MouseEvent("click", {
        bubbles: true,
        cancelable: true,
        view: window,
        detail: 1,
        screenX: tap.screenX,
        screenY: tap.screenY,
        clientX: tap.clientX,
        clientY: tap.clientY,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        shiftKey: event.shiftKey,
        metaKey: event.metaKey,
      });
      event.target.dispatchEvent(click);
    };

    const handleTouchCancel = () => guard.cancel();

    const handleTouchMove = (event: ScaledTouchEvent) => {
      guard.move(event.changedTouches);

      // disable pinch zoom
      if ("scale" in event && event.scale !== 1) {
        event.preventDefault();
        event.stopPropagation();
      }
    };

    document.addEventListener("touchstart", handleTouchStart, { capture: true, passive: false });
    document.addEventListener("touchend", handleTouchEnd);
    document.addEventListener("touchcancel", handleTouchCancel);
    document.addEventListener("touchmove", handleTouchMove, { passive: false });
    return () => {
      document.removeEventListener("touchstart", handleTouchStart, { capture: true });
      document.removeEventListener("touchend", handleTouchEnd);
      document.removeEventListener("touchcancel", handleTouchCancel);
      document.removeEventListener("touchmove", handleTouchMove);
    };
  }, []);

  useEffect(() => {
    const handleViewportChange = () => setViewportShape(getViewportShape);

    // disable scroll events
    const handleScroll = (event: Event) => {
      window.scroll(0, 0);
      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener("orientationchange", handleViewportChange);
    window.visualViewport.addEventListener("resize", handleViewportChange);
    window.visualViewport.addEventListener("scroll", handleScroll);
    window.addEventListener("scroll", handleScroll);

    return () => {
      window.removeEventListener("orientationchange", handleViewportChange);
      window.visualViewport.removeEventListener("resize", handleViewportChange);
      window.visualViewport.removeEventListener("scroll", handleScroll);
      window.removeEventListener("scroll", handleScroll);
    };
  }, []);

  const layout = useLayout();

  useHotkeys("alt+y", () => layout.toggleSwapMainPanels(), globalHotkeyOptions);

  useHotkeys("alt+c", () => layout.toggleFullScreenChat(), globalHotkeyOptions);

  const location = useLocation();
  useEffect(() => layout.toggleFullScreenChat(false), [location]);

  const { height, width, orientation } = viewportShape;

  const style = !FORCE_FIXED_SIZE
    ? {
        "--layout-height": "100vh",
        "--layout-width": "100vw",
      }
    : viewportShape.useFixedSize
      ? {
          "--layout-height": `${height}px`,
          "--layout-width": `${width}px`,
        }
      : {
          "--layout-height": "100%",
          "--layout-width": "100%",
        };

  return (
    <div
      ref={rootRef}
      style={style}
      className={clsx(className, {
        "layout": true,
        [`layout--${DEVICE_TYPE}`]: true,
        "layout--pwa": IS_PWA,
        ...layoutShapeClassNames(width, height, orientation),
        ...layoutStateClassNames(layout),
      })}
    >
      <NotificationToast />
      {DEVICE_TYPE !== DeviceType.Portable && (
        <div className="layout__header">
          <Header />
        </div>
      )}
      <div className="layout__body">{children}</div>
      {DEVICE_TYPE === DeviceType.Portable && (
        <div className="layout__footer">
          <Footer />
        </div>
      )}
    </div>
  );
};

export default withTheme(withLayoutContext(Layout));
