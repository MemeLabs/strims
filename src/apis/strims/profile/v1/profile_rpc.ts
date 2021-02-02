import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

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

export class ProfileServiceClient {
  constructor(private readonly host: RPCHost) {}

  public create(arg: ICreateProfileRequest = new CreateProfileRequest()): Promise<CreateProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Create", new CreateProfileRequest(arg)));
  }

  public load(arg: ILoadProfileRequest = new LoadProfileRequest()): Promise<LoadProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Load", new LoadProfileRequest(arg)));
  }

  public get(arg: IGetProfileRequest = new GetProfileRequest()): Promise<GetProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Get", new GetProfileRequest(arg)));
  }

  public update(arg: IUpdateProfileRequest = new UpdateProfileRequest()): Promise<UpdateProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Update", new UpdateProfileRequest(arg)));
  }

  public delete(arg: IDeleteProfileRequest = new DeleteProfileRequest()): Promise<DeleteProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.Delete", new DeleteProfileRequest(arg)));
  }

  public list(arg: IListProfilesRequest = new ListProfilesRequest()): Promise<ListProfilesResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.List", new ListProfilesRequest(arg)));
  }

  public loadSession(arg: ILoadSessionRequest = new LoadSessionRequest()): Promise<LoadSessionResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileService.LoadSession", new LoadSessionRequest(arg)));
  }
}

