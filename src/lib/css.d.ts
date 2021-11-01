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
  }
}
