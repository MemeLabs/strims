import * as CSS from "csstype";

declare module "csstype" {
  interface Properties {
    aspectRatio?: string | number;
    "--offset"?: string | number;
  }
}
