import "../styles/layout.scss";
import "../../components/Layout/Layout.scss";

import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import clsx from "clsx";
import React, { RefCallback, useContext, useEffect, useRef, useState } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { useDrag } from "react-use-gesture";
import UAParser from "ua-parser-js";

import { FrontendClient } from "../../apis/client";
import { registerChatFrontendService } from "../../apis/strims/chat/v1/chat_rpc";
import { registerNetworkServiceService } from "../../apis/strims/network/v1/network_rpc";
import Header from "../../components/Layout/Header";
import NetworkNav from "../../components/Layout/NetworkNav";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import { Provider as NetworkProvider } from "../../contexts/Network";
import { Provider as ThemeProvider } from "../../contexts/Theme";
import isPWA from "../../lib/isPWA";
import TestChat from "../components/TestChat";
import { LayoutContext, LayoutContextProvider } from "../contexts/Layout";
import ChatService from "../mocks/chat/service";
import NetworkService from "../mocks/network/service";

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

interface LayoutPageProps {
  rootRef: RefCallback<HTMLElement>;
}

const LayoutPage: React.FC<LayoutPageProps> = ({ rootRef }) => {
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

  const {
    swapMainPanels,
    showContent,
    showChat,
    showVideo,
    theaterMode,
    expandNav,
    toggleSwapMainPanels,
    toggleShowContent,
    toggleShowChat,
    toggleShowVideo,
    toggleTheaterMode,
  } = useContext(LayoutContext);

  const style = FORCE_FIXED_SIZE
    ? {
        "--layout-height": `${height}px`,
        "--layout-width": `${width}px`,
      }
    : {};

  return (
    <div
      ref={rootRef}
      style={style}
      className={clsx({
        "layout": true,
        "layout--dark": true,
        [`layout--${DEVICE_TYPE}`]: true,
        "layout--pwa": isPWA,
        "layout--portrait": orientation === 0,
        "layout--landscape_ccw": orientation === 90,
        "layout--landscape_cw": orientation === -90,
        "layout--min_aspect_ratio_1": aspectRatio >= 1.2,
        "layout--min_aspect_ratio_2": aspectRatio >= 2,
        "layout--min_aspect_ratio_4": aspectRatio >= 3,
        "layout--min_width_sm": width >= 576,
        "layout--min_width_md": width >= 768,
        "layout--min_width_lg": width >= 992,
        "layout--min_width_xl": width >= 1200,
        "layout--meme_open": !closed,
        "layout--swap": swapMainPanels,
        "layout--hide_chat": !showChat,
        "layout--hide_video": !showVideo,
        "layout--theater_mode": theaterMode,
        "layout--expand_nav": expandNav,
      })}
    >
      <div className="deadzone"></div>
      <div className="header">
        <Header />
      </div>
      <div className="body">
        <div className="nav_panel">
          <NetworkNav />
        </div>
        <main className="foo_1">
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
            style={{ "--layout-offset": `${dx}px` }}
            {...dragHandlers}
          >
            {showVideo && (
              <div className="video_panel">
                <button onClick={toggleClosed}>toggle</button>
                <button onClick={() => toggleShowChat((prev) => !prev)}>toggle chat</button>
                <button onClick={() => toggleShowVideo((prev) => !prev)}>toggle video</button>
                <button onClick={() => toggleTheaterMode((prev) => !prev)}>
                  toggle theater mode
                </button>
              </div>
            )}
            <div className="chat_panel">
              <TestChat />
              {/* <button onClick={() => toggleSwapMainPanels((prev) => !prev)}>swap sides</button>
              <button onClick={toggleClosed}>toggle</button>
              <button onClick={() => toggleShowChat((prev) => !prev)}>toggle chat</button>
              <button onClick={() => toggleShowVideo((prev) => !prev)}>toggle video</button>
              <button onClick={() => toggleTheaterMode((prev) => !prev)}>toggle theater mode</button>
              <Scrollbars
                renderView={(props) => <div {...props} className={"chat_panel__scroller"} />}
              >
                {new Array(40).fill(0).map((_, i) => (
                  <div key={i}>{i}</div>
                ))}
              </Scrollbars> */}
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
        </main>
      </div>
      <div className="footer"></div>
    </div>
  );
};

const LayoutTest: React.FC = () => {
  const [[chatService, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService();
    registerChatFrontendService(svc, chatService);
    registerNetworkServiceService(svc, new NetworkService());

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatService.destroy(), [chatService]);

  return (
    <ApiProvider value={client}>
      <NetworkProvider>
        <ThemeProvider>
          <LayoutContextProvider>{LayoutPage}</LayoutContextProvider>
        </ThemeProvider>
      </NetworkProvider>
    </ApiProvider>
  );
};

export default LayoutTest;
