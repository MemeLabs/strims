// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./LayoutControl.scss";

import { useDrag } from "@use-gesture/react";
import React, { useRef } from "react";

import { useLayout } from "../../contexts/Layout";
import { useTheme } from "../../contexts/Theme";

const LayoutControl: React.FC = () => {
  const ref = useRef<HTMLDivElement>(null);
  const handle = useRef<HTMLDivElement>(null);
  const pos = useRef({ x: 0, y: 0 }).current;

  const move = (x: number, y: number) => {
    ref.current.style.setProperty("left", `${pos.x + x}px`);
    ref.current.style.setProperty("top", `${pos.y + y}px`);
  };

  useDrag(
    ({ movement: [mx, my], down }) => {
      if (down) {
        move(mx, my);
      } else {
        pos.x += mx;
        pos.y += my;
      }
    },
    {
      target: handle,
    }
  );

  const {
    showVideo,
    showChat,
    theaterMode,
    overlayState,
    toggleShowVideo,
    toggleShowChat,
    toggleTheaterMode,
    toggleSwapMainPanels,
    toggleOverlayOpen,
  } = useLayout();

  const { colorScheme, setColorScheme } = useTheme();

  return (
    <div ref={ref} className="layout_control">
      <div ref={handle} className="layout_control__handle" />
      <button onClick={() => toggleShowVideo()}>{showVideo ? "hide" : "show"} video</button>
      <button onClick={() => toggleShowChat()}>{showChat ? "hide" : "show"} chat</button>
      <button onClick={() => toggleTheaterMode()}>
        {theaterMode ? "disable" : "enable"} theater mode
      </button>
      <button onClick={() => toggleSwapMainPanels()}>swap main panels</button>
      <button onClick={() => toggleOverlayOpen(!overlayState.open)}>toggle overlay</button>
      <button onClick={() => setColorScheme(colorScheme === "dark" ? "light" : "dark")}>
        toggle theme
      </button>
    </div>
  );
};

export default LayoutControl;
