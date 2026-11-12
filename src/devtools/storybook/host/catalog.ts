// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { Story, StoryModule, StoryVisual } from "../types";
import { ROOT } from "./protocol";
import { HOST_ALLOW } from "./recorder";

export { ROOT };

const stories = require.context("../../stories/", true, /\.stories\.tsx$/, "lazy");

// Every UI file's path, keys only: "weak" puts none of them in a chunk
// (webpack still compiles them). Not watch/index.tsx, the watch page's entry,
// which mounts the page and exports no component.
const sources = require.context(
  "../../../",
  true,
  /^\.\/(?!watch\/index\.tsx$)(components|pages|watch)\/(?!.*\.(test|stories)\.tsx$).*\.tsx$/,
  "weak"
);

/** Story module keys ("./chat/Chat.stories.tsx"), sorted. */
export const moduleKeys: string[] = stories
  .keys()
  .filter((k) => k.startsWith("./"))
  .sort((a, b) => a.localeCompare(b));

// A module key's path below stories/, its route's: "chat/Chat".
export const modulePath = (key: string): string => key.replace(/^\.\/|\.stories\.tsx$/g, "");

// The source file a module at the mirrored path covers: "components/Chat/Message.tsx".
export const mirroredFile = (key: string): string => `${modulePath(key)}.tsx`;

/** The catalog: every UI file's path below src/, sorted. */
export const catalogFiles: string[] = sources
  .keys()
  .filter((k) => k.startsWith("./"))
  .map((k) => k.substring(2))
  .sort((a, b) => a.localeCompare(b));

const catalogSet = new Set(catalogFiles);

// A story's route: /storybook/<module path>/<name>.
export const storyRoute = (key: string, name: string): string =>
  `${ROOT}/${modulePath(key)}/${encodeURIComponent(name)}`;

const loaded = new Map<string, Promise<StoryModule>>();

// A story module, loaded once.
export const loadModule = (key: string): Promise<StoryModule> => {
  let mod = loaded.get(key);
  if (!mod) {
    mod = stories(key) as Promise<StoryModule>;
    loaded.set(key, mod);
  }
  return mod;
};

// The module key and story name a path below /storybook names, if any.
export const matchRoute = (pathname: string): { key: string; name?: string } | null => {
  const rest = pathname.startsWith(ROOT + "/") ? pathname.substring(ROOT.length + 1) : "";
  if (!rest) {
    return null;
  }
  // The longest module path the route starts with.
  let best: { key: string; name?: string } = null;
  for (const key of moduleKeys) {
    const path = modulePath(key);
    if (rest === path || rest === path + "/") {
      best = { key };
    } else if (
      rest.startsWith(path + "/") &&
      (!best || path.length > modulePath(best.key).length)
    ) {
      const name = rest.substring(path.length + 1).split("/")[0];
      best = { key, name: decodeURIComponent(name) };
    }
  }
  return best;
};

export type FileStatus = "covered" | "skipped" | "missing";

export interface IndexFile {
  path: string;
  status: FileStatus;
  /** Covered: the covering modules' story routes. */
  stories?: string[];
  /** Skipped: why, and which module says so. */
  reason?: string;
  skippedBy?: string;
}

export interface IndexStory {
  route: string;
  module: string;
  name: string;
  /** The story's allowErrors as /source/flags. */
  allowErrors: string[];
  visual?: StoryVisual;
}

// Build-dependent content that hack/storybook/check.py --compare paints
// over in every story's screenshots, each with why. `text` replaces the
// element's text before it's measured, so the box is the same in every build.
export interface HostMask {
  selector: string;
  text?: string;
  reason: string;
}

export const HOST_MASK: HostMask[] = [
  {
    selector: ".landing_page__header__revision",
    text: "rev. 0000000",
    reason: "the build's git hash",
  },
];

export interface IndexData {
  stories: IndexStory[];
  modules: { module: string; stories: number; error?: string }[];
  files: IndexFile[];
  counts: Record<FileStatus | "total", number>;
  /** covers/skips naming files outside the catalog, and skips of covered files. */
  stale: { module: string; path: string; kind: "covers" | "skips" | "skips-covered" }[];
  hostAllowErrors: { pattern: string; reason: string }[];
  hostMask: HostMask[];
  /** What the build was made from: the commit (GIT_HASH) and the time its
   * webpack config was read (BUILD_TIME, ms since the epoch), so
   * hack/storybook/check.py --compare knows a build that isn't the tree's. */
  build: { commit: string; time: number };
}

// Loads every story module and resolves the catalog against them.
export const buildIndex = async (): Promise<IndexData> => {
  const results = await Promise.allSettled(moduleKeys.map(loadModule));

  const data: IndexData = {
    stories: [],
    modules: [],
    files: [],
    counts: { total: catalogFiles.length, covered: 0, skipped: 0, missing: 0 },
    stale: [],
    hostAllowErrors: HOST_ALLOW.map(({ pattern, reason }) => ({
      pattern: String(pattern),
      reason,
    })),
    hostMask: HOST_MASK,
    build: { commit: GIT_HASH, time: BUILD_TIME },
  };
  const covering = new Map<string, string[]>();
  const skips = new Map<string, { reason: string; module: string }>();

  results.forEach((res, i) => {
    const key = moduleKeys[i];
    const module = modulePath(key);
    if (res.status === "rejected") {
      data.modules.push({ module, stories: 0, error: String(res.reason) });
      return;
    }
    const list: Story[] = Array.isArray(res.value.default) ? res.value.default : [];
    data.modules.push({ module, stories: list.length });
    const routes = list.map((s) => storyRoute(key, s.name));
    for (const [j, s] of list.entries()) {
      data.stories.push({
        route: routes[j],
        module,
        name: s.name,
        allowErrors: (s.allowErrors ?? []).map(String),
        ...(s.visual ? { visual: s.visual } : {}),
      });
    }

    const { covers = [], skips: moduleSkips = {} } = res.value.catalog ?? {};
    const mirrored = mirroredFile(key);
    for (const path of catalogSet.has(mirrored) ? [mirrored, ...covers] : covers) {
      if (!catalogSet.has(path)) {
        data.stale.push({ module, path, kind: "covers" });
      } else if (routes.length) {
        covering.set(path, [...(covering.get(path) ?? []), ...routes]);
      }
    }
    for (const [path, reason] of Object.entries(moduleSkips)) {
      if (!catalogSet.has(path)) {
        data.stale.push({ module, path, kind: "skips" });
      } else {
        skips.set(path, { reason, module });
      }
    }
  });

  for (const path of catalogFiles) {
    const routes = covering.get(path);
    const skip = skips.get(path);
    if (routes) {
      if (skip) {
        data.stale.push({ module: skip.module, path, kind: "skips-covered" });
      }
      data.files.push({ path, status: "covered", stories: routes });
    } else if (skip) {
      data.files.push({ path, status: "skipped", reason: skip.reason, skippedBy: skip.module });
    } else {
      data.files.push({ path, status: "missing" });
    }
  }
  for (const f of data.files) {
    data.counts[f.status]++;
  }
  return data;
};
