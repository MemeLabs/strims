// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import type { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  type IPublishRequest as strims_network_v1_directory_IPublishRequest,
  PublishRequest as strims_network_v1_directory_PublishRequest,
  PublishResponse as strims_network_v1_directory_PublishResponse,
  type IUnpublishRequest as strims_network_v1_directory_IUnpublishRequest,
  UnpublishRequest as strims_network_v1_directory_UnpublishRequest,
  UnpublishResponse as strims_network_v1_directory_UnpublishResponse,
  type IJoinRequest as strims_network_v1_directory_IJoinRequest,
  JoinRequest as strims_network_v1_directory_JoinRequest,
  JoinResponse as strims_network_v1_directory_JoinResponse,
  type IPartRequest as strims_network_v1_directory_IPartRequest,
  PartRequest as strims_network_v1_directory_PartRequest,
  PartResponse as strims_network_v1_directory_PartResponse,
  type IPingRequest as strims_network_v1_directory_IPingRequest,
  PingRequest as strims_network_v1_directory_PingRequest,
  PingResponse as strims_network_v1_directory_PingResponse,
  type IModerateListingRequest as strims_network_v1_directory_IModerateListingRequest,
  ModerateListingRequest as strims_network_v1_directory_ModerateListingRequest,
  ModerateListingResponse as strims_network_v1_directory_ModerateListingResponse,
  type IModerateUserRequest as strims_network_v1_directory_IModerateUserRequest,
  ModerateUserRequest as strims_network_v1_directory_ModerateUserRequest,
  ModerateUserResponse as strims_network_v1_directory_ModerateUserResponse,
  type IFrontendPublishRequest as strims_network_v1_directory_IFrontendPublishRequest,
  FrontendPublishRequest as strims_network_v1_directory_FrontendPublishRequest,
  FrontendPublishResponse as strims_network_v1_directory_FrontendPublishResponse,
  type IFrontendUnpublishRequest as strims_network_v1_directory_IFrontendUnpublishRequest,
  FrontendUnpublishRequest as strims_network_v1_directory_FrontendUnpublishRequest,
  FrontendUnpublishResponse as strims_network_v1_directory_FrontendUnpublishResponse,
  type IFrontendJoinRequest as strims_network_v1_directory_IFrontendJoinRequest,
  FrontendJoinRequest as strims_network_v1_directory_FrontendJoinRequest,
  FrontendJoinResponse as strims_network_v1_directory_FrontendJoinResponse,
  type IFrontendPartRequest as strims_network_v1_directory_IFrontendPartRequest,
  FrontendPartRequest as strims_network_v1_directory_FrontendPartRequest,
  FrontendPartResponse as strims_network_v1_directory_FrontendPartResponse,
  type IFrontendTestRequest as strims_network_v1_directory_IFrontendTestRequest,
  FrontendTestRequest as strims_network_v1_directory_FrontendTestRequest,
  FrontendTestResponse as strims_network_v1_directory_FrontendTestResponse,
  type IFrontendModerateListingRequest as strims_network_v1_directory_IFrontendModerateListingRequest,
  FrontendModerateListingRequest as strims_network_v1_directory_FrontendModerateListingRequest,
  FrontendModerateListingResponse as strims_network_v1_directory_FrontendModerateListingResponse,
  type IFrontendModerateUserRequest as strims_network_v1_directory_IFrontendModerateUserRequest,
  FrontendModerateUserRequest as strims_network_v1_directory_FrontendModerateUserRequest,
  FrontendModerateUserResponse as strims_network_v1_directory_FrontendModerateUserResponse,
  type IFrontendGetUsersRequest as strims_network_v1_directory_IFrontendGetUsersRequest,
  FrontendGetUsersRequest as strims_network_v1_directory_FrontendGetUsersRequest,
  FrontendGetUsersResponse as strims_network_v1_directory_FrontendGetUsersResponse,
  type IFrontendGetListingRequest as strims_network_v1_directory_IFrontendGetListingRequest,
  FrontendGetListingRequest as strims_network_v1_directory_FrontendGetListingRequest,
  FrontendGetListingResponse as strims_network_v1_directory_FrontendGetListingResponse,
  type IFrontendGetListingsRequest as strims_network_v1_directory_IFrontendGetListingsRequest,
  FrontendGetListingsRequest as strims_network_v1_directory_FrontendGetListingsRequest,
  FrontendGetListingsResponse as strims_network_v1_directory_FrontendGetListingsResponse,
  type IFrontendWatchListingsRequest as strims_network_v1_directory_IFrontendWatchListingsRequest,
  FrontendWatchListingsRequest as strims_network_v1_directory_FrontendWatchListingsRequest,
  FrontendWatchListingsResponse as strims_network_v1_directory_FrontendWatchListingsResponse,
  type IFrontendWatchListingUsersRequest as strims_network_v1_directory_IFrontendWatchListingUsersRequest,
  FrontendWatchListingUsersRequest as strims_network_v1_directory_FrontendWatchListingUsersRequest,
  FrontendWatchListingUsersResponse as strims_network_v1_directory_FrontendWatchListingUsersResponse,
  type IFrontendWatchAssetBundlesRequest as strims_network_v1_directory_IFrontendWatchAssetBundlesRequest,
  FrontendWatchAssetBundlesRequest as strims_network_v1_directory_FrontendWatchAssetBundlesRequest,
  FrontendWatchAssetBundlesResponse as strims_network_v1_directory_FrontendWatchAssetBundlesResponse,
  type ISnippetSubscribeRequest as strims_network_v1_directory_ISnippetSubscribeRequest,
  SnippetSubscribeRequest as strims_network_v1_directory_SnippetSubscribeRequest,
  SnippetSubscribeResponse as strims_network_v1_directory_SnippetSubscribeResponse,
} from "./directory.pb";

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
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  publish(req: strims_network_v1_directory_PublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PublishResponse> | strims_network_v1_directory_PublishResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  unpublish(req: strims_network_v1_directory_UnpublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_UnpublishResponse> | strims_network_v1_directory_UnpublishResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  join(req: strims_network_v1_directory_JoinRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_JoinResponse> | strims_network_v1_directory_JoinResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  part(req: strims_network_v1_directory_PartRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PartResponse> | strims_network_v1_directory_PartResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  ping(req: strims_network_v1_directory_PingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_PingResponse> | strims_network_v1_directory_PingResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  moderateListing(req: strims_network_v1_directory_ModerateListingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_ModerateListingResponse> | strims_network_v1_directory_ModerateListingResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  moderateUser(req: strims_network_v1_directory_ModerateUserRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_ModerateUserResponse> | strims_network_v1_directory_ModerateUserResponse { throw new Error("not implemented"); }
}

export const registerDirectoryService = (host: strims_rpc_Service, service: DirectoryService): void => {
  host.registerMethod<strims_network_v1_directory_PublishRequest, strims_network_v1_directory_PublishResponse>("strims.network.v1.directory.Directory.Publish", (req, call) => service.publish(req, call), strims_network_v1_directory_PublishRequest);
  host.registerMethod<strims_network_v1_directory_UnpublishRequest, strims_network_v1_directory_UnpublishResponse>("strims.network.v1.directory.Directory.Unpublish", (req, call) => service.unpublish(req, call), strims_network_v1_directory_UnpublishRequest);
  host.registerMethod<strims_network_v1_directory_JoinRequest, strims_network_v1_directory_JoinResponse>("strims.network.v1.directory.Directory.Join", (req, call) => service.join(req, call), strims_network_v1_directory_JoinRequest);
  host.registerMethod<strims_network_v1_directory_PartRequest, strims_network_v1_directory_PartResponse>("strims.network.v1.directory.Directory.Part", (req, call) => service.part(req, call), strims_network_v1_directory_PartRequest);
  host.registerMethod<strims_network_v1_directory_PingRequest, strims_network_v1_directory_PingResponse>("strims.network.v1.directory.Directory.Ping", (req, call) => service.ping(req, call), strims_network_v1_directory_PingRequest);
  host.registerMethod<strims_network_v1_directory_ModerateListingRequest, strims_network_v1_directory_ModerateListingResponse>("strims.network.v1.directory.Directory.ModerateListing", (req, call) => service.moderateListing(req, call), strims_network_v1_directory_ModerateListingRequest);
  host.registerMethod<strims_network_v1_directory_ModerateUserRequest, strims_network_v1_directory_ModerateUserResponse>("strims.network.v1.directory.Directory.ModerateUser", (req, call) => service.moderateUser(req, call), strims_network_v1_directory_ModerateUserRequest);
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
  getListing(req: strims_network_v1_directory_FrontendGetListingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetListingResponse> | strims_network_v1_directory_FrontendGetListingResponse;
  getListings(req: strims_network_v1_directory_FrontendGetListingsRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetListingsResponse> | strims_network_v1_directory_FrontendGetListingsResponse;
  watchListings(req: strims_network_v1_directory_FrontendWatchListingsRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchListingsResponse>;
  watchListingUsers(req: strims_network_v1_directory_FrontendWatchListingUsersRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchListingUsersResponse>;
  watchAssetBundles(req: strims_network_v1_directory_FrontendWatchAssetBundlesRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchAssetBundlesResponse>;
}

export class UnimplementedDirectoryFrontendService implements DirectoryFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  publish(req: strims_network_v1_directory_FrontendPublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendPublishResponse> | strims_network_v1_directory_FrontendPublishResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  unpublish(req: strims_network_v1_directory_FrontendUnpublishRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendUnpublishResponse> | strims_network_v1_directory_FrontendUnpublishResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  join(req: strims_network_v1_directory_FrontendJoinRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendJoinResponse> | strims_network_v1_directory_FrontendJoinResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  part(req: strims_network_v1_directory_FrontendPartRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendPartResponse> | strims_network_v1_directory_FrontendPartResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  test(req: strims_network_v1_directory_FrontendTestRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendTestResponse> | strims_network_v1_directory_FrontendTestResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  moderateListing(req: strims_network_v1_directory_FrontendModerateListingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendModerateListingResponse> | strims_network_v1_directory_FrontendModerateListingResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  moderateUser(req: strims_network_v1_directory_FrontendModerateUserRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendModerateUserResponse> | strims_network_v1_directory_FrontendModerateUserResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getUsers(req: strims_network_v1_directory_FrontendGetUsersRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetUsersResponse> | strims_network_v1_directory_FrontendGetUsersResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getListing(req: strims_network_v1_directory_FrontendGetListingRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetListingResponse> | strims_network_v1_directory_FrontendGetListingResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getListings(req: strims_network_v1_directory_FrontendGetListingsRequest, call: strims_rpc_Call): Promise<strims_network_v1_directory_FrontendGetListingsResponse> | strims_network_v1_directory_FrontendGetListingsResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watchListings(req: strims_network_v1_directory_FrontendWatchListingsRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchListingsResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watchListingUsers(req: strims_network_v1_directory_FrontendWatchListingUsersRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchListingUsersResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watchAssetBundles(req: strims_network_v1_directory_FrontendWatchAssetBundlesRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_FrontendWatchAssetBundlesResponse> { throw new Error("not implemented"); }
}

export const registerDirectoryFrontendService = (host: strims_rpc_Service, service: DirectoryFrontendService): void => {
  host.registerMethod<strims_network_v1_directory_FrontendPublishRequest, strims_network_v1_directory_FrontendPublishResponse>("strims.network.v1.directory.DirectoryFrontend.Publish", (req, call) => service.publish(req, call), strims_network_v1_directory_FrontendPublishRequest);
  host.registerMethod<strims_network_v1_directory_FrontendUnpublishRequest, strims_network_v1_directory_FrontendUnpublishResponse>("strims.network.v1.directory.DirectoryFrontend.Unpublish", (req, call) => service.unpublish(req, call), strims_network_v1_directory_FrontendUnpublishRequest);
  host.registerMethod<strims_network_v1_directory_FrontendJoinRequest, strims_network_v1_directory_FrontendJoinResponse>("strims.network.v1.directory.DirectoryFrontend.Join", (req, call) => service.join(req, call), strims_network_v1_directory_FrontendJoinRequest);
  host.registerMethod<strims_network_v1_directory_FrontendPartRequest, strims_network_v1_directory_FrontendPartResponse>("strims.network.v1.directory.DirectoryFrontend.Part", (req, call) => service.part(req, call), strims_network_v1_directory_FrontendPartRequest);
  host.registerMethod<strims_network_v1_directory_FrontendTestRequest, strims_network_v1_directory_FrontendTestResponse>("strims.network.v1.directory.DirectoryFrontend.Test", (req, call) => service.test(req, call), strims_network_v1_directory_FrontendTestRequest);
  host.registerMethod<strims_network_v1_directory_FrontendModerateListingRequest, strims_network_v1_directory_FrontendModerateListingResponse>("strims.network.v1.directory.DirectoryFrontend.ModerateListing", (req, call) => service.moderateListing(req, call), strims_network_v1_directory_FrontendModerateListingRequest);
  host.registerMethod<strims_network_v1_directory_FrontendModerateUserRequest, strims_network_v1_directory_FrontendModerateUserResponse>("strims.network.v1.directory.DirectoryFrontend.ModerateUser", (req, call) => service.moderateUser(req, call), strims_network_v1_directory_FrontendModerateUserRequest);
  host.registerMethod<strims_network_v1_directory_FrontendGetUsersRequest, strims_network_v1_directory_FrontendGetUsersResponse>("strims.network.v1.directory.DirectoryFrontend.GetUsers", (req, call) => service.getUsers(req, call), strims_network_v1_directory_FrontendGetUsersRequest);
  host.registerMethod<strims_network_v1_directory_FrontendGetListingRequest, strims_network_v1_directory_FrontendGetListingResponse>("strims.network.v1.directory.DirectoryFrontend.GetListing", (req, call) => service.getListing(req, call), strims_network_v1_directory_FrontendGetListingRequest);
  host.registerMethod<strims_network_v1_directory_FrontendGetListingsRequest, strims_network_v1_directory_FrontendGetListingsResponse>("strims.network.v1.directory.DirectoryFrontend.GetListings", (req, call) => service.getListings(req, call), strims_network_v1_directory_FrontendGetListingsRequest);
  host.registerMethod<strims_network_v1_directory_FrontendWatchListingsRequest, strims_network_v1_directory_FrontendWatchListingsResponse>("strims.network.v1.directory.DirectoryFrontend.WatchListings", (req, call) => service.watchListings(req, call), strims_network_v1_directory_FrontendWatchListingsRequest);
  host.registerMethod<strims_network_v1_directory_FrontendWatchListingUsersRequest, strims_network_v1_directory_FrontendWatchListingUsersResponse>("strims.network.v1.directory.DirectoryFrontend.WatchListingUsers", (req, call) => service.watchListingUsers(req, call), strims_network_v1_directory_FrontendWatchListingUsersRequest);
  host.registerMethod<strims_network_v1_directory_FrontendWatchAssetBundlesRequest, strims_network_v1_directory_FrontendWatchAssetBundlesResponse>("strims.network.v1.directory.DirectoryFrontend.WatchAssetBundles", (req, call) => service.watchAssetBundles(req, call), strims_network_v1_directory_FrontendWatchAssetBundlesRequest);
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

  public getListing(req?: strims_network_v1_directory_IFrontendGetListingRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_directory_FrontendGetListingResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.directory.DirectoryFrontend.GetListing", new strims_network_v1_directory_FrontendGetListingRequest(req)), strims_network_v1_directory_FrontendGetListingResponse, opts);
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

  public watchAssetBundles(req?: strims_network_v1_directory_IFrontendWatchAssetBundlesRequest): GenericReadable<strims_network_v1_directory_FrontendWatchAssetBundlesResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.directory.DirectoryFrontend.WatchAssetBundles", new strims_network_v1_directory_FrontendWatchAssetBundlesRequest(req)), strims_network_v1_directory_FrontendWatchAssetBundlesResponse);
  }
}

export interface DirectorySnippetService {
  subscribe(req: strims_network_v1_directory_SnippetSubscribeRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_SnippetSubscribeResponse>;
}

export class UnimplementedDirectorySnippetService implements DirectorySnippetService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  subscribe(req: strims_network_v1_directory_SnippetSubscribeRequest, call: strims_rpc_Call): GenericReadable<strims_network_v1_directory_SnippetSubscribeResponse> { throw new Error("not implemented"); }
}

export const registerDirectorySnippetService = (host: strims_rpc_Service, service: DirectorySnippetService): void => {
  host.registerMethod<strims_network_v1_directory_SnippetSubscribeRequest, strims_network_v1_directory_SnippetSubscribeResponse>("strims.network.v1.directory.DirectorySnippet.Subscribe", (req, call) => service.subscribe(req, call), strims_network_v1_directory_SnippetSubscribeRequest);
}

export class DirectorySnippetClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public subscribe(req?: strims_network_v1_directory_ISnippetSubscribeRequest): GenericReadable<strims_network_v1_directory_SnippetSubscribeResponse> {
    return this.host.expectMany(this.host.call("strims.network.v1.directory.DirectorySnippet.Subscribe", new strims_network_v1_directory_SnippetSubscribeRequest(req)), strims_network_v1_directory_SnippetSubscribeResponse);
  }
}

