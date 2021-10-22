import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  IPublishRequest,
  PublishRequest,
  PublishResponse,
  IUnpublishRequest,
  UnpublishRequest,
  UnpublishResponse,
  IJoinRequest,
  JoinRequest,
  JoinResponse,
  IPartRequest,
  PartRequest,
  PartResponse,
  IPingRequest,
  PingRequest,
  PingResponse,
  IFrontendOpenRequest,
  FrontendOpenRequest,
  FrontendOpenResponse,
  IFrontendTestRequest,
  FrontendTestRequest,
  FrontendTestResponse,
  ISnippetSubscribeRequest,
  SnippetSubscribeRequest,
  SnippetSubscribeResponse,
} from "./directory";

registerType("strims.network.v1.directory.PublishRequest", PublishRequest);
registerType("strims.network.v1.directory.PublishResponse", PublishResponse);
registerType("strims.network.v1.directory.UnpublishRequest", UnpublishRequest);
registerType("strims.network.v1.directory.UnpublishResponse", UnpublishResponse);
registerType("strims.network.v1.directory.JoinRequest", JoinRequest);
registerType("strims.network.v1.directory.JoinResponse", JoinResponse);
registerType("strims.network.v1.directory.PartRequest", PartRequest);
registerType("strims.network.v1.directory.PartResponse", PartResponse);
registerType("strims.network.v1.directory.PingRequest", PingRequest);
registerType("strims.network.v1.directory.PingResponse", PingResponse);
registerType("strims.network.v1.directory.FrontendOpenRequest", FrontendOpenRequest);
registerType("strims.network.v1.directory.FrontendOpenResponse", FrontendOpenResponse);
registerType("strims.network.v1.directory.FrontendTestRequest", FrontendTestRequest);
registerType("strims.network.v1.directory.FrontendTestResponse", FrontendTestResponse);
registerType("strims.network.v1.directory.SnippetSubscribeRequest", SnippetSubscribeRequest);
registerType("strims.network.v1.directory.SnippetSubscribeResponse", SnippetSubscribeResponse);

export interface DirectoryService {
  publish(req: PublishRequest, call: strims_rpc_Call): Promise<PublishResponse> | PublishResponse;
  unpublish(req: UnpublishRequest, call: strims_rpc_Call): Promise<UnpublishResponse> | UnpublishResponse;
  join(req: JoinRequest, call: strims_rpc_Call): Promise<JoinResponse> | JoinResponse;
  part(req: PartRequest, call: strims_rpc_Call): Promise<PartResponse> | PartResponse;
  ping(req: PingRequest, call: strims_rpc_Call): Promise<PingResponse> | PingResponse;
}

export const registerDirectoryService = (host: strims_rpc_Service, service: DirectoryService): void => {
  host.registerMethod<PublishRequest, PublishResponse>("strims.network.v1.directory.Directory.Publish", service.publish.bind(service));
  host.registerMethod<UnpublishRequest, UnpublishResponse>("strims.network.v1.directory.Directory.Unpublish", service.unpublish.bind(service));
  host.registerMethod<JoinRequest, JoinResponse>("strims.network.v1.directory.Directory.Join", service.join.bind(service));
  host.registerMethod<PartRequest, PartResponse>("strims.network.v1.directory.Directory.Part", service.part.bind(service));
  host.registerMethod<PingRequest, PingResponse>("strims.network.v1.directory.Directory.Ping", service.ping.bind(service));
}

export class DirectoryClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public publish(req?: IPublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<PublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Publish", new PublishRequest(req)), opts);
  }

  public unpublish(req?: IUnpublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UnpublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Unpublish", new UnpublishRequest(req)), opts);
  }

  public join(req?: IJoinRequest, opts?: strims_rpc_UnaryCallOptions): Promise<JoinResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Join", new JoinRequest(req)), opts);
  }

  public part(req?: IPartRequest, opts?: strims_rpc_UnaryCallOptions): Promise<PartResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Part", new PartRequest(req)), opts);
  }

  public ping(req?: IPingRequest, opts?: strims_rpc_UnaryCallOptions): Promise<PingResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Ping", new PingRequest(req)), opts);
  }
}

export interface DirectoryFrontendService {
  open(req: FrontendOpenRequest, call: strims_rpc_Call): GenericReadable<FrontendOpenResponse>;
  test(req: FrontendTestRequest, call: strims_rpc_Call): Promise<FrontendTestResponse> | FrontendTestResponse;
}

export const registerDirectoryFrontendService = (host: strims_rpc_Service, service: DirectoryFrontendService): void => {
  host.registerMethod<FrontendOpenRequest, FrontendOpenResponse>("strims.network.v1.directory.DirectoryFrontend.Open", service.open.bind(service));
  host.registerMethod<FrontendTestRequest, FrontendTestResponse>("strims.network.v1.directory.DirectoryFrontend.Test", service.test.bind(service));
}

export class DirectoryFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public open(req?: IFrontendOpenRequest): GenericReadable<FrontendOpenResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.directory.DirectoryFrontend.Open", new FrontendOpenRequest(req)));
  }

  public test(req?: IFrontendTestRequest, opts?: strims_rpc_UnaryCallOptions): Promise<FrontendTestResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.Test", new FrontendTestRequest(req)), opts);
  }
}

export interface DirectorySnippetService {
  subscribe(req: SnippetSubscribeRequest, call: strims_rpc_Call): GenericReadable<SnippetSubscribeResponse>;
}

export const registerDirectorySnippetService = (host: strims_rpc_Service, service: DirectorySnippetService): void => {
  host.registerMethod<SnippetSubscribeRequest, SnippetSubscribeResponse>("strims.network.v1.directory.DirectorySnippet.Subscribe", service.subscribe.bind(service));
}

export class DirectorySnippetClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public subscribe(req?: ISnippetSubscribeRequest): GenericReadable<SnippetSubscribeResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.directory.DirectorySnippet.Subscribe", new SnippetSubscribeRequest(req)));
  }
}
