// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as CSS from "csstype";

declare module "csstype" {
  interface Properties {
    aspectRatio?: string | number;

    "--layout-width"?: string | number;
    "--layout-height"?: string | number;
    "--layout-offset"?: string | number;

    "--tooltip-anchor-x"?: string | number;
    "--tooltip-anchor-y"?: string | number;
    "--tooltip-anchor-width"?: string | number;
    "--tooltip-anchor-height"?: string | number;

    "--chat-width"?: string | number;
    "--chat-height"?: string | number;

    "--video-height"?: string | number;

    "--fill-color"?: string;

    "--context-menu-x"?: string | number;
    "--context-menu-y"?: string | number;

    "--menu-x"?: string;
    "--menu-y"?: string;
    "--menu-scale"?: string;
  }
}
