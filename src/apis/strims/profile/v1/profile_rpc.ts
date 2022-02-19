import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IGetProfileRequest,
  GetProfileRequest,
  GetProfileResponse,
  IUpdateProfileRequest,
  UpdateProfileRequest,
  UpdateProfileResponse,
} from "./profile";

export interface ProfileFrontendService {
  get(req: GetProfileRequest, call: strims_rpc_Call): Promise<GetProfileResponse> | GetProfileResponse;
  update(req: UpdateProfileRequest, call: strims_rpc_Call): Promise<UpdateProfileResponse> | UpdateProfileResponse;
}

export class UnimplementedProfileFrontendService implements ProfileFrontendService {
  get(req: GetProfileRequest, call: strims_rpc_Call): Promise<GetProfileResponse> | GetProfileResponse { throw new Error("not implemented"); }
  update(req: UpdateProfileRequest, call: strims_rpc_Call): Promise<UpdateProfileResponse> | UpdateProfileResponse { throw new Error("not implemented"); }
}

export const registerProfileFrontendService = (host: strims_rpc_Service, service: ProfileFrontendService): void => {
  host.registerMethod<GetProfileRequest, GetProfileResponse>("strims.profile.v1.ProfileFrontend.Get", service.get.bind(service), GetProfileRequest);
  host.registerMethod<UpdateProfileRequest, UpdateProfileResponse>("strims.profile.v1.ProfileFrontend.Update", service.update.bind(service), UpdateProfileRequest);
}

export class ProfileFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public get(req?: IGetProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.Get", new GetProfileRequest(req)), GetProfileResponse, opts);
  }

  public update(req?: IUpdateProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.Update", new UpdateProfileRequest(req)), UpdateProfileResponse, opts);
  }
}

