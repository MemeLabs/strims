// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// What stories build on: the story module types, mock API clients, a router
// of the story's own, static context providers and shared fixtures.
//
// A story module, stories/<path under src>.stories.tsx (it covers
// src/<path>.tsx), default-exports Story[] and may export `catalog`:
//
//   import { Catalog, Story } from "../../../storybook";
//   const stories: Story[] = [{ name: "Default", component: () => <X /> }];
//   export default stories;
//   export const catalog: Catalog = { skips: { "components/X/routes.tsx": "route table" } };

export * from "./types";
export { useStoryHost } from "./context";
export { DesktopOnly } from "./DesktopOnly";
export type { DesktopOnlyProps } from "./DesktopOnly";
export { MockApi, mockClient, useMockService, useMockServices } from "./MockApi";
export type { MockApiProps, MockServices, ServiceTypes } from "./MockApi";
export { StoryRouter } from "./StoryRouter";
export type { StoryRouterProps } from "./StoryRouter";
export { MockNotifications, SessionProvider, StaticNetwork, StaticPlayer } from "./providers";
export type { MockNotificationsProps, StaticNetworkProps, StaticPlayerProps } from "./providers";
export * as fixtures from "./fixtures";
