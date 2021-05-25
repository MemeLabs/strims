import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  IDirectoryPublishRequest,
  DirectoryPublishRequest,
  DirectoryPublishResponse,
  IDirectoryUnpublishRequest,
  DirectoryUnpublishRequest,
  DirectoryUnpublishResponse,
  IDirectoryJoinRequest,
  DirectoryJoinRequest,
  DirectoryJoinResponse,
  IDirectoryPartRequest,
  DirectoryPartRequest,
  DirectoryPartResponse,
  IDirectoryPingRequest,
  DirectoryPingRequest,
  DirectoryPingResponse,
  IDirectoryFrontendOpenRequest,
  DirectoryFrontendOpenRequest,
  DirectoryFrontendOpenResponse,
  IDirectoryFrontendTestRequest,
  DirectoryFrontendTestRequest,
  DirectoryFrontendTestResponse,
} from "./directory";

registerType("strims.network.v1.DirectoryPublishRequest", DirectoryPublishRequest);
registerType("strims.network.v1.DirectoryPublishResponse", DirectoryPublishResponse);
registerType("strims.network.v1.DirectoryUnpublishRequest", DirectoryUnpublishRequest);
registerType("strims.network.v1.DirectoryUnpublishResponse", DirectoryUnpublishResponse);
registerType("strims.network.v1.DirectoryJoinRequest", DirectoryJoinRequest);
registerType("strims.network.v1.DirectoryJoinResponse", DirectoryJoinResponse);
registerType("strims.network.v1.DirectoryPartRequest", DirectoryPartRequest);
registerType("strims.network.v1.DirectoryPartResponse", DirectoryPartResponse);
registerType("strims.network.v1.DirectoryPingRequest", DirectoryPingRequest);
registerType("strims.network.v1.DirectoryPingResponse", DirectoryPingResponse);
registerType("strims.network.v1.DirectoryFrontendOpenRequest", DirectoryFrontendOpenRequest);
registerType("strims.network.v1.DirectoryFrontendOpenResponse", DirectoryFrontendOpenResponse);
registerType("strims.network.v1.DirectoryFrontendTestRequest", DirectoryFrontendTestRequest);
registerType("strims.network.v1.DirectoryFrontendTestResponse", DirectoryFrontendTestResponse);

export interface DirectoryService {
  publish(req: DirectoryPublishRequest, call: strims_rpc_Call): Promise<DirectoryPublishResponse> | DirectoryPublishResponse;
  unpublish(req: DirectoryUnpublishRequest, call: strims_rpc_Call): Promise<DirectoryUnpublishResponse> | DirectoryUnpublishResponse;
  join(req: DirectoryJoinRequest, call: strims_rpc_Call): Promise<DirectoryJoinResponse> | DirectoryJoinResponse;
  part(req: DirectoryPartRequest, call: strims_rpc_Call): Promise<DirectoryPartResponse> | DirectoryPartResponse;
  ping(req: DirectoryPingRequest, call: strims_rpc_Call): Promise<DirectoryPingResponse> | DirectoryPingResponse;
}

export const registerDirectoryService = (host: strims_rpc_Service, service: DirectoryService): void => {
  host.registerMethod<DirectoryPublishRequest, DirectoryPublishResponse>("strims.network.v1.Directory.Publish", service.publish.bind(service));
  host.registerMethod<DirectoryUnpublishRequest, DirectoryUnpublishResponse>("strims.network.v1.Directory.Unpublish", service.unpublish.bind(service));
  host.registerMethod<DirectoryJoinRequest, DirectoryJoinResponse>("strims.network.v1.Directory.Join", service.join.bind(service));
  host.registerMethod<DirectoryPartRequest, DirectoryPartResponse>("strims.network.v1.Directory.Part", service.part.bind(service));
  host.registerMethod<DirectoryPingRequest, DirectoryPingResponse>("strims.network.v1.Directory.Ping", service.ping.bind(service));
}

export class DirectoryClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public publish(req?: IDirectoryPublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DirectoryPublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Publish", new DirectoryPublishRequest(req)), opts);
  }

  public unpublish(req?: IDirectoryUnpublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DirectoryUnpublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Unpublish", new DirectoryUnpublishRequest(req)), opts);
  }

  public join(req?: IDirectoryJoinRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DirectoryJoinResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Join", new DirectoryJoinRequest(req)), opts);
  }

  public part(req?: IDirectoryPartRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DirectoryPartResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Part", new DirectoryPartRequest(req)), opts);
  }

  public ping(req?: IDirectoryPingRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DirectoryPingResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Ping", new DirectoryPingRequest(req)), opts);
  }
}

export interface DirectoryFrontendService {
  open(req: DirectoryFrontendOpenRequest, call: strims_rpc_Call): GenericReadable<DirectoryFrontendOpenResponse>;
  test(req: DirectoryFrontendTestRequest, call: strims_rpc_Call): Promise<DirectoryFrontendTestResponse> | DirectoryFrontendTestResponse;
}

export const registerDirectoryFrontendService = (host: strims_rpc_Service, service: DirectoryFrontendService): void => {
  host.registerMethod<DirectoryFrontendOpenRequest, DirectoryFrontendOpenResponse>("strims.network.v1.DirectoryFrontend.Open", service.open.bind(service));
  host.registerMethod<DirectoryFrontendTestRequest, DirectoryFrontendTestResponse>("strims.network.v1.DirectoryFrontend.Test", service.test.bind(service));
}

export class DirectoryFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public open(req?: IDirectoryFrontendOpenRequest): GenericReadable<DirectoryFrontendOpenResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.DirectoryFrontend.Open", new DirectoryFrontendOpenRequest(req)));
  }

  public test(req?: IDirectoryFrontendTestRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DirectoryFrontendTestResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.DirectoryFrontend.Test", new DirectoryFrontendTestRequest(req)), opts);
  }
}

