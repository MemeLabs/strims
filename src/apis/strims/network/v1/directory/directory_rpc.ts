import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  strims_network_v1_directory_IPublishRequest,
  strims_network_v1_directory_PublishRequest,
  strims_network_v1_directory_PublishResponse,
  strims_network_v1_directory_IUnpublishRequest,
  strims_network_v1_directory_UnpublishRequest,
  strims_network_v1_directory_UnpublishResponse,
  strims_network_v1_directory_IJoinRequest,
  strims_network_v1_directory_JoinRequest,
  strims_network_v1_directory_JoinResponse,
  strims_network_v1_directory_IPartRequest,
  strims_network_v1_directory_PartRequest,
  strims_network_v1_directory_PartResponse,
  strims_network_v1_directory_IPingRequest,
  strims_network_v1_directory_PingRequest,
  strims_network_v1_directory_PingResponse,
  strims_network_v1_directory_IModerateListingRequest,
  strims_network_v1_directory_ModerateListingRequest,
  strims_network_v1_directory_ModerateListingResponse,
  strims_network_v1_directory_IModerateUserRequest,
  strims_network_v1_directory_ModerateUserRequest,
  strims_network_v1_directory_ModerateUserResponse,
  strims_network_v1_directory_IFrontendPublishRequest,
  strims_network_v1_directory_FrontendPublishRequest,
  strims_network_v1_directory_FrontendPublishResponse,
  strims_network_v1_directory_IFrontendUnpublishRequest,
  strims_network_v1_directory_FrontendUnpublishRequest,
  strims_network_v1_directory_FrontendUnpublishResponse,
  strims_network_v1_directory_IFrontendJoinRequest,
  strims_network_v1_directory_FrontendJoinRequest,
  strims_network_v1_directory_FrontendJoinResponse,
  strims_network_v1_directory_IFrontendPartRequest,
  strims_network_v1_directory_FrontendPartRequest,
  strims_network_v1_directory_FrontendPartResponse,
  strims_network_v1_directory_IFrontendTestRequest,
  strims_network_v1_directory_FrontendTestRequest,
  strims_network_v1_directory_FrontendTestResponse,
  strims_network_v1_directory_IFrontendModerateListingRequest,
  strims_network_v1_directory_FrontendModerateListingRequest,
  strims_network_v1_directory_FrontendModerateListingResponse,
  strims_network_v1_directory_IFrontendModerateUserRequest,
  strims_network_v1_directory_FrontendModerateUserRequest,
  strims_network_v1_directory_FrontendModerateUserResponse,
  strims_network_v1_directory_IFrontendGetUsersRequest,
  strims_network_v1_directory_FrontendGetUsersRequest,
  strims_network_v1_directory_FrontendGetUsersResponse,
  strims_network_v1_directory_IFrontendGetListingsRequest,
  strims_network_v1_directory_FrontendGetListingsRequest,
  strims_network_v1_directory_FrontendGetListingsResponse,
  strims_network_v1_directory_IFrontendWatchListingsRequest,
  strims_network_v1_directory_FrontendWatchListingsRequest,
  strims_network_v1_directory_FrontendWatchListingsResponse,
  strims_network_v1_directory_IFrontendWatchListingUsersRequest,
  strims_network_v1_directory_FrontendWatchListingUsersRequest,
  strims_network_v1_directory_FrontendWatchListingUsersResponse,
  strims_network_v1_directory_ISnippetSubscribeRequest,
  strims_network_v1_directory_SnippetSubscribeRequest,
  strims_network_v1_directory_SnippetSubscribeResponse,
} from "./directory";

