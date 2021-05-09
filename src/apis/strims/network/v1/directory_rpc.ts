import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
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

export class DirectoryClient {
  constructor(private readonly host: RPCHost) {}

  public publish(arg?: IDirectoryPublishRequest): Promise<DirectoryPublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Publish", new DirectoryPublishRequest(arg)));
  }

  public unpublish(arg?: IDirectoryUnpublishRequest): Promise<DirectoryUnpublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Unpublish", new DirectoryUnpublishRequest(arg)));
  }

  public join(arg?: IDirectoryJoinRequest): Promise<DirectoryJoinResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Join", new DirectoryJoinRequest(arg)));
  }

  public part(arg?: IDirectoryPartRequest): Promise<DirectoryPartResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Part", new DirectoryPartRequest(arg)));
  }

  public ping(arg?: IDirectoryPingRequest): Promise<DirectoryPingResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.Directory.Ping", new DirectoryPingRequest(arg)));
  }
}

export class DirectoryFrontendClient {
  constructor(private readonly host: RPCHost) {}

  public open(arg?: IDirectoryFrontendOpenRequest): GenericReadable<DirectoryFrontendOpenResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.DirectoryFrontend.Open", new DirectoryFrontendOpenRequest(arg)));
  }

  public test(arg?: IDirectoryFrontendTestRequest): Promise<DirectoryFrontendTestResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.DirectoryFrontend.Test", new DirectoryFrontendTestRequest(arg)));
  }
}

