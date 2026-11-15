// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import type { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  type IEgressOpenStreamRequest as strims_video_v1_IEgressOpenStreamRequest,
  EgressOpenStreamRequest as strims_video_v1_EgressOpenStreamRequest,
  EgressOpenStreamResponse as strims_video_v1_EgressOpenStreamResponse,
} from "./egress.pb";

export interface EgressService {
  openStream(req: strims_video_v1_EgressOpenStreamRequest, call: strims_rpc_Call): GenericReadable<strims_video_v1_EgressOpenStreamResponse>;
}

export class UnimplementedEgressService implements EgressService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  openStream(req: strims_video_v1_EgressOpenStreamRequest, call: strims_rpc_Call): GenericReadable<strims_video_v1_EgressOpenStreamResponse> { throw new Error("not implemented"); }
}

export const registerEgressService = (host: strims_rpc_Service, service: EgressService): void => {
  host.registerMethod<strims_video_v1_EgressOpenStreamRequest, strims_video_v1_EgressOpenStreamResponse>("strims.video.v1.Egress.OpenStream", (req, call) => service.openStream(req, call), strims_video_v1_EgressOpenStreamRequest);
}

export class EgressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public openStream(req?: strims_video_v1_IEgressOpenStreamRequest): GenericReadable<strims_video_v1_EgressOpenStreamResponse> {
    return this.host.expectMany(this.host.call("strims.video.v1.Egress.OpenStream", new strims_video_v1_EgressOpenStreamRequest(req)), strims_video_v1_EgressOpenStreamResponse);
  }
}

