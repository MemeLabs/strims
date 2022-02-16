import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IVideoChannelListRequest,
  VideoChannelListRequest,
  VideoChannelListResponse,
  IVideoChannelGetRequest,
  VideoChannelGetRequest,
  VideoChannelGetResponse,
  IVideoChannelCreateRequest,
  VideoChannelCreateRequest,
  VideoChannelCreateResponse,
  IVideoChannelUpdateRequest,
  VideoChannelUpdateRequest,
  VideoChannelUpdateResponse,
  IVideoChannelDeleteRequest,
  VideoChannelDeleteRequest,
  VideoChannelDeleteResponse,
} from "./channel";

export interface VideoChannelFrontendService {
  list(req: VideoChannelListRequest, call: strims_rpc_Call): Promise<VideoChannelListResponse> | VideoChannelListResponse;
  get(req: VideoChannelGetRequest, call: strims_rpc_Call): Promise<VideoChannelGetResponse> | VideoChannelGetResponse;
  create(req: VideoChannelCreateRequest, call: strims_rpc_Call): Promise<VideoChannelCreateResponse> | VideoChannelCreateResponse;
  update(req: VideoChannelUpdateRequest, call: strims_rpc_Call): Promise<VideoChannelUpdateResponse> | VideoChannelUpdateResponse;
  delete(req: VideoChannelDeleteRequest, call: strims_rpc_Call): Promise<VideoChannelDeleteResponse> | VideoChannelDeleteResponse;
}

export const registerVideoChannelFrontendService = (host: strims_rpc_Service, service: VideoChannelFrontendService): void => {
  host.registerMethod<VideoChannelListRequest, VideoChannelListResponse>("strims.video.v1.VideoChannelFrontend.List", service.list.bind(service), VideoChannelListRequest);
  host.registerMethod<VideoChannelGetRequest, VideoChannelGetResponse>("strims.video.v1.VideoChannelFrontend.Get", service.get.bind(service), VideoChannelGetRequest);
  host.registerMethod<VideoChannelCreateRequest, VideoChannelCreateResponse>("strims.video.v1.VideoChannelFrontend.Create", service.create.bind(service), VideoChannelCreateRequest);
  host.registerMethod<VideoChannelUpdateRequest, VideoChannelUpdateResponse>("strims.video.v1.VideoChannelFrontend.Update", service.update.bind(service), VideoChannelUpdateRequest);
  host.registerMethod<VideoChannelDeleteRequest, VideoChannelDeleteResponse>("strims.video.v1.VideoChannelFrontend.Delete", service.delete.bind(service), VideoChannelDeleteRequest);
}

export class VideoChannelFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public list(req?: IVideoChannelListRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelListResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.List", new VideoChannelListRequest(req)), VideoChannelListResponse, opts);
  }

  public get(req?: IVideoChannelGetRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelGetResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Get", new VideoChannelGetRequest(req)), VideoChannelGetResponse, opts);
  }

  public create(req?: IVideoChannelCreateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelCreateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Create", new VideoChannelCreateRequest(req)), VideoChannelCreateResponse, opts);
  }

  public update(req?: IVideoChannelUpdateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelUpdateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Update", new VideoChannelUpdateRequest(req)), VideoChannelUpdateResponse, opts);
  }

  public delete(req?: IVideoChannelDeleteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelDeleteResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Delete", new VideoChannelDeleteRequest(req)), VideoChannelDeleteResponse, opts);
  }
}

