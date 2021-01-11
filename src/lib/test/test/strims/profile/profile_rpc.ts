import { RPCHost } from "../../../../rpc/host";
import { registerType } from "../../../../pb/registry";

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

registerType(".strims.profile.CreateProfileRequest", CreateProfileRequest);
registerType(".strims.profile.CreateProfileResponse", CreateProfileResponse);
registerType(".strims.profile.LoadProfileRequest", LoadProfileRequest);
registerType(".strims.profile.LoadProfileResponse", LoadProfileResponse);
registerType(".strims.profile.GetProfileRequest", GetProfileRequest);
registerType(".strims.profile.GetProfileResponse", GetProfileResponse);
registerType(".strims.profile.UpdateProfileRequest", UpdateProfileRequest);
registerType(".strims.profile.UpdateProfileResponse", UpdateProfileResponse);
registerType(".strims.profile.DeleteProfileRequest", DeleteProfileRequest);
registerType(".strims.profile.DeleteProfileResponse", DeleteProfileResponse);
registerType(".strims.profile.ListProfilesRequest", ListProfilesRequest);
registerType(".strims.profile.ListProfilesResponse", ListProfilesResponse);
registerType(".strims.profile.LoadSessionRequest", LoadSessionRequest);
registerType(".strims.profile.LoadSessionResponse", LoadSessionResponse);

export class ProfileServiceClient {
  constructor(private readonly host: RPCHost) {}

  public create(arg: ICreateProfileRequest = new CreateProfileRequest()): Promise<CreateProfileResponse> {
    return this.host.expectOne(this.host.call(".strims.profile.ProfileService.Create", new CreateProfileRequest(arg)));
  }

  public load(arg: ILoadProfileRequest = new LoadProfileRequest()): Promise<LoadProfileResponse> {
    return this.host.expectOne(this.host.call(".strims.profile.ProfileService.Load", new LoadProfileRequest(arg)));
  }

  public get(arg: IGetProfileRequest = new GetProfileRequest()): Promise<GetProfileResponse> {
    return this.host.expectOne(this.host.call(".strims.profile.ProfileService.Get", new GetProfileRequest(arg)));
  }

  public update(arg: IUpdateProfileRequest = new UpdateProfileRequest()): Promise<UpdateProfileResponse> {
    return this.host.expectOne(this.host.call(".strims.profile.ProfileService.Update", new UpdateProfileRequest(arg)));
  }

  public delete(arg: IDeleteProfileRequest = new DeleteProfileRequest()): Promise<DeleteProfileResponse> {
    return this.host.expectOne(this.host.call(".strims.profile.ProfileService.Delete", new DeleteProfileRequest(arg)));
  }

  public list(arg: IListProfilesRequest = new ListProfilesRequest()): Promise<ListProfilesResponse> {
    return this.host.expectOne(this.host.call(".strims.profile.ProfileService.List", new ListProfilesRequest(arg)));
  }

  public loadSession(arg: ILoadSessionRequest = new LoadSessionRequest()): Promise<LoadSessionResponse> {
    return this.host.expectOne(this.host.call(".strims.profile.ProfileService.LoadSession", new LoadSessionRequest(arg)));
  }
}

