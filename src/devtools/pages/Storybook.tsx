// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/storybook.scss";

import clsx from "clsx";
import React, { ReactNode, useCallback, useEffect, useState } from "react";
import {
  MdComputer,
  MdDarkMode,
  MdFullscreen,
  MdFullscreenExit,
  MdLightMode,
  MdOpenInNew,
  MdRefresh,
  MdSmartphone,
  MdStayCurrentLandscape,
  MdTabletAndroid,
  MdTabletMac,
} from "react-icons/md";
import { Navigate, useLocation, useSearchParams } from "react-router";

import Nav from "../components/Nav";
import CatalogPage from "../storybook/host/CatalogPage";
import ErrorList from "../storybook/host/ErrorList";
import StorybookNav from "../storybook/host/Nav";
import { ROOT, matchRoute, modulePath } from "../storybook/host/catalog";
import { DEVICES, Device, isDevice } from "../storybook/host/devices";
import { StoryIframe, documentUrl, useStoryIndex } from "../storybook/host/frames";
import { frameRoute } from "../storybook/host/protocol";
import type { SessionSnapshot } from "../storybook/host/recorder";

// The toolbar's state lives in the route's query, so links (and
// hack/storybook/check.py) set it: theme=dark|light, device=pc|portrait|landscape|
// ipad_portrait|ipad_landscape|tablet_portrait|tablet_landscape, fullscreen=1.
const useToolbarParams = () => {
  const [params, setParams] = useSearchParams();
  const theme = params.get("theme") === "light" ? "light" : "dark";
  const deviceParam = params.get("device");
  const device: Device = isDevice(deviceParam) ? deviceParam : "pc";
  const fullscreen = params.get("fullscreen") === "1";
  const set = useCallback(
    (name: string, value: string | null) =>
      setParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          if (value === null) {
            next.delete(name);
          } else {
            next.set(name, value);
          }
          return next;
        },
        { replace: true }
      ),
    [setParams]
  );
  return { theme, device, fullscreen, set } as const;
};

interface ToolButtonProps {
  title: string;
  active?: boolean;
  onClick: () => void;
  children: ReactNode;
}

const ToolButton: React.FC<ToolButtonProps> = ({ title, active, onClick, children }) => (
  <button
    className={clsx("storybook__toolbar_button", { "storybook__toolbar_button--active": active })}
    title={title}
    onClick={onClick}
  >
    {children}
  </button>
);

// A landscape tablet's icon: its portrait one on its side.
const turned = (icon: ReactNode) => <span className="storybook__toolbar_turned">{icon}</span>;

const DEVICE_ICONS: Record<Device, ReactNode> = {
  pc: <MdComputer />,
  portrait: <MdSmartphone />,
  landscape: <MdStayCurrentLandscape />,
  ipad_portrait: <MdTabletMac />,
  ipad_landscape: turned(<MdTabletMac />),
  tablet_portrait: <MdTabletAndroid />,
  tablet_landscape: turned(<MdTabletAndroid />),
};

const Storybook: React.FC = () => {
  const { pathname, search } = useLocation();
  const { theme, device, fullscreen, set } = useToolbarParams();
  const match = matchRoute(pathname);
  const { index, error: indexError, frame: indexFrame } = useStoryIndex();

  // The story's frame document; a new one for each story, theme, device and
  // remount, so each is a fresh realm.
  const [nonce, setNonce] = useState(0);
  const src = match?.name !== undefined ? frameRoute(pathname, { theme, device }) : null;
  const [snapshot, setSnapshot] = useState<SessionSnapshot>(null);
  const [routerPath, setRouterPath] = useState<string>(null);
  useEffect(() => {
    setSnapshot(null);
    setRouterPath(null);
  }, [src, nonce]);

  // A module's route without a story's name: its first story's.
  const first =
    match && match.name === undefined
      ? index?.stories.find((s) => s.module === modulePath(match.key))
      : undefined;

  let stage: ReactNode;
  if (src) {
    stage = (
      <StoryIframe
        key={`${src}#${nonce}`}
        src={src}
        device={device}
        onSnapshot={setSnapshot}
        onRouterPath={setRouterPath}
      />
    );
  } else if (first) {
    stage = <Navigate to={{ pathname: first.route, search }} replace />;
  } else if (match && !index && !indexError) {
    stage = <p className="loading_message">loading the stories</p>;
  } else if (match) {
    stage = <p className="storybook__message">{modulePath(match.key)} has no stories</p>;
  } else {
    stage = (
      <CatalogPage
        data={index}
        error={indexError}
        notFound={pathname.replace(/\/$/, "") === ROOT ? undefined : pathname}
      />
    );
  }

  return (
    <>
      <Nav />
      <div className={clsx("storybook", { "storybook--fullscreen": fullscreen })}>
        <div className="storybook__nav">
          <StorybookNav index={index} />
        </div>
        <div className="storybook__content">
          <div className="storybook__toolbar">
            <span className="storybook__toolbar_title">
              {match ? pathname.substring(ROOT.length + 1) : "catalog"}
            </span>
            {routerPath !== null && (
              <span className="storybook__router_path" title="the story's own router's path">
                {routerPath}
              </span>
            )}
            <span className="storybook__toolbar_spacer" />
            <ToolButton
              title={`theme: ${theme}`}
              onClick={() => set("theme", theme === "dark" ? "light" : null)}
            >
              {theme === "dark" ? <MdDarkMode /> : <MdLightMode />}
            </ToolButton>
            {(Object.keys(DEVICES) as Device[]).map((d) => (
              <ToolButton
                key={d}
                title={`device: ${d}`}
                active={d === device}
                onClick={() => set("device", d === "pc" ? null : d)}
              >
                {DEVICE_ICONS[d]}
              </ToolButton>
            ))}
            <ToolButton title="remount" onClick={() => setNonce((n) => n + 1)}>
              <MdRefresh />
            </ToolButton>
            {src && (
              <a
                className="storybook__toolbar_button"
                title="the story's frame alone"
                href={documentUrl(src)}
                target="_blank"
                rel="noreferrer"
              >
                <MdOpenInNew />
              </a>
            )}
            <ToolButton
              title="fullscreen"
              onClick={() => set("fullscreen", fullscreen ? null : "1")}
            >
              {fullscreen ? <MdFullscreenExit /> : <MdFullscreen />}
            </ToolButton>
          </div>
          <div
            className={clsx(
              "storybook__stage",
              `storybook__stage--${src ? (DEVICES[device] ? "framed" : device) : "catalog"}`
            )}
          >
            {stage}
          </div>
          {snapshot && <ErrorList snapshot={snapshot} />}
        </div>
        {indexFrame}
      </div>
    </>
  );
};

export default Storybook;
