// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/storybook.scss";

import clsx from "clsx";
import React, { ReactNode, useCallback, useEffect, useLayoutEffect, useMemo, useState } from "react";
import {
  MdComputer,
  MdDarkMode,
  MdFullscreen,
  MdFullscreenExit,
  MdLightMode,
  MdRefresh,
  MdSmartphone,
  MdStayCurrentLandscape,
} from "react-icons/md";
import { useLocation, useSearchParams } from "react-router-dom";

import { WithThemeProps, withTheme } from "../../components/Theme";
import { Provider as ThemeProvider, useTheme } from "../../contexts/Theme";
import Nav from "../components/Nav";
import { StoryHostContext, StoryHostValue } from "../storybook/context";
import CatalogPage from "../storybook/host/CatalogPage";
import { ROOT, matchRoute } from "../storybook/host/catalog";
import ErrorList from "../storybook/host/ErrorList";
import StorybookNav from "../storybook/host/Nav";
import { Session, activate } from "../storybook/host/recorder";
import { DEVICES, Device, StoryView } from "../storybook/host/StoryView";

// The toolbar's state lives in the route's query, so links (and
// hack/storybook/check.py) set it: theme=dark|light, device=pc|portrait|landscape,
// fullscreen=1.
const useToolbarParams = () => {
  const [params, setParams] = useSearchParams();
  const theme = params.get("theme") === "light" ? "light" : "dark";
  const device = (["portrait", "landscape"].find((d) => d === params.get("device")) ?? "pc") as Device;
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

const ThemeSync: React.FC<{ theme: "dark" | "light" }> = ({ theme }) => {
  const { colorScheme, setColorScheme } = useTheme();
  useLayoutEffect(() => {
    if (colorScheme !== theme) {
      setColorScheme(theme);
    }
  }, [theme, colorScheme]);
  return null;
};

const StoryContainer = withTheme<{ children: ReactNode }>(
  ({ className, children }: WithThemeProps & { children: ReactNode }) => (
    <div className={clsx(className, "storybook__content")}>{children}</div>
  )
);

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

const DEVICE_ICONS: Record<Device, ReactNode> = {
  pc: <MdComputer />,
  portrait: <MdSmartphone />,
  landscape: <MdStayCurrentLandscape />,
};

const Storybook: React.FC = () => {
  const { pathname } = useLocation();
  const { theme, device, fullscreen, set } = useToolbarParams();
  const match = matchRoute(pathname);

  const [nonce, setNonce] = useState(0);
  const [routerPath, setRouterPath] = useState<string>(null);
  const host = useMemo<StoryHostValue>(() => ({ setRouterPath }), []);

  // A session per mount of a story, active from its first render, so errors
  // made while it renders count.
  const session = useMemo(() => {
    if (!match) {
      return null;
    }
    const session = new Session(pathname);
    activate(session);
    return session;
  }, [match?.key, pathname, nonce]);
  useEffect(() => (session ? activate(session) : undefined), [session]);

  return (
    <>
      <Nav />
      <div className={clsx("storybook", { "storybook--fullscreen": fullscreen })}>
        <div className="storybook__nav">
          <StorybookNav />
        </div>
        <ThemeProvider>
          <ThemeSync theme={theme} />
          <StoryContainer>
            <div className="storybook__toolbar">
              <span className="storybook__toolbar_title">{match ? pathname.substring(ROOT.length + 1) : "catalog"}</span>
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
              <ToolButton title="fullscreen" onClick={() => set("fullscreen", fullscreen ? null : "1")}>
                {fullscreen ? <MdFullscreenExit /> : <MdFullscreen />}
              </ToolButton>
            </div>
            <div className={clsx("storybook__stage", `storybook__stage--${match ? device : "catalog"}`)}>
              {match ? (
                <StoryHostContext.Provider value={host}>
                  <StoryView
                    key={`${pathname}#${nonce}`}
                    moduleKey={match.key}
                    name={match.name}
                    device={device}
                    session={session}
                  />
                </StoryHostContext.Provider>
              ) : (
                <CatalogPage notFound={pathname.replace(/\/$/, "") === ROOT ? undefined : pathname} />
              )}
            </div>
            {session && <ErrorList session={session} />}
          </StoryContainer>
        </ThemeProvider>
      </div>
    </>
  );
};

export default Storybook;
