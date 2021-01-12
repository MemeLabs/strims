import { WorkerAPI } from "../apis/client";

declare const init: (
  baseURI: string
) => Promise<{
  init(service: "default" | "broker", api: WorkerAPI, ...any): Promise<any>;
}>;

export default init;