export interface DirectoryService {
  publish(req: strims_network_v1_directory_PublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PublishResponse> | strims_network_v1_directory_PublishResponse;
  unpublish(req: strims_network_v1_directory_UnpublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_UnpublishResponse> | strims_network_v1_directory_UnpublishResponse;
  join(req: strims_network_v1_directory_JoinRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_JoinResponse> | strims_network_v1_directory_JoinResponse;
  part(req: strims_network_v1_directory_PartRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PartResponse> | strims_network_v1_directory_PartResponse;
  ping(req: strims_network_v1_directory_PingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PingResponse> | strims_network_v1_directory_PingResponse;
  moderateListing(req: strims_network_v1_directory_ModerateListingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_ModerateListingResponse> | strims_network_v1_directory_ModerateListingResponse;
  moderateUser(req: strims_network_v1_directory_ModerateUserRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_ModerateUserResponse> | strims_network_v1_directory_ModerateUserResponse;
}

export class UnimplementedDirectoryService implements DirectoryService {
  publish(req: strims_network_v1_directory_PublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PublishResponse> | strims_network_v1_directory_PublishResponse { throw new Error("not implemented"); }
  unpublish(req: strims_network_v1_directory_UnpublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_UnpublishResponse> | strims_network_v1_directory_UnpublishResponse { throw new Error("not implemented"); }
  join(req: strims_network_v1_directory_JoinRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_JoinResponse> | strims_network_v1_directory_JoinResponse { throw new Error("not implemented"); }
  part(req: strims_network_v1_directory_PartRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PartResponse> | strims_network_v1_directory_PartResponse { throw new Error("not implemented"); }
  ping(req: strims_network_v1_directory_PingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PingResponse> | strims_network_v1_directory_PingResponse { throw new Error("not implemented"); }
  moderateListing(req: strims_network_v1_directory_ModerateListingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_ModerateListingResponse> | strims_network_v1_directory_ModerateListingResponse { throw new Error("not implemented"); }
  moderateUser(req: strims_network_v1_directory_ModerateUserRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_ModerateUserResponse> | strims_network_v1_directory_ModerateUserResponse { throw new Error("not implemented"); }
}

export const registerDirectoryService = (host: strims_rpc_Service, service: DirectoryService): void => {
  host.registerMethod<strims_network_v1_directory_PublishRequest, strims_network_v1_directory_PublishResponse>("strims.network.v1.directory.Directory.Publish", service.publish.bind(service), strims_network_v1_directory_PublishRequest);
  host.registerMethod<strims_network_v1_directory_UnpublishRequest, strims_network_v1_directory_UnpublishResponse>("strims.network.v1.directory.Directory.Unpublish", service.unpublish.bind(service), strims_network_v1_directory_UnpublishRequest);
  host.registerMethod<strims_network_v1_directory_JoinRequest, strims_network_v1_directory_JoinResponse>("strims.network.v1.directory.Directory.Join", service.join.bind(service), strims_network_v1_directory_JoinRequest);
  host.registerMethod<strims_network_v1_directory_PartRequest, strims_network_v1_directory_PartResponse>("strims.network.v1.directory.Directory.Part", service.part.bind(service), strims_network_v1_directory_PartRequest);
  host.registerMethod<strims_network_v1_directory_PingRequest, strims_network_v1_directory_PingResponse>("strims.network.v1.directory.Directory.Ping", service.ping.bind(service), strims_network_v1_directory_PingRequest);
  host.registerMethod<strims_network_v1_directory_ModerateListingRequest, strims_network_v1_directory_ModerateListingResponse>("strims.network.v1.directory.Directory.ModerateListing", service.moderateListing.bind(service), strims_network_v1_directory_ModerateListingRequest);
  host.registerMethod<strims_network_v1_directory_ModerateUserRequest, strims_network_v1_directory_ModerateUserResponse>("strims.network.v1.directory.Directory.ModerateUser", service.moderateUser.bind(service), strims_network_v1_directory_ModerateUserRequest);
}

export class DirectoryClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public publish(req?: strims_network_v1_directory_IPublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_PublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Publish", new strims_network_v1_directory_PublishRequest(req)), strims_network_v1_directory_PublishResponse, opts);
  }

  public unpublish(req?: strims_network_v1_directory_IUnpublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_UnpublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Unpublish", new strims_network_v1_directory_UnpublishRequest(req)), strims_network_v1_directory_UnpublishResponse, opts);
  }

  public join(req?: strims_network_v1_directory_IJoinRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_JoinResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Join", new strims_network_v1_directory_JoinRequest(req)), strims_network_v1_directory_JoinResponse, opts);
  }

  public part(req?: strims_network_v1_directory_IPartRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_PartResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Part", new strims_network_v1_directory_PartRequest(req)), strims_network_v1_directory_PartResponse, opts);
  }

  public ping(req?: strims_network_v1_directory_IPingRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_PingResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.Ping", new strims_network_v1_directory_PingRequest(req)), strims_network_v1_directory_PingResponse, opts);
  }

  public moderateListing(req?: strims_network_v1_directory_IModerateListingRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_ModerateListingResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.ModerateListing", new strims_network_v1_directory_ModerateListingRequest(req)), strims_network_v1_directory_ModerateListingResponse, opts);
  }

  public moderateUser(req?: strims_network_v1_directory_IModerateUserRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_ModerateUserResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.Directory.ModerateUser", new strims_network_v1_directory_ModerateUserRequest(req)), strims_network_v1_directory_ModerateUserResponse, opts);
  }
}

export interface DirectoryFrontendService {
  publish(req: strims_network_v1_directory_FrontendPublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendPublishResponse> | strims_network_v1_directory_FrontendPublishResponse;
  unpublish(req: strims_network_v1_directory_FrontendUnpublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendUnpublishResponse> | strims_network_v1_directory_FrontendUnpublishResponse;
  join(req: strims_network_v1_directory_FrontendJoinRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendJoinResponse> | strims_network_v1_directory_FrontendJoinResponse;
  part(req: strims_network_v1_directory_FrontendPartRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendPartResponse> | strims_network_v1_directory_FrontendPartResponse;
  test(req: strims_network_v1_directory_FrontendTestRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendTestResponse> | strims_network_v1_directory_FrontendTestResponse;
  moderateListing(req: strims_network_v1_directory_FrontendModerateListingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendModerateListingResponse> | strims_network_v1_directory_FrontendModerateListingResponse;
  moderateUser(req: strims_network_v1_directory_FrontendModerateUserRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendModerateUserResponse> | strims_network_v1_directory_FrontendModerateUserResponse;
  getUsers(req: strims_network_v1_directory_FrontendGetUsersRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetUsersResponse> | strims_network_v1_directory_FrontendGetUsersResponse;
  getListings(req: strims_network_v1_directory_FrontendGetListingsRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetListingsResponse> | strims_network_v1_directory_FrontendGetListingsResponse;
  watchListings(req: strims_network_v1_directory_FrontendWatchListingsRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchListingsResponse>;
  watchListingUsers(req: strims_network_v1_directory_FrontendWatchListingUsersRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchListingUsersResponse>;
}

export class UnimplementedDirectoryFrontendService implements DirectoryFrontendService {
  publish(req: strims_network_v1_directory_FrontendPublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendPublishResponse> | strims_network_v1_directory_FrontendPublishResponse { throw new Error("not implemented"); }
  unpublish(req: strims_network_v1_directory_FrontendUnpublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendUnpublishResponse> | strims_network_v1_directory_FrontendUnpublishResponse { throw new Error("not implemented"); }
  join(req: strims_network_v1_directory_FrontendJoinRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendJoinResponse> | strims_network_v1_directory_FrontendJoinResponse { throw new Error("not implemented"); }
  part(req: strims_network_v1_directory_FrontendPartRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendPartResponse> | strims_network_v1_directory_FrontendPartResponse { throw new Error("not implemented"); }
  test(req: strims_network_v1_directory_FrontendTestRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendTestResponse> | strims_network_v1_directory_FrontendTestResponse { throw new Error("not implemented"); }
  moderateListing(req: strims_network_v1_directory_FrontendModerateListingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendModerateListingResponse> | strims_network_v1_directory_FrontendModerateListingResponse { throw new Error("not implemented"); }
  moderateUser(req: strims_network_v1_directory_FrontendModerateUserRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendModerateUserResponse> | strims_network_v1_directory_FrontendModerateUserResponse { throw new Error("not implemented"); }
  getUsers(req: strims_network_v1_directory_FrontendGetUsersRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetUsersResponse> | strims_network_v1_directory_FrontendGetUsersResponse { throw new Error("not implemented"); }
  getListings(req: strims_network_v1_directory_FrontendGetListingsRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetListingsResponse> | strims_network_v1_directory_FrontendGetListingsResponse { throw new Error("not implemented"); }
  watchListings(req: strims_network_v1_directory_FrontendWatchListingsRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchListingsResponse> { throw new Error("not implemented"); }
  watchListingUsers(req: strims_network_v1_directory_FrontendWatchListingUsersRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchListingUsersResponse> { throw new Error("not implemented"); }
}

export const registerDirectoryFrontendService = (host: strims_rpc_Service, service: DirectoryFrontendService): void => {
  host.registerMethod<strims_network_v1_directory_FrontendPublishRequest, strims_network_v1_directory_FrontendPublishResponse>("strims.network.v1.directory.DirectoryFrontend.Publish", service.publish.bind(service), strims_network_v1_directory_FrontendPublishRequest);
  host.registerMethod<strims_network_v1_directory_FrontendUnpublishRequest, strims_network_v1_directory_FrontendUnpublishResponse>("strims.network.v1.directory.DirectoryFrontend.Unpublish", service.unpublish.bind(service), strims_network_v1_directory_FrontendUnpublishRequest);
  host.registerMethod<strims_network_v1_directory_FrontendJoinRequest, strims_network_v1_directory_FrontendJoinResponse>("strims.network.v1.directory.DirectoryFrontend.Join", service.join.bind(service), strims_network_v1_directory_FrontendJoinRequest);
  host.registerMethod<strims_network_v1_directory_FrontendPartRequest, strims_network_v1_directory_FrontendPartResponse>("strims.network.v1.directory.DirectoryFrontend.Part", service.part.bind(service), strims_network_v1_directory_FrontendPartRequest);
  host.registerMethod<strims_network_v1_directory_FrontendTestRequest, strims_network_v1_directory_FrontendTestResponse>("strims.network.v1.directory.DirectoryFrontend.Test", service.test.bind(service), strims_network_v1_directory_FrontendTestRequest);
  host.registerMethod<strims_network_v1_directory_FrontendModerateListingRequest, strims_network_v1_directory_FrontendModerateListingResponse>("strims.network.v1.directory.DirectoryFrontend.ModerateListing", service.moderateListing.bind(service), strims_network_v1_directory_FrontendModerateListingRequest);
  host.registerMethod<strims_network_v1_directory_FrontendModerateUserRequest, strims_network_v1_directory_FrontendModerateUserResponse>("strims.network.v1.directory.DirectoryFrontend.ModerateUser", service.moderateUser.bind(service), strims_network_v1_directory_FrontendModerateUserRequest);
  host.registerMethod<strims_network_v1_directory_FrontendGetUsersRequest, strims_network_v1_directory_FrontendGetUsersResponse>("strims.network.v1.directory.DirectoryFrontend.GetUsers", service.getUsers.bind(service), strims_network_v1_directory_FrontendGetUsersRequest);
  host.registerMethod<strims_network_v1_directory_FrontendGetListingsRequest, strims_network_v1_directory_FrontendGetListingsResponse>("strims.network.v1.directory.DirectoryFrontend.GetListings", service.getListings.bind(service), strims_network_v1_directory_FrontendGetListingsRequest);
  host.registerMethod<strims_network_v1_directory_FrontendWatchListingsRequest, strims_network_v1_directory_FrontendWatchListingsResponse>("strims.network.v1.directory.DirectoryFrontend.WatchListings", service.watchListings.bind(service), strims_network_v1_directory_FrontendWatchListingsRequest);
  host.registerMethod<strims_network_v1_directory_FrontendWatchListingUsersRequest, strims_network_v1_directory_FrontendWatchListingUsersResponse>("strims.network.v1.directory.DirectoryFrontend.WatchListingUsers", service.watchListingUsers.bind(service), strims_network_v1_directory_FrontendWatchListingUsersRequest);
}

export class DirectoryFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public publish(req?: strims_network_v1_directory_IFrontendPublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendPublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.Publish", new strims_network_v1_directory_FrontendPublishRequest(req)), strims_network_v1_directory_FrontendPublishResponse, opts);
  }

  public unpublish(req?: strims_network_v1_directory_IFrontendUnpublishRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendUnpublishResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.Unpublish", new strims_network_v1_directory_FrontendUnpublishRequest(req)), strims_network_v1_directory_FrontendUnpublishResponse, opts);
  }

  public join(req?: strims_network_v1_directory_IFrontendJoinRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendJoinResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.Join", new strims_network_v1_directory_FrontendJoinRequest(req)), strims_network_v1_directory_FrontendJoinResponse, opts);
  }

  public part(req?: strims_network_v1_directory_IFrontendPartRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendPartResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.Part", new strims_network_v1_directory_FrontendPartRequest(req)), strims_network_v1_directory_FrontendPartResponse, opts);
  }

  public test(req?: strims_network_v1_directory_IFrontendTestRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendTestResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.Test", new strims_network_v1_directory_FrontendTestRequest(req)), strims_network_v1_directory_FrontendTestResponse, opts);
  }

  public moderateListing(req?: strims_network_v1_directory_IFrontendModerateListingRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendModerateListingResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.ModerateListing", new strims_network_v1_directory_FrontendModerateListingRequest(req)), strims_network_v1_directory_FrontendModerateListingResponse, opts);
  }

  public moderateUser(req?: strims_network_v1_directory_IFrontendModerateUserRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendModerateUserResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.ModerateUser", new strims_network_v1_directory_FrontendModerateUserRequest(req)), strims_network_v1_directory_FrontendModerateUserResponse, opts);
  }

  public getUsers(req?: strims_network_v1_directory_IFrontendGetUsersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendGetUsersResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.GetUsers", new strims_network_v1_directory_FrontendGetUsersRequest(req)), strims_network_v1_directory_FrontendGetUsersResponse, opts);
  }

  public getListings(req?: strims_network_v1_directory_IFrontendGetListingsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendGetListingsResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.GetListings", new strims_network_v1_directory_FrontendGetListingsRequest(req)), strims_network_v1_directory_FrontendGetListingsResponse, opts);
  }

  public watchListings(req?: strims_network_v1_directory_IFrontendWatchListingsRequest): GenericReadable<strims_network_v1_directory_FrontendWatchListingsResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.directory.DirectoryFrontend.WatchListings", new strims_network_v1_directory_FrontendWatchListingsRequest(req)), strims_network_v1_directory_FrontendWatchListingsResponse);
  }

  public watchListingUsers(req?: strims_network_v1_directory_IFrontendWatchListingUsersRequest): GenericReadable<strims_network_v1_directory_FrontendWatchListingUsersResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.directory.DirectoryFrontend.WatchListingUsers", new strims_network_v1_directory_FrontendWatchListingUsersRequest(req)), strims_network_v1_directory_FrontendWatchListingUsersResponse);
  }
}

export interface DirectorySnippetService {
  subscribe(req: strims_network_v1_directory_SnippetSubscribeRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_SnippetSubscribeResponse>;
}

export class UnimplementedDirectorySnippetService implements DirectorySnippetService {
  subscribe(req: strims_network_v1_directory_SnippetSubscribeRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_SnippetSubscribeResponse> { throw new Error("not implemented"); }
}

export const registerDirectorySnippetService = (host: strims_rpc_Service, service: DirectorySnippetService): void => {
  host.registerMethod<strims_network_v1_directory_SnippetSubscribeRequest, strims_network_v1_directory_SnippetSubscribeResponse>("strims.network.v1.directory.DirectorySnippet.Subscribe", service.subscribe.bind(service), strims_network_v1_directory_SnippetSubscribeRequest);
}

export class DirectorySnippetClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public subscribe(req?: strims_network_v1_directory_ISnippetSubscribeRequest): GenericReadable<strims_network_v1_directory_SnippetSubscribeResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.directory.DirectorySnippet.Subscribe", new strims_network_v1_directory_SnippetSubscribeRequest(req)), strims_network_v1_directory_SnippetSubscribeResponse);
  }
}

