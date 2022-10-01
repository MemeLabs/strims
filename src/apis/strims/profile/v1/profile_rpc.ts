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
  strims_profile_v1_IDeleteDeviceRequest,
  strims_profile_v1_DeleteDeviceRequest,
  strims_profile_v1_DeleteDeviceResponse,
  strims_profile_v1_IGetDeviceRequest,
  strims_profile_v1_GetDeviceRequest,
  strims_profile_v1_GetDeviceResponse,
  strims_profile_v1_IListDevicesRequest,
  strims_profile_v1_ListDevicesRequest,
  strims_profile_v1_ListDevicesResponse,
} from "./profile";

export interface ProfileFrontendService {
  get(req: strims_profile_v1_GetProfileRequest, call: strims_rpc_Call): Promise<strims_profile_v1_GetProfileResponse> | strims_profile_v1_GetProfileResponse;
  update(req: strims_profile_v1_UpdateProfileRequest, call: strims_rpc_Call): Promise<strims_profile_v1_UpdateProfileResponse> | strims_profile_v1_UpdateProfileResponse;
  deleteDevice(req: strims_profile_v1_DeleteDeviceRequest, call: strims_rpc_Call): Promise<strims_profile_v1_DeleteDeviceResponse> | strims_profile_v1_DeleteDeviceResponse;
  getDevice(req: strims_profile_v1_GetDeviceRequest, call: strims_rpc_Call): Promise<strims_profile_v1_GetDeviceResponse> | strims_profile_v1_GetDeviceResponse;
  listDevices(req: strims_profile_v1_ListDevicesRequest, call: strims_rpc_Call): Promise<strims_profile_v1_ListDevicesResponse> | strims_profile_v1_ListDevicesResponse;
}

export class UnimplementedProfileFrontendService implements ProfileFrontendService {
  get(req: strims_profile_v1_GetProfileRequest, call: strims_rpc_Call): Promise<strims_profile_v1_GetProfileResponse> | strims_profile_v1_GetProfileResponse { throw new Error("not implemented"); }
  update(req: strims_profile_v1_UpdateProfileRequest, call: strims_rpc_Call): Promise<strims_profile_v1_UpdateProfileResponse> | strims_profile_v1_UpdateProfileResponse { throw new Error("not implemented"); }
  deleteDevice(req: strims_profile_v1_DeleteDeviceRequest, call: strims_rpc_Call): Promise<strims_profile_v1_DeleteDeviceResponse> | strims_profile_v1_DeleteDeviceResponse { throw new Error("not implemented"); }
  getDevice(req: strims_profile_v1_GetDeviceRequest, call: strims_rpc_Call): Promise<strims_profile_v1_GetDeviceResponse> | strims_profile_v1_GetDeviceResponse { throw new Error("not implemented"); }
  listDevices(req: strims_profile_v1_ListDevicesRequest, call: strims_rpc_Call): Promise<strims_profile_v1_ListDevicesResponse> | strims_profile_v1_ListDevicesResponse { throw new Error("not implemented"); }
}

export const registerProfileFrontendService = (host: strims_rpc_Service, service: ProfileFrontendService): void => {
  host.registerMethod<strims_profile_v1_GetProfileRequest, strims_profile_v1_GetProfileResponse>("strims.profile.v1.ProfileFrontend.Get", service.get.bind(service), strims_profile_v1_GetProfileRequest);
  host.registerMethod<strims_profile_v1_UpdateProfileRequest, strims_profile_v1_UpdateProfileResponse>("strims.profile.v1.ProfileFrontend.Update", service.update.bind(service), strims_profile_v1_UpdateProfileRequest);
  host.registerMethod<strims_profile_v1_DeleteDeviceRequest, strims_profile_v1_DeleteDeviceResponse>("strims.profile.v1.ProfileFrontend.DeleteDevice", service.deleteDevice.bind(service), strims_profile_v1_DeleteDeviceRequest);
  host.registerMethod<strims_profile_v1_GetDeviceRequest, strims_profile_v1_GetDeviceResponse>("strims.profile.v1.ProfileFrontend.GetDevice", service.getDevice.bind(service), strims_profile_v1_GetDeviceRequest);
  host.registerMethod<strims_profile_v1_ListDevicesRequest, strims_profile_v1_ListDevicesResponse>("strims.profile.v1.ProfileFrontend.ListDevices", service.listDevices.bind(service), strims_profile_v1_ListDevicesRequest);
}

export class ProfileFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public get(req?: strims_profile_v1_IGetProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_profile_v1_GetProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.Get", new strims_profile_v1_GetProfileRequest(req)), strims_profile_v1_GetProfileResponse, opts);
  }

  public update(req?: strims_profile_v1_IUpdateProfileRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_profile_v1_UpdateProfileResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.Update", new strims_profile_v1_UpdateProfileRequest(req)), strims_profile_v1_UpdateProfileResponse, opts);
  }

  public deleteDevice(req?: strims_profile_v1_IDeleteDeviceRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_profile_v1_DeleteDeviceResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.DeleteDevice", new strims_profile_v1_DeleteDeviceRequest(req)), strims_profile_v1_DeleteDeviceResponse, opts);
  }

  public getDevice(req?: strims_profile_v1_IGetDeviceRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_profile_v1_GetDeviceResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.GetDevice", new strims_profile_v1_GetDeviceRequest(req)), strims_profile_v1_GetDeviceResponse, opts);
  }

  public listDevices(req?: strims_profile_v1_IListDevicesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_profile_v1_ListDevicesResponse> {
    return this.host.expectOne(this.host.call("strims.profile.v1.ProfileFrontend.ListDevices", new strims_profile_v1_ListDevicesRequest(req)), strims_profile_v1_ListDevicesResponse, opts);
  }
}

