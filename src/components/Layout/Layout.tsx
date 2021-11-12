import "./Layout.scss";

import clsx from "clsx";
import React, { RefCallback, useEffect, useState } from "react";

import { useLayout, withLayoutContext } from "../../contexts/Layout";
import { DEVICE_TYPE, DeviceType, IS_PWA, OS } from "../../lib/userAgent";
import NotificationToast from "../Notification/Toast";
import { withTheme } from "../Theme";
import Footer from "./Footer";
import Header from "./Header";

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

  // in ios if the on screen keyboard was opened while the app was in pwa
  // fullscreen mode with landscape orientation the VisualViewport api returns
  // erronious small heights in portrait mode. this bug clears after the next
  // resize event in portrait mode.
  let safariViewportBug = prev?.safariViewportBug ?? false;
  let useFixedSize = true;
  if (IS_PWA && OS === "iOS") {
    if (prev?.orientation !== 0 && orientation !== 0 && prev?.height !== height) {
      safariViewportBug = true;
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
}

const Layout: React.FC<LayoutProps> = ({ className, rootRef, children }) => {
  const [viewportShape, setViewportShape] = useState(getViewportShape);

  useEffect(() => {
    if (DEVICE_TYPE !== DeviceType.Portable || IS_PWA) {
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
      window.scrollTo(0, 0);
      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener("orientationchange", handleViewportChange);
    window.visualViewport.addEventListener("resize", handleViewportChange);
    window.visualViewport.addEventListener("scroll", handleScroll);

    return () => {
      window.removeEventListener("orientationchange", handleViewportChange);
      window.visualViewport.removeEventListener("resize", handleViewportChange);
      window.visualViewport.removeEventListener("scroll", handleScroll);
    };
  }, []);

  const layout = useLayout();

  const { height, width, orientation } = viewportShape;
  const aspectRatio = width / height;

  const style =
    FORCE_FIXED_SIZE && viewportShape.useFixedSize
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
        "layout--portrait": orientation === 0,
        "layout--landscape_ccw": orientation === 90,
        "layout--landscape_cw": orientation === -90,
        "layout--min_aspect_ratio_0_6": aspectRatio >= 0.6,
        "layout--min_aspect_ratio_1_2": aspectRatio >= 1.2,
        "layout--min_aspect_ratio_2": aspectRatio >= 2,
        "layout--min_aspect_ratio_4": aspectRatio >= 3,
        "layout--min_width_sm": width >= 576,
        "layout--min_width_md": width >= 768,
        "layout--min_width_lg": width >= 992,
        "layout--min_width_xl": width >= 1200,
        "layout--meme_open": !layout.showContent.closed,
        "layout--swap": layout.swapMainPanels,
        "layout--show_chat": layout.showChat,
        "layout--show_video": layout.showVideo,
        "layout--theater_mode": layout.theaterMode,
        "layout--expand_nav": layout.expandNav,
      })}
    >
      <NotificationToast />
      <div className="deadzone"></div>
      <div className="layout__header">
        <Header />
      </div>
      <div className="layout__body">{children}</div>
      <div className="layout__footer">
        <Footer />
      </div>
    </div>
  );
};

export default withTheme(withLayoutContext(Layout));
