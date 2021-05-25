import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  IVideoChannelListRequest,
  VideoChannelListRequest,
  VideoChannelListResponse,
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

registerType("strims.video.v1.VideoChannelListRequest", VideoChannelListRequest);
registerType("strims.video.v1.VideoChannelListResponse", VideoChannelListResponse);
registerType("strims.video.v1.VideoChannelCreateRequest", VideoChannelCreateRequest);
registerType("strims.video.v1.VideoChannelCreateResponse", VideoChannelCreateResponse);
registerType("strims.video.v1.VideoChannelUpdateRequest", VideoChannelUpdateRequest);
registerType("strims.video.v1.VideoChannelUpdateResponse", VideoChannelUpdateResponse);
registerType("strims.video.v1.VideoChannelDeleteRequest", VideoChannelDeleteRequest);
registerType("strims.video.v1.VideoChannelDeleteResponse", VideoChannelDeleteResponse);

export interface VideoChannelFrontendService {
  list(req: VideoChannelListRequest, call: strims_rpc_Call): Promise<VideoChannelListResponse> | VideoChannelListResponse;
  create(req: VideoChannelCreateRequest, call: strims_rpc_Call): Promise<VideoChannelCreateResponse> | VideoChannelCreateResponse;
  update(req: VideoChannelUpdateRequest, call: strims_rpc_Call): Promise<VideoChannelUpdateResponse> | VideoChannelUpdateResponse;
  delete(req: VideoChannelDeleteRequest, call: strims_rpc_Call): Promise<VideoChannelDeleteResponse> | VideoChannelDeleteResponse;
}

export const registerVideoChannelFrontendService = (host: strims_rpc_Service, service: VideoChannelFrontendService): void => {
  host.registerMethod<VideoChannelListRequest, VideoChannelListResponse>("strims.video.v1.VideoChannelFrontend.List", service.list.bind(service));
  host.registerMethod<VideoChannelCreateRequest, VideoChannelCreateResponse>("strims.video.v1.VideoChannelFrontend.Create", service.create.bind(service));
  host.registerMethod<VideoChannelUpdateRequest, VideoChannelUpdateResponse>("strims.video.v1.VideoChannelFrontend.Update", service.update.bind(service));
  host.registerMethod<VideoChannelDeleteRequest, VideoChannelDeleteResponse>("strims.video.v1.VideoChannelFrontend.Delete", service.delete.bind(service));
}

export class VideoChannelFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public list(req?: IVideoChannelListRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelListResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.List", new VideoChannelListRequest(req)), opts);
  }

  public create(req?: IVideoChannelCreateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelCreateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Create", new VideoChannelCreateRequest(req)), opts);
  }

  public update(req?: IVideoChannelUpdateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelUpdateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Update", new VideoChannelUpdateRequest(req)), opts);
  }

  public delete(req?: IVideoChannelDeleteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<VideoChannelDeleteResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Delete", new VideoChannelDeleteRequest(req)), opts);
  }
}

