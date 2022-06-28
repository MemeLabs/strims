import strims_rpc_Host from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  strims_video_v1_IEgressOpenStreamRequest,
  strims_video_v1_EgressOpenStreamRequest,
  strims_video_v1_EgressOpenStreamResponse,
} from "./egress";

export interface EgressService {
  openStream(req: strims_video_v1_EgressOpenStreamRequest, call: strims_rpc_Call): GenericReadable<strims_video_v1_EgressOpenStreamResponse>;
}

export class UnimplementedEgressService implements EgressService {
  openStream(req: strims_video_v1_EgressOpenStreamRequest, call: strims_rpc_Call): GenericReadable<strims_video_v1_EgressOpenStreamResponse> { throw new Error("not implemented"); }
}

export const registerEgressService = (host: strims_rpc_Service, service: EgressService): void => {
  host.registerMethod<strims_video_v1_EgressOpenStreamRequest, strims_video_v1_EgressOpenStreamResponse>("strims.video.v1.Egress.OpenStream", service.openStream.bind(service), strims_video_v1_EgressOpenStreamRequest);
}

export class EgressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public openStream(req?: strims_video_v1_IEgressOpenStreamRequest): GenericReadable<strims_video_v1_EgressOpenStreamResponse> {
    return this.host.expectMany(this.host.call("strims.video.v1.Egress.OpenStream", new strims_video_v1_EgressOpenStreamRequest(req)), strims_video_v1_EgressOpenStreamResponse);
  }
}

