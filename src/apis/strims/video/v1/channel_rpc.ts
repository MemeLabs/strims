import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_video_v1_IVideoChannelListRequest,
  strims_video_v1_VideoChannelListRequest,
  strims_video_v1_VideoChannelListResponse,
  strims_video_v1_IVideoChannelGetRequest,
  strims_video_v1_VideoChannelGetRequest,
  strims_video_v1_VideoChannelGetResponse,
  strims_video_v1_IVideoChannelCreateRequest,
  strims_video_v1_VideoChannelCreateRequest,
  strims_video_v1_VideoChannelCreateResponse,
  strims_video_v1_IVideoChannelUpdateRequest,
  strims_video_v1_VideoChannelUpdateRequest,
  strims_video_v1_VideoChannelUpdateResponse,
  strims_video_v1_IVideoChannelDeleteRequest,
  strims_video_v1_VideoChannelDeleteRequest,
  strims_video_v1_VideoChannelDeleteResponse,
} from "./channel";

export interface VideoChannelFrontendService {
  list(req: strims_video_v1_VideoChannelListRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelListResponse> | strims_video_v1_VideoChannelListResponse;
  get(req: strims_video_v1_VideoChannelGetRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelGetResponse> | strims_video_v1_VideoChannelGetResponse;
  create(req: strims_video_v1_VideoChannelCreateRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelCreateResponse> | strims_video_v1_VideoChannelCreateResponse;
  update(req: strims_video_v1_VideoChannelUpdateRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelUpdateResponse> | strims_video_v1_VideoChannelUpdateResponse;
  delete(req: strims_video_v1_VideoChannelDeleteRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelDeleteResponse> | strims_video_v1_VideoChannelDeleteResponse;
}

export class UnimplementedVideoChannelFrontendService implements VideoChannelFrontendService {
  list(req: strims_video_v1_VideoChannelListRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelListResponse> | strims_video_v1_VideoChannelListResponse { throw new Error("not implemented"); }
  get(req: strims_video_v1_VideoChannelGetRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelGetResponse> | strims_video_v1_VideoChannelGetResponse { throw new Error("not implemented"); }
  create(req: strims_video_v1_VideoChannelCreateRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelCreateResponse> | strims_video_v1_VideoChannelCreateResponse { throw new Error("not implemented"); }
  update(req: strims_video_v1_VideoChannelUpdateRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelUpdateResponse> | strims_video_v1_VideoChannelUpdateResponse { throw new Error("not implemented"); }
  delete(req: strims_video_v1_VideoChannelDeleteRequest, call: strims_rpc_Call): Promise<strims_video_v1_VideoChannelDeleteResponse> | strims_video_v1_VideoChannelDeleteResponse { throw new Error("not implemented"); }
}

export const registerVideoChannelFrontendService = (host: strims_rpc_Service, service: VideoChannelFrontendService): void => {
  host.registerMethod<strims_video_v1_VideoChannelListRequest, strims_video_v1_VideoChannelListResponse>("strims.video.v1.VideoChannelFrontend.List", service.list.bind(service), strims_video_v1_VideoChannelListRequest);
  host.registerMethod<strims_video_v1_VideoChannelGetRequest, strims_video_v1_VideoChannelGetResponse>("strims.video.v1.VideoChannelFrontend.Get", service.get.bind(service), strims_video_v1_VideoChannelGetRequest);
  host.registerMethod<strims_video_v1_VideoChannelCreateRequest, strims_video_v1_VideoChannelCreateResponse>("strims.video.v1.VideoChannelFrontend.Create", service.create.bind(service), strims_video_v1_VideoChannelCreateRequest);
  host.registerMethod<strims_video_v1_VideoChannelUpdateRequest, strims_video_v1_VideoChannelUpdateResponse>("strims.video.v1.VideoChannelFrontend.Update", service.update.bind(service), strims_video_v1_VideoChannelUpdateRequest);
  host.registerMethod<strims_video_v1_VideoChannelDeleteRequest, strims_video_v1_VideoChannelDeleteResponse>("strims.video.v1.VideoChannelFrontend.Delete", service.delete.bind(service), strims_video_v1_VideoChannelDeleteRequest);
}

export class VideoChannelFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public list(req?: strims_video_v1_IVideoChannelListRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoChannelListResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.List", new strims_video_v1_VideoChannelListRequest(req)), strims_video_v1_VideoChannelListResponse, opts);
  }

  public get(req?: strims_video_v1_IVideoChannelGetRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoChannelGetResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Get", new strims_video_v1_VideoChannelGetRequest(req)), strims_video_v1_VideoChannelGetResponse, opts);
  }

  public create(req?: strims_video_v1_IVideoChannelCreateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoChannelCreateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Create", new strims_video_v1_VideoChannelCreateRequest(req)), strims_video_v1_VideoChannelCreateResponse, opts);
  }

  public update(req?: strims_video_v1_IVideoChannelUpdateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoChannelUpdateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Update", new strims_video_v1_VideoChannelUpdateRequest(req)), strims_video_v1_VideoChannelUpdateResponse, opts);
  }

  public delete(req?: strims_video_v1_IVideoChannelDeleteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_VideoChannelDeleteResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Delete", new strims_video_v1_VideoChannelDeleteRequest(req)), strims_video_v1_VideoChannelDeleteResponse, opts);
  }
}

