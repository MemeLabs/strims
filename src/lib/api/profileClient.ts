import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class Profile {
  constructor(private readonly host: RPCHost) {}

  public create(
    arg: pb.ICreateProfileRequest = new pb.CreateProfileRequest()
  ): Promise<pb.CreateProfileResponse> {
    return this.host.expectOne(this.host.call("Profile/Create", new pb.CreateProfileRequest(arg)));
  }
  public load(
    arg: pb.ILoadProfileRequest = new pb.LoadProfileRequest()
  ): Promise<pb.LoadProfileResponse> {
    return this.host.expectOne(this.host.call("Profile/Load", new pb.LoadProfileRequest(arg)));
  }
  public get(
    arg: pb.IGetProfileRequest = new pb.GetProfileRequest()
  ): Promise<pb.GetProfileResponse> {
    return this.host.expectOne(this.host.call("Profile/Get", new pb.GetProfileRequest(arg)));
  }
  public update(
    arg: pb.IUpdateProfileRequest = new pb.UpdateProfileRequest()
  ): Promise<pb.UpdateProfileResponse> {
    return this.host.expectOne(this.host.call("Profile/Update", new pb.UpdateProfileRequest(arg)));
  }
  public delete(
    arg: pb.IDeleteProfileRequest = new pb.DeleteProfileRequest()
  ): Promise<pb.DeleteProfileResponse> {
    return this.host.expectOne(this.host.call("Profile/Delete", new pb.DeleteProfileRequest(arg)));
  }
  public list(
    arg: pb.IListProfilesRequest = new pb.ListProfilesRequest()
  ): Promise<pb.ListProfilesResponse> {
    return this.host.expectOne(this.host.call("Profile/List", new pb.ListProfilesRequest(arg)));
  }
  public loadSession(
    arg: pb.ILoadSessionRequest = new pb.LoadSessionRequest()
  ): Promise<pb.LoadSessionResponse> {
    return this.host.expectOne(
      this.host.call("Profile/LoadSession", new pb.LoadSessionRequest(arg))
    );
  }
}
