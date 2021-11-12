import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import { isEqual } from "lodash";
import React, { useRef } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";

import NetworkNav from "../../components/Layout/NetworkNav";
import { ContentState, useLayout } from "../../contexts/Layout";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import Chat from "./Chat";
import Player from "./Player";

const DRAG_THRESHOLD = 200;

export const LayoutBody: React.FC = ({ children }) => {
  const layout = useLayout();

  const foo2 = useRef<HTMLDivElement>(null);
  const setDragOffset = (v: number) =>
    foo2.current?.style.setProperty("--layout-drag-offset", `${v}px`);

  const toggleClosed = () => {
    setDragOffset(0);
    layout.setShowContent(({ closed }) => ({
      closed: !closed,
      closing: closed,
      dragging: false,
    }));
  };

  if (DEVICE_TYPE === DeviceType.Portable) {
    useDrag(
      ({ movement: [mx], swipe: [sx], dragging }) => {
        let next: ContentState;
        if (dragging) {
          if (layout.showContent.closing) {
            setDragOffset(Math.max(mx, 0));
            next = { closed: false, closing: true, dragging: true };
          } else {
            setDragOffset(Math.max(-mx, 0));
            next = { closed: mx >= -10, closing: false, dragging: true };
          }
        } else {
          const closed =
            (layout.showContent.closing && (sx === 1 || mx > DRAG_THRESHOLD)) ||
            (!layout.showContent.closing && sx !== -1 && mx > -DRAG_THRESHOLD);
          setDragOffset(0);
          next = { closed, closing: !closed, dragging: false };
        }
        if (!isEqual(layout.showContent, next)) {
          layout.setShowContent(next);
        }
      },
      {
        target: foo2,
        eventOptions: {
          capture: true,
          passive: false,
        },
      }
    );
  }

  const testButtons = (
    <>
      <div>
        <button onClick={toggleClosed}>toggle</button>
      </div>
      <div>
        <button onClick={() => layout.toggleShowChat()}>toggle chat</button>
      </div>
      <div>
        <button onClick={() => layout.toggleShowVideo()}>toggle video</button>
      </div>
      <div>
        <button onClick={() => layout.toggleTheaterMode()}>toggle theater mode</button>
      </div>
      <div>
        <button onClick={() => layout.toggleSwapMainPanels()}>swap sides</button>
      </div>
    </>
  );

  return (
    <>
      <div className="layout__nav">
        <NetworkNav />
      </div>
      <main className="foo_1">
        <div className="content_panel">
          <Scrollbars autoHide>
            <div className="scroll_content_test">{children}</div>
          </Scrollbars>
        </div>
        <div
          ref={foo2}
          className={clsx({
            "foo_2": true,
            "foo_2--dragging": layout.showContent.dragging,
            "foo_2--closing": layout.showContent.closing,
          })}
        >
          {layout.showVideo && (
            <div className="layout__video">
              {/* testButtons */}
              <Player />
            </div>
          )}
          <div className="layout__chat">
            <Chat />
          </div>
        </div>
      </main>
    </>
  );
};

export default LayoutBody;
