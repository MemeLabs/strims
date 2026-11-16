// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { PluginCreator } from "postcss";

const INSET = /env\(\s*safe-area-inset-(top|right|bottom|left)\s*\)/g;

/**
 * A PostCSS plugin that has the styles read each env(safe-area-inset-<side>)
 * through a custom property, as
 * var(--safe-area-inset-<side>, env(safe-area-inset-<side>)). The app sets
 * none of them, so a browser reads its own insets. The storybook's
 * landscape phone frame sets them to an iPhone's
 * (src/devtools/storybook/host/devices.ts), since page script can't change
 * a desktop browser's env() values.
 */
export const safeAreaInsets: PluginCreator<void> = () => ({
  postcssPlugin: "safe-area-insets",
  // This runs once, at the end, because a Declaration listener would revisit
  // each value it changes, and the new value still holds the env().
  OnceExit(root) {
    root.walkDecls((decl) => {
      if (decl.value.includes("safe-area-inset-")) {
        decl.value = decl.value.replace(
          INSET,
          "var(--safe-area-inset-$1, env(safe-area-inset-$1))"
        );
      }
    });
  },
});
safeAreaInsets.postcss = true;
