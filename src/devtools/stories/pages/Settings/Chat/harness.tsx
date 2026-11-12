// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./harness.stories.scss";

import React, { ReactElement, ReactNode, useEffect, useRef, useState } from "react";
import { Route } from "react-router-dom";

import { HLSEgressIsSupportedResponse } from "../../../../../apis/strims/video/v1/hls_egress";
import { UnimplementedHLSEgressService } from "../../../../../apis/strims/video/v1/hls_egress_rpc";
import { VideoIngressIsSupportedResponse } from "../../../../../apis/strims/video/v1/ingress";
import { UnimplementedVideoIngressService } from "../../../../../apis/strims/video/v1/ingress_rpc";
import SettingsLayout from "../../../../../pages/Settings/Layout";
import { createSettingsRoutes } from "../../../../../root/MainRouter";
import ChatServerService from "../../../../mocks/chatserver/service";
import NetworkService from "../../../../mocks/network/service";
import { MockApi, StoryRouter } from "../../../../storybook";

// The settings nav asks whether the node has video ingress and HLS egress.
class VideoIngressService extends UnimplementedVideoIngressService {
  isSupported() {
    return new VideoIngressIsSupportedResponse({ supported: false });
  }
}

class HLSEgressService extends UnimplementedHLSEgressService {
  isSupported() {
    return new HLSEgressIsSupportedResponse({ supported: false });
  }
}

/** Something a story does to its rendered page once mounted (the checker never clicks). */
export type Act = (root: HTMLElement) => Promise<void>;

export interface ChatSettingsProps {
  /** Where the story starts, e.g. "/settings/chat-servers/1/emotes". */
  path: string;
  /** The chat server mock; default the fixtures'. */
  service?: () => ChatServerService;
  /** Mounts this at `path` in the settings layout in place of the app's settings routes. */
  element?: ReactElement;
  act?: Act;
}

const Acting: React.FC<{ act?: Act; children: ReactNode }> = ({ act, children }) => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    // A failed act fails the story: the state it shows isn't the one named.
    // eslint-disable-next-line no-console
    act?.(ref.current).catch((e) => console.error("story act failed:", e));
  }, []);
  return (
    <div ref={ref} className="chat_settings_story">
      {children}
    </div>
  );
};

// The settings pages at `path`: the app's settings routes (the chat servers'
// route table among them) in the settings layout, over the chat server and
// network mocks (fixture networks 1..5, the servers' networks).
export const ChatSettings: React.FC<ChatSettingsProps> = ({
  path,
  service = () => new ChatServerService(),
  element,
  act,
}) => (
  <MockApi
    services={() => ({
      chatServer: service(),
      network: new NetworkService({ count: 5 }),
      videoIngress: new VideoIngressService(),
      hlsEgress: new HLSEgressService(),
    })}
  >
    <Acting act={act}>
      <StoryRouter initialPath={path}>
        {element ? (
          <Route path="settings" element={<SettingsLayout />}>
            <Route path={path.replace(/^\/settings\//, "")} element={element} />
          </Route>
        ) : (
          createSettingsRoutes(<SettingsLayout />)
        )}
      </StoryRouter>
    </Acting>
  </MockApi>
);

// What `load` gives, once it has (undefined till then); a failed load fails the story.
export const useLoaded = <T,>(load: () => Promise<T>): T | undefined => {
  const [value, setValue] = useState<T>();
  useEffect(() => {
    // eslint-disable-next-line no-console
    load().then(setValue, (e) => console.error("story load failed:", e));
  }, []);
  return value;
};

// The checker takes a story's picture a second after it mounts (or once the
// network has been quiet for a moment): acts poll often to finish by then.
const POLL_MS = 10;

// Waits until `test` gives something.
const poll = async <T,>(test: () => T, what: string, timeout: number): Promise<T> => {
  const deadline = performance.now() + timeout;
  for (;;) {
    const v = test();
    if (v) {
      return v;
    }
    if (performance.now() > deadline) {
      throw new Error(`${what} after ${timeout} ms`);
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_MS));
  }
};

// The first element under `root` matching `selector` (and `test`), once there is one.
export const waitFor = <T extends Element = HTMLElement>(
  root: HTMLElement,
  selector: string,
  test: (el: T) => boolean = () => true,
  timeout = 4000
): Promise<T> =>
  poll(() => Array.from(root.querySelectorAll<T>(selector)).find(test), `no ${selector}`, timeout);

// Waits until `test` holds.
export const waitUntil = async (test: () => boolean, what: string, timeout = 4000) => {
  await poll(test, `not ${what}`, timeout);
};

// Sets an input's value as typing would, so React's onChange hears it.
export const typeInto = (input: HTMLInputElement | HTMLTextAreaElement, value: string): void => {
  const proto = Object.getPrototypeOf(input) as object;
  Object.getOwnPropertyDescriptor(proto, "value").set.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
};

// The form field labelled `label`.
const field = (root: HTMLElement, label: string): Promise<HTMLElement> =>
  waitFor(
    root,
    ".input_label",
    (el) => el.querySelector(".input_label__text")?.textContent.trim() === label
  );

// Types `value` into the text input labelled `label`.
export const fill = async (root: HTMLElement, label: string, value: string): Promise<void> =>
  typeInto((await field(root, label)).querySelector("input, textarea"), value);

// Scrolls the field labelled `label` into view.
export const scrollTo = async (root: HTMLElement, label: string): Promise<void> =>
  (await field(root, label)).scrollIntoView({ block: "center" });

// Submits the page's form, which validates it.
export const submit = async (root: HTMLElement): Promise<void> => {
  const form = await waitFor<HTMLFormElement>(root, "form.thing_form");
  form.requestSubmit();
};

// Opens the react-select labelled `label` and picks the option `option`.
export const select = async (root: HTMLElement, label: string, option: string): Promise<void> => {
  // A key opens the menu: a click wants focus, which a headless page may not get.
  const input = await waitFor(await field(root, label), ".input_select__input");
  input.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "ArrowDown" }));
  const el = await waitFor(document.body, ".input_select__option", (o) => o.textContent === option);
  el.click();
};

// Clicks the first element matching `selector` (and `test`).
export const click = async (
  root: HTMLElement,
  selector: string,
  test?: (el: HTMLElement) => boolean
): Promise<void> => (await waitFor(root, selector, test)).click();

// Drops the PNGs `files` on a file input, one by one, waiting after each
// until `dropped(count)` says the page took it.
export const dropFiles = async (
  input: HTMLInputElement,
  files: { name: string; url: string }[],
  dropped: (count: number) => boolean
): Promise<void> => {
  const blobs = await Promise.all(files.map(async ({ url }) => (await fetch(url)).blob()));
  for (let i = 0; i < files.length; i++) {
    const dt = new DataTransfer();
    dt.items.add(new File([blobs[i]], `${files[i].name}.png`, { type: "image/png" }));
    input.files = dt.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
    await waitUntil(() => dropped(i + 1), `${i + 1} files dropped`);
  }
};
