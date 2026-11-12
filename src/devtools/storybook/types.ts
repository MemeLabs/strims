// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type React from "react";

/** One story: a component the storybook mounts at /storybook/<module>/<name>. */
export interface Story {
  /** The route's last segment; keep it URL-safe. */
  name: string;
  component: React.ComponentType;
  /**
   * Errors this story expects. Each is tested against an error's whole text
   * (message, then stack); any recorded error none of these (nor the host's
   * allow list) matches fails the story.
   */
  allowErrors?: RegExp[];
  /**
   * What hack/storybook/check.py --compare leaves out of this story's
   * screenshots, for content no capture setting holds still. Use it
   * sparingly, and say why.
   */
  visual?: StoryVisual;
}

export interface StoryVisual {
  /** What in the story varies between captures of the same build. */
  reason: string;
  /** Selectors whose boxes both screenshots paint over. */
  mask?: string[];
  /** Not compared at all. */
  skip?: boolean;
}

/**
 * What a story module says about the catalog of UI files (paths relative to
 * src/, e.g. "components/Chat/Message.tsx"). A module at
 * stories/<path>.stories.tsx covers src/<path>.tsx already.
 */
export interface Catalog {
  /** More files this module's stories cover. */
  covers?: string[];
  /** Files no story is wanted for (route tables, boot code, hooks that render nothing), with why. */
  skips?: Record<string, string>;
}

/** A *.stories.tsx module's exports. */
export interface StoryModule {
  default: Story[];
  catalog?: Catalog;
}
