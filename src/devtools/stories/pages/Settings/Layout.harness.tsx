// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// What the settings stories share: the app's settings route table over
// mock services, and a way to show a form's errors without clicking.

import "../../../../components/Form/Form.scss";
import "../../../../pages/Settings/Layout.scss";
import "./Layout.stories.scss";

import React, { ReactNode, RefObject, useEffect, useRef } from "react";

import SettingsLayout from "../../../../pages/Settings/Layout";
import { createSettingsRoutes } from "../../../../root/MainRouter";
import AutoseedService from "../../../mocks/autoseed/service";
import DebugService from "../../../mocks/debug/service";
import HLSEgressService from "../../../mocks/hlsegress/service";
import NetworkService from "../../../mocks/network/service";
import VideoChannelService from "../../../mocks/videochannel/service";
import VideoIngressService from "../../../mocks/videoingress/service";
import { MockApi, MockServices, StoryRouter } from "../../../storybook";

// The settings pages' services: the story's, the rest default mocks (five
// fixture networks).
export const settingsServices = (own: MockServices = {}): MockServices => ({
  ...own,
  network: own.network ?? new NetworkService({ count: 5 }),
  videoIngress: own.videoIngress ?? new VideoIngressService(),
  hlsEgress: own.hlsEgress ?? new HLSEgressService(),
  videoChannel: own.videoChannel ?? new VideoChannelService(),
  autoseed: own.autoseed ?? new AutoseedService(),
  debug: own.debug ?? new DebugService(),
});

// Long enough after a form renders for the page's calls to have answered
// (the mocks answer at once) and its values to be reset to theirs.
const SUBMIT_DELAY_MS = 300;

// Submits the first form under `ref` once one renders, as its submit
// button would: its validation errors show, or with valid values its save
// call is made.
export const useSubmitOnLoad = (ref: RefObject<HTMLElement>, enabled: boolean): void =>
  useEffect(() => {
    if (!enabled) {
      return;
    }
    let timeout: ReturnType<typeof setTimeout>;
    const interval = setInterval(() => {
      const form = ref.current?.querySelector("form");
      if (form) {
        clearInterval(interval);
        timeout = setTimeout(() => form.requestSubmit(), SUBMIT_DELAY_MS);
      }
    }, 50);
    return () => {
      clearInterval(interval);
      clearTimeout(timeout);
    };
  }, []);

export interface SettingsStoryProps {
  /** Where the story starts, e.g. "/settings/autoseed/config". */
  path: string;
  /** Services over settingsServices' defaults, made once at mount. */
  services?: () => MockServices;
  /** Shows the settings nav beside the page (default the page alone). */
  nav?: boolean;
  /** Submits the page's form once it renders (useSubmitOnLoad). */
  submit?: boolean;
}

// The app's settings routes (createSettingsRoutes in SettingsLayout) at
// `path`, so pages read their params from real paths and links between
// them lead to the real pages.
export const SettingsStory: React.FC<SettingsStoryProps> = ({ path, services, nav, submit }) => {
  const ref = useRef<HTMLDivElement>(null);
  useSubmitOnLoad(ref, submit);
  return (
    <MockApi services={() => settingsServices(services?.())}>
      <div className="settings_story" ref={ref}>
        <StoryRouter initialPath={path}>
          {createSettingsRoutes(<SettingsLayout nav={nav ? undefined : null} />)}
        </StoryRouter>
      </div>
    </MockApi>
  );
};

export interface SettingsContentProps {
  /** Wraps the children in a form's list styles (.thing_form). */
  form?: boolean;
  children: ReactNode;
}

// A settings page's content area without the routes, for components the
// pages are made of.
export const SettingsContent = React.forwardRef<HTMLDivElement, SettingsContentProps>(
  ({ form, children }, ref) => (
    <div className="settings" ref={ref}>
      <div className="settings__body">
        <main className="settings__content">
          {form ? <div className="thing_form">{children}</div> : children}
        </main>
      </div>
    </div>
  )
);
SettingsContent.displayName = "SettingsContent";

/** Text longer than a settings form's row, for wrapping. */
export const longTitle =
  "A title long enough that it can't fit on one line of the form, however wide the window " +
  "is, so it has to wrap onto the next";
export const longDescription = Array(4)
  .fill("A description that runs on well past the width of the form.")
  .join(" ");

// A promise that resolves `ms` later with `value`, for a call that answers late.
export const later = <T,>(ms: number, value: T): Promise<T> =>
  new Promise((resolve) => setTimeout(() => resolve(value), ms));
