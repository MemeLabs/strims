// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// What the settings stories (networks, bootstraps, profile, VNIC) share: the
// sections' real route tables under the settings layout, over mocks of every
// service they call, with toasts, a clipboard that shows what was copied, and
// scripted steps (fill, submit, click) that reach a state on mount.

import "./harness.stories.scss";

import clsx from "clsx";
import React, {
  ReactElement,
  ReactNode,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { Route } from "react-router-dom";

import { Image, ImageType } from "../../../../../apis/strims/type/image";
import Toast from "../../../../../components/Notification/Toast";
import BootstrapRoutes from "../../../../../pages/Settings/Bootstrap";
import SettingsLayout from "../../../../../pages/Settings/Layout";
import NetworkRoutes from "../../../../../pages/Settings/Network";
import ProfileRoutes from "../../../../../pages/Settings/Profile";
import VNICPage from "../../../../../pages/Settings/VNIC";
import BootstrapService from "../../../../mocks/bootstrap/service";
import NetworkService from "../../../../mocks/network/service";
import ProfileService from "../../../../mocks/profile/service";
import ReplicationService from "../../../../mocks/replication/service";
import VNICService from "../../../../mocks/vnic/service";
import {
  MockApi,
  MockNotifications,
  MockServices,
  StoryRouter,
  fixtures,
} from "../../../../storybook";

// The services the settings pages call, their mocks' defaults (networks 1..5).
const settingsServices = (): MockServices => ({
  network: new NetworkService({ count: 5 }),
  bootstrap: new BootstrapService(),
  profile: new ProfileService(),
  replication: new ReplicationService(),
  vnic: new VNICService(),
});

// `service` with `methods` answering `ms` late, for loading states (the
// client times calls out at 5 s).
export const slow = <T extends object>(service: T, methods: (keyof T)[], ms = 3500): T =>
  new Proxy(service, {
    get: (target, prop, receiver) => {
      const value: unknown = Reflect.get(target, prop, receiver);
      if (typeof value !== "function" || !methods.includes(prop as keyof T)) {
        return value;
      }
      return (...args: unknown[]) =>
        new Promise((resolve, reject) =>
          setTimeout(() => {
            try {
              resolve(Reflect.apply(value, target, args));
            } catch (e) {
              reject(e);
            }
          }, ms)
        );
    },
  });

// The fixture PNG at `url` as the image messages carry it (type.Image).
export const pngImage = async (url: string): Promise<Image> => {
  const data = await fixtures.imageData(url);
  const img = new window.Image();
  img.src = url;
  await img.decode();
  return new Image({
    type: ImageType.IMAGE_TYPE_PNG,
    data,
    height: img.naturalHeight,
    width: img.naturalWidth,
  });
};

// --- scripted steps ---

/** A step run on the story's root after mount, e.g. filling and submitting a form. */
export type Step = (root: HTMLElement) => Promise<void>;

const FIND_TIMEOUT_MS = 4000;

// The first element in `root` matching `selector` (and holding `text`, if
// given), once it renders.
export const find = async <T extends HTMLElement = HTMLElement>(
  root: HTMLElement,
  selector: string,
  text?: string
): Promise<T> => {
  const deadline = Date.now() + FIND_TIMEOUT_MS;
  for (;;) {
    const el = Array.from(root.querySelectorAll<T>(selector)).find(
      (e) => text === undefined || e.textContent.trim() === text
    );
    if (el) {
      return el;
    }
    if (Date.now() > deadline) {
      throw new Error(
        `storybook: no ${selector}${text === undefined ? "" : ` "${text}"`} rendered`
      );
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
};

// Types `value` into the input named `name`, as React's onChange hears it:
// through the prototype's setter, past the one React puts on the element.
export const fill =
  (name: string, value: string): Step =>
  async (root) => {
    const input = await find<HTMLInputElement>(root, `[name="${name}"]`);
    Reflect.set(Object.getPrototypeOf(input) as object, "value", value, input);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  };

// Submits the page's form (validating it first, as a click on its button would).
export const submit =
  (selector = "form"): Step =>
  async (root) => {
    (await find<HTMLFormElement>(root, selector)).requestSubmit();
  };

// Clicks the element matching `selector` (with `text`, if given): menu items
// need no open menu.
export const click =
  (selector: string, text?: string): Step =>
  async (root) => {
    (await find(root, selector, text)).click();
  };

// Waits for `selector` (with `text`) before the next step.
export const waitFor =
  (selector: string, text?: string): Step =>
  async (root) => {
    await find(root, selector, text);
  };

// Opens the first SelectInput's menu, as its arrow down key does (a mouse
// down opens it only once the input has focus, which a headless page's may not).
export const openSelect = (): Step => async (root) => {
  const input = await find(root, ".input_select__control input");
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
};

// Picks the open SelectInput menu's option labelled `label`.
export const choose = (label: string): Step => click(".input_select__option", label);

// Steps search the story's frame, the layout root menus and modals portal into.
const useSteps = (ref: React.RefObject<HTMLElement>, steps: Step[]) => {
  useEffect(() => {
    let cancelled = false;
    const root = ref.current.closest<HTMLElement>("[data-story-root]") ?? ref.current;
    void (async () => {
      for (const step of steps) {
        if (cancelled) {
          return;
        }
        await step(root);
        // Lets React render what the step did before the next looks.
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
    })().catch((e: Error) => {
      if (!cancelled) {
        // eslint-disable-next-line no-console
        console.error(e);
      }
    });
    return () => void (cancelled = true);
  }, []);
};

// --- the clipboard ---

// navigator.clipboard.writeText answers from memory while the story is
// mounted (headless pages may not write the real one), and the copied text
// shows below the page.
const useClipboard = (): string => {
  const [copied, setCopied] = useState<string>(null);
  useLayoutEffect(() => {
    const clipboard = {
      writeText: (text: string) => {
        setCopied(text);
        return Promise.resolve();
      },
    };
    Object.defineProperty(navigator, "clipboard", { value: clipboard, configurable: true });
    return () => void delete (navigator as { clipboard?: unknown }).clipboard;
  }, []);
  return copied;
};

// --- the frame ---

// The frame's layout--* classes, which the settings styles key on `.layout`.
const useFrameLayoutClasses = (ref: React.RefObject<HTMLElement>): string[] => {
  const [classes, setClasses] = useState<string[]>([]);
  useLayoutEffect(() => {
    const frame = ref.current?.closest("[data-story-root]");
    if (!frame) {
      return;
    }
    const read = () =>
      setClasses(Array.from(frame.classList).filter((c) => c.startsWith("layout--")));
    read();
    const observer = new MutationObserver(read);
    observer.observe(frame, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);
  return classes;
};

const STORY_PATH = "/settings/story";

export interface SettingsStoryProps {
  /** Where the story starts, e.g. "/settings/networks/1" (default STORY_PATH with `element`). */
  path?: string;
  /** Services over settingsServices()'s. */
  services?: () => MockServices;
  /** A component of its own at /settings/story, beside the real routes. */
  element?: ReactElement;
  /** Run in order after mount. */
  steps?: Step[];
  /** Below the settings page (the clipboard's line comes after it). */
  children?: ReactNode;
}

// The networks, bootstrap, profile and VNIC route tables under the settings
// layout (its nav left out), as MainRouter's createSettingsRoutes mounts them.
export const SettingsStory: React.FC<SettingsStoryProps> = ({
  path,
  services,
  element,
  steps = [],
  children,
}) => {
  const ref = useRef<HTMLDivElement>(null);
  const classes = useFrameLayoutClasses(ref);
  const copied = useClipboard();
  useSteps(ref, steps);

  return (
    <MockApi services={() => ({ ...settingsServices(), ...services?.() })}>
      <MockNotifications>
        <div ref={ref} className={clsx("layout", "settings_story", classes)}>
          <div className="settings_story__page">
            <StoryRouter initialPath={path ?? STORY_PATH}>
              <Route path="/settings" element={<SettingsLayout nav={null} />}>
                {element && <Route path="story" element={element} />}
                <Route path="profile/*" element={<ProfileRoutes />} />
                <Route path="networks/*" element={<NetworkRoutes />} />
                <Route path="bootstrap/*" element={<BootstrapRoutes />} />
                <Route path="vnic/*" element={<VNICPage />} />
              </Route>
            </StoryRouter>
          </div>
          {children}
          {copied !== null && (
            <div className="settings_story__clipboard">
              <span>copied to the clipboard</span>
              <code>{copied}</code>
            </div>
          )}
          <Toast timeoutMs={60000} />
        </div>
      </MockNotifications>
    </MockApi>
  );
};
