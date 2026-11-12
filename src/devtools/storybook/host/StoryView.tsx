// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, {
  ReactNode,
  Suspense,
  lazy,
  useCallback,
  useEffect,
  useLayoutEffect,
  useState,
  useSyncExternalStore,
} from "react";
import { Navigate, useLocation } from "react-router-dom";

import { layoutShapeClassNames, layoutStateClassNames } from "../../../components/Layout/shape";
import { WithRootRefProps, useLayout, withLayoutContext } from "../../../contexts/Layout";
import { DEVICE_TYPE } from "../../../lib/userAgent";
import type { Story } from "../types";
import { loadModule, storyRoute } from "./catalog";
import StoryErrorBoundary from "./ErrorBoundary";
import { Session } from "./recorder";

export type Device = "pc" | "portrait" | "landscape";

interface DeviceShape {
  width: number;
  height: number;
  /** As window.orientation reads it. */
  orientation: number;
}

// Device frames: the content area (null), or a fixed phone-sized viewport.
export const DEVICES: Record<Device, DeviceShape | null> = {
  pc: null,
  portrait: { width: 390, height: 844, orientation: 0 },
  landscape: { width: 844, height: 390, orientation: 90 },
};

const useElementSize = (el: HTMLElement | null) => {
  const [size, setSize] = useState({ width: 0, height: 0 });
  useLayoutEffect(() => {
    if (!el) {
      return;
    }
    const measure = () => setSize({ width: el.clientWidth, height: el.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [el]);
  return size;
};

interface FrameProps {
  device: Device;
  session: Session;
  children: ReactNode;
}

// The story's root, and the Layout context's portal root, so modals and menus
// stay in the frame. It carries the classes and variables the app's Layout
// derives for its size, but the device type (layout--pc or --portable) is the
// user agent's, read once at load (lib/userAgent): components that branch on
// it (SwipablePanel, ChatBar, Footer or Header) render the desktop version
// in a phone frame too.
const FrameBase: React.FC<FrameProps & WithRootRefProps> = ({ rootRef, device, session, children }) => {
  const [el, setEl] = useState<HTMLElement>(null);
  const ref = useCallback((e: HTMLElement) => {
    rootRef(e);
    setEl(e);
  }, []);
  const measured = useElementSize(DEVICES[device] ? null : el);
  const { width, height, orientation } = DEVICES[device] ?? { ...measured, orientation: 0 };
  const layout = useLayout();
  const { state } = useSyncExternalStore(session.subscribe, session.getSnapshot);

  const style = {
    "--layout-width": `${width}px`,
    "--layout-height": `${height}px`,
    ...(DEVICES[device] ? { width: `${width}px`, height: `${height}px` } : {}),
  } as React.CSSProperties;

  return (
    <div
      ref={ref}
      style={style}
      className={clsx(
        "storybook_frame",
        `storybook_frame--${device}`,
        `layout--${DEVICE_TYPE}`,
        width && height ? layoutShapeClassNames(width, height, orientation) : null,
        layoutStateClassNames(layout)
      )}
      data-story-root=""
      data-story-route={session.route}
      data-story-state={state}
    >
      {children}
    </div>
  );
};

const Frame = withLayoutContext(FrameBase);

interface ModuleViewProps {
  name?: string;
  session: Session;
}

const moduleViews = new Map<string, React.ComponentType<ModuleViewProps>>();

// One lazy component per module, so re-renders of the page (and remounts)
// don't load it again.
const moduleView = (key: string): React.ComponentType<ModuleViewProps> => {
  let view = moduleViews.get(key);
  if (!view) {
    view = lazy(async () => {
      const mod = await loadModule(key);
      const stories: Story[] = Array.isArray(mod.default) ? mod.default : [];
      const View: React.FC<ModuleViewProps> = ({ name, session }) => {
        const { search } = useLocation();
        const story = stories.find((s) => s.name === name);
        session.setStoryAllow(story?.allowErrors);
        if (!story) {
          if (!stories.length) {
            return <p className="storybook__message">{key} has no stories</p>;
          }
          return <Navigate to={{ pathname: storyRoute(key, stories[0].name), search }} replace />;
        }
        const C = story.component;
        return <C />;
      };
      View.displayName = `Stories(${key})`;
      return { default: View };
    });
    moduleViews.set(key, view);
  }
  return view;
};

const Mounted: React.FC<{ session: Session }> = ({ session }) => {
  useEffect(() => session.setMounted(), []);
  return null;
};

const LoadingMessage = () => <p className="loading_message">loading</p>;

export interface StoryViewProps {
  moduleKey: string;
  name?: string;
  device: Device;
  session: Session;
}

// A story in its frame, its errors caught and recorded into `session`.
export const StoryView: React.FC<StoryViewProps> = ({ moduleKey, name, device, session }) => {
  const View = moduleView(moduleKey);
  return (
    <Frame device={device} session={session}>
      <StoryErrorBoundary onError={(text) => session.crash(text)}>
        <Suspense fallback={<LoadingMessage />}>
          <View name={name} session={session} />
          <Mounted session={session} />
        </Suspense>
      </StoryErrorBoundary>
    </Frame>
  );
};
