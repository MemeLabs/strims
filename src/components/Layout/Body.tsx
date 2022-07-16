// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, Suspense, useEffect } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { Outlet } from "react-router";
import { useToggle } from "react-use";

import NetworkNav from "../../components/Layout/NetworkNav";
import { useLayout } from "../../contexts/Layout";
import LoadingPlaceholder from "../LoadingPlaceholder";
import SwipablePanel, { DragState } from "../SwipablePanel";
import VideoMeta from "../VideoMeta";
import Chat from "./Chat";
import ChatBar from "./ChatBar";
import Player from "./Player";

interface LayoutBodyProps {
  children?: ReactNode;
}

export const LayoutBody: React.FC<LayoutBodyProps> = ({ children }) => {
  const { showVideo, overlayState, setOverlayState } = useLayout();

  const handleDragStateChange = (state: DragState) => {
    setOverlayState({
      open: !state.closed,
      transitioning: state.transitioning,
    });
  };

  const [open, toggleOpen] = useToggle(false);
  useEffect(() => {
    if (!overlayState.transitioning) {
      toggleOpen(overlayState.open);
    }
  }, [overlayState]);

  return (
    <>
      <div className="layout__nav">
        <NetworkNav />
      </div>
      <main className="foo_1">
        <div className="content_panel">
          <Scrollbars autoHide>
            <div className="scroll_content_test">
              <Suspense fallback={<LoadingPlaceholder />}>
                <Outlet />
                {children}
              </Suspense>
            </div>
          </Scrollbars>
        </div>
        <SwipablePanel
          className="foo_2"
          direction="left"
          open={open}
          onDragStateChange={handleDragStateChange}
          preventScroll={true}
        >
          {showVideo && (
            <div className="layout__video">
              <Player>
                <VideoMeta />
              </Player>
            </div>
          )}
          <div className="layout__chat">
            <Chat />
          </div>
        </SwipablePanel>
        <div className="layout__chat_bar">
          <ChatBar />
        </div>
      </main>
    </>
  );
};

export default LayoutBody;
