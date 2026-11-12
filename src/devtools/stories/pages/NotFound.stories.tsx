// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import NotFound from "../../../pages/NotFound";
import { Catalog, Story } from "../../storybook";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <NotFound />,
  },
];

export default stories;

export const catalog: Catalog = {
  skips: {
    "pages/Categories.tsx": "a placeholder page: an empty header and section, nothing shows",
    "pages/Streams.tsx": "a placeholder page: an empty header and section, nothing shows",
  },
};
