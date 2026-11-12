// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, {
  ReactNode,
  Suspense,
  lazy,
  useEffect,
  useLayoutEffect,
  useState,
  useSyncExternalStore,
} from "react";

import { layoutShapeClassNames, layoutStateClassNames } from "../../../components/Layout/shape";
import { WithThemeProps, withTheme } from "../../../components/Theme";
import { WithRootRefProps, useLayout, withLayoutContext } from "../../../contexts/Layout";
import { DEVICE_TYPE } from "../../../lib/userAgent";
import type { Story } from "../types";
import { loadModule } from "./catalog";
import { DEVICES, Device } from "./devices";
import StoryErrorBoundary from "./ErrorBoundary";
import { Session } from "./recorder";

// The frame document's viewport: the device's size on the storybook page
// (its iframe's) and in hack/storybook/check.py (its browser's).
const useViewportSize = () => {
  const [size, setSize] = useState(() => ({
    width: window.innerWidth,
    height: window.innerHeight,
  }));
  useLayoutEffect(() => {
    const measure = () => setSize({ width: window.innerWidth, height: window.innerHeight });
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);
  return size;
};

interface FrameProps {
  device: Device;
  session: Session;
  children: ReactNode;
}

// The story's root, filling the frame document's viewport, and the Layout
// context's portal root. It carries the theme and the classes and
// variables the app's Layout derives for its viewport; the device type
// (layout--pc or --portable) is the realm's user agent's, a phone's in a
// phone frame (preload.ts).
const FrameBase: React.FC<FrameProps & WithRootRefProps & WithThemeProps> = ({
  rootRef,
  className,
  device,
  session,
  children,
}) => {
  const { width, height } = useViewportSize();
  const orientation = DEVICES[device]?.orientation ?? 0;
  const layout = useLayout();
  const { state } = useSyncExternalStore(session.subscribe, session.getSnapshot);

  const style = {
    "--layout-width": `${width}px`,
    "--layout-height": `${height}px`,
  } as React.CSSProperties;

  return (
    <div
      ref={rootRef}
      style={style}
      className={clsx(
        className,
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

const Frame = withTheme(withLayoutContext(FrameBase));

interface ModuleViewProps {
  name?: string;
  session: Session;
}

// The module's story (its first when the route names none).
const moduleView = (key: string): React.ComponentType<ModuleViewProps> =>
  lazy(async () => {
    const mod = await loadModule(key);
    const stories: Story[] = Array.isArray(mod.default) ? mod.default : [];
    const View: React.FC<ModuleViewProps> = ({ name, session }) => {
      const story = name === undefined ? stories[0] : stories.find((s) => s.name === name);
      session.setStoryAllow(story?.allowErrors);
      if (!story) {
        return (
          <p className="storybook__message">
            {stories.length ? `${key} has no story named ${name}` : `${key} has no stories`}
          </p>
        );
      }
      const C = story.component;
      return <C />;
    };
    View.displayName = `Stories(${key})`;
    return { default: View };
  });

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
  const [View] = useState(() => moduleView(moduleKey));
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
