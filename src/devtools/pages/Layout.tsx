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

const [DEVICE_TYPE, OS] = ((): [DeviceType, string] => {
  const ua = new UAParser();
  return [
    {
      "console": DeviceType.TV,
      "smarttv": DeviceType.TV,
      "mobile": DeviceType.Portable,
      "tablet": DeviceType.Portable,
      "wearable": DeviceType.Portable,
      "embedded": DeviceType.Portable,
    }[ua.getDevice().type] ?? DeviceType.PC,
    ua.getOS().name,
  ];
})();
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
  if (isPWA && OS === "iOS") {
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

interface LayoutPageProps {
  rootRef: RefCallback<HTMLElement>;
}

const LayoutPage: React.FC<LayoutPageProps> = ({ rootRef }) => {
  const [viewportShape, setViewportShape] = useState(getViewportShape);

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
        console.log("event stopped");
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

  const foo2 = useRef<HTMLDivElement>(null);
  const setDX = (v: number) => foo2.current?.style.setProperty("--layout-offset", `${v}px`);

  const [memeState, setClosed] = useState([true, false, false]);
  const [closed, closing, dragging] = memeState;
  const toggleClosed = () => {
    setDX(0);
    setClosed(([closed]) => [!closed, closed, false]);
  };
  const threshold = 200;

  const dragHandlers =
    DEVICE_TYPE === DeviceType.Portable
      ? useDrag(({ movement: [mx], swipe: [sx], dragging }) => {
          const prev = memeState;
          let next: [boolean, boolean, boolean];
          if (dragging) {
            if (closing) {
              setDX(Math.max(mx, 0));
              next = [false, true, true];
            } else {
              setDX(Math.max(-mx, 0));
              next = [mx >= -10, false, true];
            }
          } else {
            const closed =
              (closing && (sx === 1 || mx > threshold)) ||
              (!closing && sx !== -1 && mx > -threshold);
            setDX(0);
            next = [closed, !closed, false];
          }
          if (prev[0] !== next[0] || prev[1] !== next[1] || prev[2] !== next[2]) {
            setClosed(next);
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
      className={clsx({
        "layout": true,
        "layout--dark": true,
        [`layout--${DEVICE_TYPE}`]: true,
        "layout--pwa": isPWA,
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
            ref={foo2}
            className={clsx({
              "foo_2": true,
              "foo_2--dragging": dragging,
              "foo_2--closing": closing,
            })}
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
                <button onClick={() => toggleSwapMainPanels((prev) => !prev)}>swap sides</button>
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
