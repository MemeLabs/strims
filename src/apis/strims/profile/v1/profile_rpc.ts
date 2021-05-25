import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ICreateProfileRequest,
  CreateProfileRequest,
  CreateProfileResponse,
  ILoadProfileRequest,
  LoadProfileRequest,
  LoadProfileResponse,
  IGetProfileRequest,
  GetProfileRequest,
  GetProfileResponse,
  IUpdateProfileRequest,
  UpdateProfileRequest,
  UpdateProfileResponse,
  IDeleteProfileRequest,
  DeleteProfileRequest,
  DeleteProfileResponse,
  IListProfilesRequest,
  ListProfilesRequest,
  ListProfilesResponse,
  ILoadSessionRequest,
  LoadSessionRequest,
  LoadSessionResponse,
} from "./profile";

registerType("strims.profile.v1.CreateProfileRequest", CreateProfileRequest);
registerType("strims.profile.v1.CreateProfileResponse", CreateProfileResponse);
registerType("strims.profile.v1.LoadProfileRequest", LoadProfileRequest);
registerType("strims.profile.v1.LoadProfileResponse", LoadProfileResponse);
registerType("strims.profile.v1.GetProfileRequest", GetProfileRequest);
registerType("strims.profile.v1.GetProfileResponse", GetProfileResponse);
registerType("strims.profile.v1.UpdateProfileRequest", UpdateProfileRequest);
registerType("strims.profile.v1.UpdateProfileResponse", UpdateProfileResponse);
registerType("strims.profile.v1.DeleteProfileRequest", DeleteProfileRequest);
registerType("strims.profile.v1.DeleteProfileResponse", DeleteProfileResponse);
registerType("strims.profile.v1.ListProfilesRequest", ListProfilesRequest);
registerType("strims.profile.v1.ListProfilesResponse", ListProfilesResponse);
registerType("strims.profile.v1.LoadSessionRequest", LoadSessionRequest);
registerType("strims.profile.v1.LoadSessionResponse", LoadSessionResponse);

export interface ProfileServiceService {
  create(req: CreateProfileRequest, call: strims_rpc_Call): Promise<CreateProfileResponse> | CreateProfileResponse;
  load(req: LoadProfileRequest, call: strims_rpc_Call): Promise<LoadProfileResponse> | LoadProfileResponse;
  get(req: GetProfileRequest, call: strims_rpc_Call): Promise<GetProfileResponse> | GetProfileResponse;
  update(req: UpdateProfileRequest, call: strims_rpc_Call): Promise<UpdateProfileResponse> | UpdateProfileResponse;
  delete(req: DeleteProfileRequest, call: strims_rpc_Call): Promise<DeleteProfileResponse> | DeleteProfileResponse;
  list(req: ListProfilesRequest, call: strims_rpc_Call): Promise<ListProfilesResponse> | ListProfilesResponse;
  loadSession(req: LoadSessionRequest, call: strims_rpc_Call): Promise<LoadSessionResponse> | LoadSessionResponse;
}

export const registerProfileServiceService = (host: strims_rpc_Service, service: ProfileServiceService): void => {
  host.registerMethod<CreateProfileRequest, CreateProfileResponse>("strims.profile.v1.ProfileService.Create", service.create.bind(service));
  host.registerMethod<LoadProfileRequest, LoadProfileResponse>("strims.profile.v1.ProfileService.Load", service.load.bind(service));
  host.registerMethod<GetProfileRequest, GetProfileResponse>("strims.profile.v1.ProfileService.Get", service.get.bind(service));
  host.registerMethod<UpdateProfileRequest, UpdateProfileResponse>("strims.profile.v1.ProfileService.Update", service.update.bind(service));
  host.registerMethod<DeleteProfileRequest, DeleteProfileResponse>("strims.profile.v1.ProfileService.Delete", service.delete.bind(service));
  host.registerMethod<ListProfilesRequest, ListProfilesResponse>("strims.profile.v1.ProfileService.List", service.list.bind(service));
  host.registerMethod<LoadSessionRequest, LoadSessionResponse>("strims.profile.v1.ProfileService.LoadSession", service.loadSession.bind(service));
}

export class ProfileServiceClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public create(req?: ICreateProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Create", new CreateProfileRequest(req)), opts);
  }

  public load(req?: ILoadProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<LoadProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Load", new LoadProfileRequest(req)), opts);
  }

  public get(req?: IGetProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Get", new GetProfileRequest(req)), opts);
  }

  public update(req?: IUpdateProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Update", new UpdateProfileRequest(req)), opts);
  }

  public delete(req?: IDeleteProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Delete", new DeleteProfileRequest(req)), opts);
  }

  public list(req?: IListProfilesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListProfilesResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.List", new ListProfilesRequest(req)), opts);
  }

  public loadSession(req?: ILoadSessionRequest, opts?: strims_rpc_UnaryCallOptions): Promise<LoadSessionResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.LoadSession", new LoadSessionRequest(req)), opts);
  }
}

