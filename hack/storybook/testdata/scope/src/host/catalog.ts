// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The two contexts the real catalog has: every stories module, lazily,
// and every component file's path, weakly.
const stories = require.context("../stories", true, /\.stories\.tsx$/, "lazy");
const sources = require.context("../components", true, /\.tsx$/, "weak");

export const components: string[] = sources.keys();

export const loadStory = (route: string) =>
  stories(`./${route}.stories.tsx`).then((m: { default: () => JSX.Element }) => m.default);
