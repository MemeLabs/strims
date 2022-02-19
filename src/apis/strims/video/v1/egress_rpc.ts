import strims_rpc_Host from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  IEgressOpenStreamRequest,
  EgressOpenStreamRequest,
  EgressOpenStreamResponse,
} from "./egress";

export interface EgressService {
  openStream(req: EgressOpenStreamRequest, call: strims_rpc_Call): GenericReadable<EgressOpenStreamResponse>;
}

export class UnimplementedEgressService implements EgressService {
  openStream(req: EgressOpenStreamRequest, call: strims_rpc_Call): GenericReadable<EgressOpenStreamResponse> { throw new Error("not implemented"); }
}

export const registerEgressService = (host: strims_rpc_Service, service: EgressService): void => {
  host.registerMethod<EgressOpenStreamRequest, EgressOpenStreamResponse>("strims.video.v1.Egress.OpenStream", service.openStream.bind(service), EgressOpenStreamRequest);
}

export class EgressClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public openStream(req?: IEgressOpenStreamRequest): GenericReadable<EgressOpenStreamResponse> {
    return this.host.expectMany(this.host.call("strims.video.v1.Egress.OpenStream", new EgressOpenStreamRequest(req)), EgressOpenStreamResponse);
  }
}

