// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Layout.scss";

import clsx from "clsx";
import React, { ReactNode, RefCallback, useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useLocation } from "react-router";

import { useLayout, withLayoutContext } from "../../contexts/Layout";
import { DEVICE_TYPE, DeviceType, IS_IOS, IS_PWA } from "../../lib/userAgent";
import NotificationToast from "../Notification/Toast";
import { withTheme } from "../Theme";
import Footer from "./Footer";
import Header from "./Header";
import { layoutShapeClassNames, layoutStateClassNames } from "./shape";

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
  const orientation = window.orientation ?? 0;

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

interface ExtendedTouchEvent extends TouchEvent {
  scale?: number;
  pageX: number;
  pageY: number;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  button: number;
  relatedTarget: EventTarget;
}

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

    let emulateClick = false;

    // disable swipe history navigation near the edge of the page to prevent
    // accidentally triggering when interacting with panels. swipes beginning
    // past the edge of the screen are unaffected.
    const handleTouchStart = (event: ExtendedTouchEvent) => {
      if (event.pageX < 34) {
        event.preventDefault();
        emulateClick = true;
      }
    };

    // when touch start events are cancelled we have to reproduce the browser's
    // native click emulation on touch end.
    const handleTouchEnd = (event: ExtendedTouchEvent) => {
      if (!emulateClick) {
        return;
      }

      emulateClick = false;
      const click = new MouseEvent("click", {
        bubbles: true,
        cancelable: true,
        view: window,
        detail: 1,
        screenX: event.pageX,
        screenY: event.pageY,
        clientX: event.pageX,
        clientY: event.pageY,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        shiftKey: event.shiftKey,
        metaKey: event.metaKey,
        button: event.button,
        relatedTarget: event.relatedTarget,
      });
      event.target.dispatchEvent(click);
    };

    // disable pinch zoom
    const handleTouchMove = (event: ExtendedTouchEvent) => {
      if ("scale" in event && event.scale !== 1) {
        event.preventDefault();
        event.stopPropagation();
      }
    };

    document.addEventListener("touchstart", handleTouchStart, { capture: true, passive: false });
    document.addEventListener("touchend", handleTouchEnd);
    document.addEventListener("touchmove", handleTouchMove, { passive: false });
    return () => {
      document.removeEventListener("touchstart", handleTouchStart);
      document.removeEventListener("touchend", handleTouchEnd);
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

  useHotkeys("alt+y", () => layout.toggleSwapMainPanels(), {
    enableOnContentEditable: true,
    enableOnFormTags: ["INPUT"],
  });

  useHotkeys("alt+c", () => layout.toggleFullScreenChat(), {
    enableOnContentEditable: true,
    enableOnFormTags: ["INPUT"],
  });

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
