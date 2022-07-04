import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_profile_v1_IGetProfileRequest,
  strims_profile_v1_GetProfileRequest,
  strims_profile_v1_GetProfileResponse,
  strims_profile_v1_IUpdateProfileRequest,
  strims_profile_v1_UpdateProfileRequest,
  strims_profile_v1_UpdateProfileResponse,
} from "./profile";

export interface ProfileFrontendService {
  get(req: strims_profile_v1_GetProfileRequest, call: strims_rpc_Call): Promise<strims_profile_v1_GetProfileResponse> | strims_profile_v1_GetProfileResponse;
  update(req: strims_profile_v1_UpdateProfileRequest, call: strims_rpc_Call): Promise<strims_profile_v1_UpdateProfileResponse> | strims_profile_v1_UpdateProfileResponse;
}

export class UnimplementedProfileFrontendService implements ProfileFrontendService {
  get(req: strims_profile_v1_GetProfileRequest, call: strims_rpc_Call): Promise<strims_profile_v1_GetProfileResponse> | strims_profile_v1_GetProfileResponse { throw new Error("not implemented"); }
  update(req: strims_profile_v1_UpdateProfileRequest, call: strims_rpc_Call): Promise<strims_profile_v1_UpdateProfileResponse> | strims_profile_v1_UpdateProfileResponse { throw new Error("not implemented"); }
}

export const registerProfileFrontendService = (host: strims_rpc_Service, service: ProfileFrontendService): void => {
  host.registerMethod<strims_profile_v1_GetProfileRequest, strims_profile_v1_GetProfileResponse>("strims.profile.v1.ProfileFrontend.Get", service.get.bind(service), strims_profile_v1_GetProfileRequest);
  host.registerMethod<strims_profile_v1_UpdateProfileRequest, strims_profile_v1_UpdateProfileResponse>("strims.profile.v1.ProfileFrontend.Update", service.update.bind(service), strims_profile_v1_UpdateProfileRequest);
}

export class ProfileFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public get(req?: strims_profile_v1_IGetProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_profile_v1_GetProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.Get", new strims_profile_v1_GetProfileRequest(req)), strims_profile_v1_GetProfileResponse, opts);
  }

  public update(req?: strims_profile_v1_IUpdateProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_profile_v1_UpdateProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.Update", new strims_profile_v1_UpdateProfileRequest(req)), strims_profile_v1_UpdateProfileResponse, opts);
  }
}

