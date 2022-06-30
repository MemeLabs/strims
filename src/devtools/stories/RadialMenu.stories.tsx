// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Base64 } from "js-base64";
import React, { useEffect, useMemo, useRef, useState } from "react";
import usePortal from "use-portal";

import { FrontendClient } from "../../apis/client";
import { registerChatFrontendService } from "../../apis/strims/chat/v1/chat_rpc";
import { registerDirectoryFrontendService } from "../../apis/strims/network/v1/directory/directory_rpc";
import Emote from "../../components/Chat/Emote";
import ChatPanel from "../../components/Chat/Shell";
import { Provider as ChatProvider } from "../../contexts/Chat";
import { Provider as DirectoryProvider } from "../../contexts/Directory";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import { useLayout } from "../../contexts/Layout";
import { useStableCallback } from "../../hooks/useStableCallback";
import { AsyncPassThrough } from "../../lib/stream";
import { RoomProvider } from "../contexts/Chat";
import { emoteNames } from "../mocks/chat/assetBundle";
import ChatService from "../mocks/chat/service";
import DirectoryService from "../mocks/directory/service";

interface RadialMenuProps {
  segmentCount: number;
  cx: number;
  cy: number;
  onClose: () => void;
}

const RadialMenu: React.FC<RadialMenuProps> = ({ segmentCount, cx, cy, onClose }) => {
  const style = useMemo(() => ({ left: `${cx}px`, top: `${cy}px` }), [cx, cy]);

  const [selectedSegment, setSelectedSegment] = useState(-1);

  const stepSize = (Math.PI * 2) / segmentCount;

  const ringStyle = useMemo(() => {
    const segments = [];
    for (let i = 0; i < segmentCount; i++) {
      const color = i === selectedSegment ? "--background-color-active" : "--background-color";
      segments.push(`var(${color}) ${i * stepSize}rad ${(i + 1) * stepSize}rad`);
    }
    return {
      cursor: selectedSegment === -1 ? "default" : "pointer",
      backgroundImage: `conic-gradient(${segments.join(",")})`,
    };
  }, [selectedSegment]);

  const ref = useRef<HTMLDivElement>();

  const handleMouseMove = useStableCallback((e: MouseEvent) => {
    if (!ref.current) {
      return;
    }

    const bounds = ref.current.getBoundingClientRect();
    const dy = e.pageY - (bounds.top + bounds.height / 2);
    const dx = e.pageX - (bounds.left + bounds.width / 2);
    const d = Math.sqrt(dx * dx + dy * dy);

    if (d < 60 || d > 150) {
      setSelectedSegment(-1);
    } else {
      const a = Math.atan2(dy, dx);
      setSelectedSegment(Math.floor(((a + Math.PI * 2.5) % (Math.PI * 2)) / stepSize));
    }
  });

  const handleMouseUp = useStableCallback(() => {
    if (selectedSegment !== -1) {
      console.log(">>>", selectedSegment);
    }
    onClose();
  });

  useEffect(() => {
    document.body.addEventListener("mousemove", handleMouseMove);
    document.body.addEventListener("mouseup", handleMouseUp);
    document.body.addEventListener("mouseleave", onClose);

    return () => {
      document.body.removeEventListener("mousemove", handleMouseMove);
      document.body.removeEventListener("mouseup", handleMouseUp);
      document.body.removeEventListener("mouseleave", onClose);
    };
  }, [onClose]);

  const elems = useMemo(() => {
    const elems: React.ReactNode[] = [];
    for (let i = 0; i < segmentCount; i++) {
      const a = (i - 1) * stepSize;
      const r = 60 + (150 - 60) / 2;

      elems.push(
        <Emote
          key={`${emoteNames[i]}`}
          name={emoteNames[i]}
          shouldAnimateForever
          style={{
            "--menu-x": `${Math.cos(a) * r + 150}px`,
            "--menu-y": `${Math.sin(a) * r + 150}px`,
            "--menu-scale": selectedSegment === i ? "1.1" : "1",
          }}
        >
          {emoteNames[i]}
        </Emote>
      );
    }
    return elems;
  }, [segmentCount, selectedSegment]);

  const layout = useLayout();
  const { Portal } = usePortal({ target: layout.root });

  return (
    <Portal>
      <div className="radial_menu" ref={ref} style={style}>
        <div className="radial_menu__ring" style={ringStyle} />
        {elems}
      </div>
    </Portal>
  );
};

type MenuState =
  | {
      open: true;
      cx: number;
      cy: number;
    }
  | {
      open: false;
    };

const Test: React.FC = () => {
  const [[chatService, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService();
    const directoryService = new DirectoryService();
    registerChatFrontendService(svc, chatService);
    registerDirectoryFrontendService(svc, directoryService);

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatService.destroy(), [chatService]);

  const [state, setState] = useState<MenuState>(() => ({ open: false }));

  const layout = useLayout();

  const handleMouseDown = useStableCallback((e: React.MouseEvent) => {
    if (e.button === 1) {
      const rootBounds = layout.root.getBoundingClientRect();
      setState({
        open: true,
        cx: e.pageX - rootBounds.left,
        cy: e.pageY - rootBounds.top,
      });
    }
  });

  const handleClose = useStableCallback(() => setState({ open: false }));

  return (
    <div className="chat_mockup">
      <ApiProvider value={client}>
        <DirectoryProvider>
          <ChatProvider>
            <RoomProvider
              networkKey={Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=")}
              serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
            >
              <ChatPanel />
              <div className="chat_mockup__test" onMouseDown={handleMouseDown}>
                {state.open && (
                  <RadialMenu segmentCount={6} cx={state.cx} cy={state.cy} onClose={handleClose} />
                )}
              </div>
            </RoomProvider>
          </ChatProvider>
        </DirectoryProvider>
      </ApiProvider>
    </div>
  );
};

export default [
  {
    name: "RadialMenu",
    component: () => <Test />,
  },
];
