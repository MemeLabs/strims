import type { PluginCreator } from "postcss";
/**
 * A PostCSS plugin that has the styles read each env(safe-area-inset-<side>)
 * through a custom property, as
 * var(--safe-area-inset-<side>, env(safe-area-inset-<side>)). The app sets
 * none of them, so a browser reads its own insets. The storybook's
 * landscape phone frame sets them to an iPhone's
 * (src/devtools/storybook/host/devices.ts), since page script can't change
 * a desktop browser's env() values.
 */
export declare const safeAreaInsets: PluginCreator<void>;
