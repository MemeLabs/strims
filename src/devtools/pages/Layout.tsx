import "../styles/layout.scss";

import clsx from "clsx";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { NavLink } from "react-router-dom";
import { useDrag } from "react-use-gesture";
import UAParser from "ua-parser-js";

import isPWA from "../../lib/isPWA";

export const enum DeviceType {
  TV = "tv",
  Portable = "portable",
  PC = "pc",
}

const DEVICE_TYPE = (() => {
  switch (new UAParser().getDevice().type) {
    case "console":
    case "smarttv":
      return DeviceType.TV;
    case "mobile":
    case "tablet":
    case "wearable":
    case "embedded":
      return DeviceType.Portable;
    default:
      return DeviceType.PC;
  }
})();
const FORCE_FIXED_SIZE = DEVICE_TYPE !== DeviceType.PC;

const getViewportSize = () => ({
  height: window.visualViewport?.height || window.innerHeight,
  width: window.visualViewport?.width || window.innerWidth,
});

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

const LayoutPage: React.FC = () => {
  const [orientation, setOrientation] = useState(window.orientation || 0);
  const [{ height, width }, setSize] = useState(getViewportSize);
  const aspectRatio = width / height;

  useEffect(() => {
    if (DEVICE_TYPE !== DeviceType.Portable || isPWA) {
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
    // update variables used in our psuedo media query class names.
    const handleOrientationChange = () => {
      setOrientation(window.orientation || 0);
      setSize(getViewportSize);
    };
    const handleResize = () => setSize(getViewportSize);

    // disable scroll events
    const handleScroll = (event: Event) => {
      window.scrollTo(0, 0);
      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener("orientationchange", handleOrientationChange);
    window.visualViewport.addEventListener("resize", handleResize);
    window.visualViewport.addEventListener("scroll", handleScroll);

    return () => {
      window.removeEventListener("orientationchange", handleOrientationChange);
      window.visualViewport.removeEventListener("resize", handleResize);
      window.visualViewport.removeEventListener("scroll", handleScroll);
    };
  }, []);

  const [[closed, closing, dx, dragging], setClosed] = useState([true, false, 0, false]);
  const toggleClosed = () => setClosed(([closed]) => [!closed, closed, 0, false]);
  const threshold = 200;

  const dragHandlers =
    DEVICE_TYPE === "portable"
      ? useDrag(({ movement: [mx], swipe: [sx], dragging }) => {
          if (dragging) {
            if (closing) {
              setClosed([false, true, Math.max(mx, 0), true]);
            } else {
              setClosed([mx >= -10, false, Math.max(-mx, 0), true]);
            }
          } else {
            const closed =
              (closing && (sx === 1 || mx > threshold)) ||
              (!closing && sx !== -1 && mx > -threshold);
            setClosed([closed, !closed, 0, false]);
          }
        })()
      : {};

  const [swap, setSwap] = useState(false);
  const [showChat, setShowChat] = useState(true);
  const [showVideo, setShowVideo] = useState(true);
  const [theaterMode, setTheaterMode] = useState(false);

  const style = FORCE_FIXED_SIZE
    ? {
        "height": `${height}px`,
        "width": `${width}px`,
      }
    : {};

  return (
    <div
      style={style}
      className={clsx({
        "root": true,
        [`root--${DEVICE_TYPE}`]: true,
        "root--pwa": isPWA,
        "root--portrait": orientation === 0,
        "root--landscape_ccw": orientation === 90,
        "root--landscape_cw": orientation === -90,
        "root--min_aspect_ratio_1_2": aspectRatio >= 1.2,
        "root--min_aspect_ratio_2": aspectRatio >= 2,
        "root--min_aspect_ratio_4": aspectRatio >= 3,
        "root--min_width_576": width >= 576,
        "root--min_width_768": width >= 768,
        "root--min_width_992": width >= 992,
        "root--min_width_1200": width >= 1200,
        "root--meme_open": !closed,
        "root--swap": swap,
        "root--hide_chat": !showChat,
        "root--hide_video": !showVideo,
        "root--theater_mode": theaterMode,
      })}
    >
      <div className="deadzone"></div>
      <div className="header"></div>
      <div className="body">
        <div className="nav_panel">
          <Scrollbars>
            <div className="scroll_content_test">
              {new Array(40).fill(0).map((_, i) => (
                <div key={i}>{i}</div>
              ))}
            </div>
          </Scrollbars>
        </div>
        <div className="foo_1">
          <div className="content_panel">
            <Scrollbars>
              <div className="scroll_content_test">
                {new Array(40).fill(0).map((_, i) => (
                  <div key={i}>{i}</div>
                ))}
              </div>
            </Scrollbars>
          </div>
          <div
            className={clsx({
              "foo_2": true,
              "foo_2--dragging": dragging,
              "foo_2--closing": closing,
            })}
            style={{ "--offset": `${dx}px` }}
            {...dragHandlers}
          >
            {showVideo && (
              <div className="video_panel">
                <button onClick={toggleClosed}>toggle</button>
                <button onClick={() => setShowChat((prev) => !prev)}>toggle chat</button>
                <button onClick={() => setShowVideo((prev) => !prev)}>toggle video</button>
                <button onClick={() => setTheaterMode((prev) => !prev)}>toggle theater mode</button>
              </div>
            )}
            <div className="chat_panel">
              <button onClick={() => setSwap((prev) => !prev)}>swap sides</button>
              <button onClick={toggleClosed}>toggle</button>
              <button onClick={() => setShowChat((prev) => !prev)}>toggle chat</button>
              <button onClick={() => setShowVideo((prev) => !prev)}>toggle video</button>
              <button onClick={() => setTheaterMode((prev) => !prev)}>toggle theater mode</button>
              <Scrollbars
                renderView={(props) => <div {...props} className={"chat_panel__scroller"} />}
              >
                {new Array(40).fill(0).map((_, i) => (
                  <div key={i}>{i}</div>
                ))}
              </Scrollbars>
              {/* <input className="test_input" type="text" name="foo" defaultValue="test" /> */}
              {/* <div>
                <NavLink to="/layout/test1" activeClassName="test_active_link">
                  test1
                </NavLink>
                <NavLink to="/layout/test2" activeClassName="test_active_link">
                  test2
                </NavLink>
                <NavLink to="/layout/test3" activeClassName="test_active_link">
                  test3
                </NavLink>
              </div> */}
            </div>
          </div>
        </div>
      </div>
      <div className="footer"></div>
    </div>
  );
};

export default LayoutPage;
