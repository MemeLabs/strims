import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  ICreateServerRequest,
  CreateServerRequest,
  CreateServerResponse,
  IUpdateServerRequest,
  UpdateServerRequest,
  UpdateServerResponse,
  IDeleteServerRequest,
  DeleteServerRequest,
  DeleteServerResponse,
  IGetServerRequest,
  GetServerRequest,
  GetServerResponse,
  IListServersRequest,
  ListServersRequest,
  ListServersResponse,
  ICreateEmoteRequest,
  CreateEmoteRequest,
  CreateEmoteResponse,
  IUpdateEmoteRequest,
  UpdateEmoteRequest,
  UpdateEmoteResponse,
  IDeleteEmoteRequest,
  DeleteEmoteRequest,
  DeleteEmoteResponse,
  IGetEmoteRequest,
  GetEmoteRequest,
  GetEmoteResponse,
  IListEmotesRequest,
  ListEmotesRequest,
  ListEmotesResponse,
  ICreateModifierRequest,
  CreateModifierRequest,
  CreateModifierResponse,
  IUpdateModifierRequest,
  UpdateModifierRequest,
  UpdateModifierResponse,
  IDeleteModifierRequest,
  DeleteModifierRequest,
  DeleteModifierResponse,
  IGetModifierRequest,
  GetModifierRequest,
  GetModifierResponse,
  IListModifiersRequest,
  ListModifiersRequest,
  ListModifiersResponse,
  ICreateTagRequest,
  CreateTagRequest,
  CreateTagResponse,
  IUpdateTagRequest,
  UpdateTagRequest,
  UpdateTagResponse,
  IDeleteTagRequest,
  DeleteTagRequest,
  DeleteTagResponse,
  IGetTagRequest,
  GetTagRequest,
  GetTagResponse,
  IListTagsRequest,
  ListTagsRequest,
  ListTagsResponse,
  ISyncAssetsRequest,
  SyncAssetsRequest,
  SyncAssetsResponse,
  IOpenClientRequest,
  OpenClientRequest,
  OpenClientResponse,
  IClientSendMessageRequest,
  ClientSendMessageRequest,
  ClientSendMessageResponse,
  IClientMuteRequest,
  ClientMuteRequest,
  ClientMuteResponse,
  IClientUnmuteRequest,
  ClientUnmuteRequest,
  ClientUnmuteResponse,
  IClientGetMuteRequest,
  ClientGetMuteRequest,
  ClientGetMuteResponse,
  IWhisperRequest,
  WhisperRequest,
  WhisperResponse,
  ISetUIConfigRequest,
  SetUIConfigRequest,
  SetUIConfigResponse,
  IWatchUIConfigRequest,
  WatchUIConfigRequest,
  WatchUIConfigResponse,
  IIgnoreRequest,
  IgnoreRequest,
  IgnoreResponse,
  IUnignoreRequest,
  UnignoreRequest,
  UnignoreResponse,
  IHighlightRequest,
  HighlightRequest,
  HighlightResponse,
  IUnhighlightRequest,
  UnhighlightRequest,
  UnhighlightResponse,
  ITagRequest,
  TagRequest,
  TagResponse,
  IUntagRequest,
  UntagRequest,
  UntagResponse,
  ISendMessageRequest,
  SendMessageRequest,
  SendMessageResponse,
  IMuteRequest,
  MuteRequest,
  MuteResponse,
  IUnmuteRequest,
  UnmuteRequest,
  UnmuteResponse,
  IGetMuteRequest,
  GetMuteRequest,
  GetMuteResponse,
  IWhisperSendMessageRequest,
  WhisperSendMessageRequest,
  WhisperSendMessageResponse,
} from "./chat";

export interface ChatServerFrontendService {
  createServer(req: CreateServerRequest, call: strims_rpc_Call): Promise<CreateServerResponse> | CreateServerResponse;
  updateServer(req: UpdateServerRequest, call: strims_rpc_Call): Promise<UpdateServerResponse> | UpdateServerResponse;
  deleteServer(req: DeleteServerRequest, call: strims_rpc_Call): Promise<DeleteServerResponse> | DeleteServerResponse;
  getServer(req: GetServerRequest, call: strims_rpc_Call): Promise<GetServerResponse> | GetServerResponse;
  listServers(req: ListServersRequest, call: strims_rpc_Call): Promise<ListServersResponse> | ListServersResponse;
  createEmote(req: CreateEmoteRequest, call: strims_rpc_Call): Promise<CreateEmoteResponse> | CreateEmoteResponse;
  updateEmote(req: UpdateEmoteRequest, call: strims_rpc_Call): Promise<UpdateEmoteResponse> | UpdateEmoteResponse;
  deleteEmote(req: DeleteEmoteRequest, call: strims_rpc_Call): Promise<DeleteEmoteResponse> | DeleteEmoteResponse;
  getEmote(req: GetEmoteRequest, call: strims_rpc_Call): Promise<GetEmoteResponse> | GetEmoteResponse;
  listEmotes(req: ListEmotesRequest, call: strims_rpc_Call): Promise<ListEmotesResponse> | ListEmotesResponse;
  createModifier(req: CreateModifierRequest, call: strims_rpc_Call): Promise<CreateModifierResponse> | CreateModifierResponse;
  updateModifier(req: UpdateModifierRequest, call: strims_rpc_Call): Promise<UpdateModifierResponse> | UpdateModifierResponse;
  deleteModifier(req: DeleteModifierRequest, call: strims_rpc_Call): Promise<DeleteModifierResponse> | DeleteModifierResponse;
  getModifier(req: GetModifierRequest, call: strims_rpc_Call): Promise<GetModifierResponse> | GetModifierResponse;
  listModifiers(req: ListModifiersRequest, call: strims_rpc_Call): Promise<ListModifiersResponse> | ListModifiersResponse;
  createTag(req: CreateTagRequest, call: strims_rpc_Call): Promise<CreateTagResponse> | CreateTagResponse;
  updateTag(req: UpdateTagRequest, call: strims_rpc_Call): Promise<UpdateTagResponse> | UpdateTagResponse;
  deleteTag(req: DeleteTagRequest, call: strims_rpc_Call): Promise<DeleteTagResponse> | DeleteTagResponse;
  getTag(req: GetTagRequest, call: strims_rpc_Call): Promise<GetTagResponse> | GetTagResponse;
  listTags(req: ListTagsRequest, call: strims_rpc_Call): Promise<ListTagsResponse> | ListTagsResponse;
  syncAssets(req: SyncAssetsRequest, call: strims_rpc_Call): Promise<SyncAssetsResponse> | SyncAssetsResponse;
}

export class UnimplementedChatServerFrontendService implements ChatServerFrontendService {
  createServer(req: CreateServerRequest, call: strims_rpc_Call): Promise<CreateServerResponse> | CreateServerResponse { throw new Error("not implemented"); }
  updateServer(req: UpdateServerRequest, call: strims_rpc_Call): Promise<UpdateServerResponse> | UpdateServerResponse { throw new Error("not implemented"); }
  deleteServer(req: DeleteServerRequest, call: strims_rpc_Call): Promise<DeleteServerResponse> | DeleteServerResponse { throw new Error("not implemented"); }
  getServer(req: GetServerRequest, call: strims_rpc_Call): Promise<GetServerResponse> | GetServerResponse { throw new Error("not implemented"); }
  listServers(req: ListServersRequest, call: strims_rpc_Call): Promise<ListServersResponse> | ListServersResponse { throw new Error("not implemented"); }
  createEmote(req: CreateEmoteRequest, call: strims_rpc_Call): Promise<CreateEmoteResponse> | CreateEmoteResponse { throw new Error("not implemented"); }
  updateEmote(req: UpdateEmoteRequest, call: strims_rpc_Call): Promise<UpdateEmoteResponse> | UpdateEmoteResponse { throw new Error("not implemented"); }
  deleteEmote(req: DeleteEmoteRequest, call: strims_rpc_Call): Promise<DeleteEmoteResponse> | DeleteEmoteResponse { throw new Error("not implemented"); }
  getEmote(req: GetEmoteRequest, call: strims_rpc_Call): Promise<GetEmoteResponse> | GetEmoteResponse { throw new Error("not implemented"); }
  listEmotes(req: ListEmotesRequest, call: strims_rpc_Call): Promise<ListEmotesResponse> | ListEmotesResponse { throw new Error("not implemented"); }
  createModifier(req: CreateModifierRequest, call: strims_rpc_Call): Promise<CreateModifierResponse> | CreateModifierResponse { throw new Error("not implemented"); }
  updateModifier(req: UpdateModifierRequest, call: strims_rpc_Call): Promise<UpdateModifierResponse> | UpdateModifierResponse { throw new Error("not implemented"); }
  deleteModifier(req: DeleteModifierRequest, call: strims_rpc_Call): Promise<DeleteModifierResponse> | DeleteModifierResponse { throw new Error("not implemented"); }
  getModifier(req: GetModifierRequest, call: strims_rpc_Call): Promise<GetModifierResponse> | GetModifierResponse { throw new Error("not implemented"); }
  listModifiers(req: ListModifiersRequest, call: strims_rpc_Call): Promise<ListModifiersResponse> | ListModifiersResponse { throw new Error("not implemented"); }
  createTag(req: CreateTagRequest, call: strims_rpc_Call): Promise<CreateTagResponse> | CreateTagResponse { throw new Error("not implemented"); }
  updateTag(req: UpdateTagRequest, call: strims_rpc_Call): Promise<UpdateTagResponse> | UpdateTagResponse { throw new Error("not implemented"); }
  deleteTag(req: DeleteTagRequest, call: strims_rpc_Call): Promise<DeleteTagResponse> | DeleteTagResponse { throw new Error("not implemented"); }
  getTag(req: GetTagRequest, call: strims_rpc_Call): Promise<GetTagResponse> | GetTagResponse { throw new Error("not implemented"); }
  listTags(req: ListTagsRequest, call: strims_rpc_Call): Promise<ListTagsResponse> | ListTagsResponse { throw new Error("not implemented"); }
  syncAssets(req: SyncAssetsRequest, call: strims_rpc_Call): Promise<SyncAssetsResponse> | SyncAssetsResponse { throw new Error("not implemented"); }
}

export const registerChatServerFrontendService = (host: strims_rpc_Service, service: ChatServerFrontendService): void => {
  host.registerMethod<CreateServerRequest, CreateServerResponse>("strims.chat.v1.ChatServerFrontend.CreateServer", service.createServer.bind(service), CreateServerRequest);
  host.registerMethod<UpdateServerRequest, UpdateServerResponse>("strims.chat.v1.ChatServerFrontend.UpdateServer", service.updateServer.bind(service), UpdateServerRequest);
  host.registerMethod<DeleteServerRequest, DeleteServerResponse>("strims.chat.v1.ChatServerFrontend.DeleteServer", service.deleteServer.bind(service), DeleteServerRequest);
  host.registerMethod<GetServerRequest, GetServerResponse>("strims.chat.v1.ChatServerFrontend.GetServer", service.getServer.bind(service), GetServerRequest);
  host.registerMethod<ListServersRequest, ListServersResponse>("strims.chat.v1.ChatServerFrontend.ListServers", service.listServers.bind(service), ListServersRequest);
  host.registerMethod<CreateEmoteRequest, CreateEmoteResponse>("strims.chat.v1.ChatServerFrontend.CreateEmote", service.createEmote.bind(service), CreateEmoteRequest);
  host.registerMethod<UpdateEmoteRequest, UpdateEmoteResponse>("strims.chat.v1.ChatServerFrontend.UpdateEmote", service.updateEmote.bind(service), UpdateEmoteRequest);
  host.registerMethod<DeleteEmoteRequest, DeleteEmoteResponse>("strims.chat.v1.ChatServerFrontend.DeleteEmote", service.deleteEmote.bind(service), DeleteEmoteRequest);
  host.registerMethod<GetEmoteRequest, GetEmoteResponse>("strims.chat.v1.ChatServerFrontend.GetEmote", service.getEmote.bind(service), GetEmoteRequest);
  host.registerMethod<ListEmotesRequest, ListEmotesResponse>("strims.chat.v1.ChatServerFrontend.ListEmotes", service.listEmotes.bind(service), ListEmotesRequest);
  host.registerMethod<CreateModifierRequest, CreateModifierResponse>("strims.chat.v1.ChatServerFrontend.CreateModifier", service.createModifier.bind(service), CreateModifierRequest);
  host.registerMethod<UpdateModifierRequest, UpdateModifierResponse>("strims.chat.v1.ChatServerFrontend.UpdateModifier", service.updateModifier.bind(service), UpdateModifierRequest);
  host.registerMethod<DeleteModifierRequest, DeleteModifierResponse>("strims.chat.v1.ChatServerFrontend.DeleteModifier", service.deleteModifier.bind(service), DeleteModifierRequest);
  host.registerMethod<GetModifierRequest, GetModifierResponse>("strims.chat.v1.ChatServerFrontend.GetModifier", service.getModifier.bind(service), GetModifierRequest);
  host.registerMethod<ListModifiersRequest, ListModifiersResponse>("strims.chat.v1.ChatServerFrontend.ListModifiers", service.listModifiers.bind(service), ListModifiersRequest);
  host.registerMethod<CreateTagRequest, CreateTagResponse>("strims.chat.v1.ChatServerFrontend.CreateTag", service.createTag.bind(service), CreateTagRequest);
  host.registerMethod<UpdateTagRequest, UpdateTagResponse>("strims.chat.v1.ChatServerFrontend.UpdateTag", service.updateTag.bind(service), UpdateTagRequest);
  host.registerMethod<DeleteTagRequest, DeleteTagResponse>("strims.chat.v1.ChatServerFrontend.DeleteTag", service.deleteTag.bind(service), DeleteTagRequest);
  host.registerMethod<GetTagRequest, GetTagResponse>("strims.chat.v1.ChatServerFrontend.GetTag", service.getTag.bind(service), GetTagRequest);
  host.registerMethod<ListTagsRequest, ListTagsResponse>("strims.chat.v1.ChatServerFrontend.ListTags", service.listTags.bind(service), ListTagsRequest);
  host.registerMethod<SyncAssetsRequest, SyncAssetsResponse>("strims.chat.v1.ChatServerFrontend.SyncAssets", service.syncAssets.bind(service), SyncAssetsRequest);
}

export class ChatServerFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createServer(req?: ICreateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateServer", new CreateServerRequest(req)), CreateServerResponse, opts);
  }

  public updateServer(req?: IUpdateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateServer", new UpdateServerRequest(req)), UpdateServerResponse, opts);
  }

  public deleteServer(req?: IDeleteServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteServer", new DeleteServerRequest(req)), DeleteServerResponse, opts);
  }

  public getServer(req?: IGetServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetServer", new GetServerRequest(req)), GetServerResponse, opts);
  }

  public listServers(req?: IListServersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListServersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListServers", new ListServersRequest(req)), ListServersResponse, opts);
  }

  public createEmote(req?: ICreateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateEmote", new CreateEmoteRequest(req)), CreateEmoteResponse, opts);
  }

  public updateEmote(req?: IUpdateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateEmote", new UpdateEmoteRequest(req)), UpdateEmoteResponse, opts);
  }

  public deleteEmote(req?: IDeleteEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteEmote", new DeleteEmoteRequest(req)), DeleteEmoteResponse, opts);
  }

  public getEmote(req?: IGetEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetEmote", new GetEmoteRequest(req)), GetEmoteResponse, opts);
  }

  public listEmotes(req?: IListEmotesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListEmotesResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListEmotes", new ListEmotesRequest(req)), ListEmotesResponse, opts);
  }

  public createModifier(req?: ICreateModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateModifier", new CreateModifierRequest(req)), CreateModifierResponse, opts);
  }

  public updateModifier(req?: IUpdateModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateModifier", new UpdateModifierRequest(req)), UpdateModifierResponse, opts);
  }

  public deleteModifier(req?: IDeleteModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteModifier", new DeleteModifierRequest(req)), DeleteModifierResponse, opts);
  }

  public getModifier(req?: IGetModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetModifier", new GetModifierRequest(req)), GetModifierResponse, opts);
  }

  public listModifiers(req?: IListModifiersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListModifiersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListModifiers", new ListModifiersRequest(req)), ListModifiersResponse, opts);
  }

  public createTag(req?: ICreateTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateTag", new CreateTagRequest(req)), CreateTagResponse, opts);
  }

  public updateTag(req?: IUpdateTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateTag", new UpdateTagRequest(req)), UpdateTagResponse, opts);
  }

  public deleteTag(req?: IDeleteTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteTag", new DeleteTagRequest(req)), DeleteTagResponse, opts);
  }

  public getTag(req?: IGetTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetTag", new GetTagRequest(req)), GetTagResponse, opts);
  }

  public listTags(req?: IListTagsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListTagsResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListTags", new ListTagsRequest(req)), ListTagsResponse, opts);
  }

  public syncAssets(req?: ISyncAssetsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SyncAssetsResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.SyncAssets", new SyncAssetsRequest(req)), SyncAssetsResponse, opts);
  }
}

export interface ChatFrontendService {
  openClient(req: OpenClientRequest, call: strims_rpc_Call): GenericReadable<OpenClientResponse>;
  clientSendMessage(req: ClientSendMessageRequest, call: strims_rpc_Call): Promise<ClientSendMessageResponse> | ClientSendMessageResponse;
  clientMute(req: ClientMuteRequest, call: strims_rpc_Call): Promise<ClientMuteResponse> | ClientMuteResponse;
  clientUnmute(req: ClientUnmuteRequest, call: strims_rpc_Call): Promise<ClientUnmuteResponse> | ClientUnmuteResponse;
  clientGetMute(req: ClientGetMuteRequest, call: strims_rpc_Call): Promise<ClientGetMuteResponse> | ClientGetMuteResponse;
  whisper(req: WhisperRequest, call: strims_rpc_Call): Promise<WhisperResponse> | WhisperResponse;
  setUIConfig(req: SetUIConfigRequest, call: strims_rpc_Call): Promise<SetUIConfigResponse> | SetUIConfigResponse;
  watchUIConfig(req: WatchUIConfigRequest, call: strims_rpc_Call): GenericReadable<WatchUIConfigResponse>;
  ignore(req: IgnoreRequest, call: strims_rpc_Call): Promise<IgnoreResponse> | IgnoreResponse;
  unignore(req: UnignoreRequest, call: strims_rpc_Call): Promise<UnignoreResponse> | UnignoreResponse;
  highlight(req: HighlightRequest, call: strims_rpc_Call): Promise<HighlightResponse> | HighlightResponse;
  unhighlight(req: UnhighlightRequest, call: strims_rpc_Call): Promise<UnhighlightResponse> | UnhighlightResponse;
  tag(req: TagRequest, call: strims_rpc_Call): Promise<TagResponse> | TagResponse;
  untag(req: UntagRequest, call: strims_rpc_Call): Promise<UntagResponse> | UntagResponse;
}

export class UnimplementedChatFrontendService implements ChatFrontendService {
  openClient(req: OpenClientRequest, call: strims_rpc_Call): GenericReadable<OpenClientResponse> { throw new Error("not implemented"); }
  clientSendMessage(req: ClientSendMessageRequest, call: strims_rpc_Call): Promise<ClientSendMessageResponse> | ClientSendMessageResponse { throw new Error("not implemented"); }
  clientMute(req: ClientMuteRequest, call: strims_rpc_Call): Promise<ClientMuteResponse> | ClientMuteResponse { throw new Error("not implemented"); }
  clientUnmute(req: ClientUnmuteRequest, call: strims_rpc_Call): Promise<ClientUnmuteResponse> | ClientUnmuteResponse { throw new Error("not implemented"); }
  clientGetMute(req: ClientGetMuteRequest, call: strims_rpc_Call): Promise<ClientGetMuteResponse> | ClientGetMuteResponse { throw new Error("not implemented"); }
  whisper(req: WhisperRequest, call: strims_rpc_Call): Promise<WhisperResponse> | WhisperResponse { throw new Error("not implemented"); }
  setUIConfig(req: SetUIConfigRequest, call: strims_rpc_Call): Promise<SetUIConfigResponse> | SetUIConfigResponse { throw new Error("not implemented"); }
  watchUIConfig(req: WatchUIConfigRequest, call: strims_rpc_Call): GenericReadable<WatchUIConfigResponse> { throw new Error("not implemented"); }
  ignore(req: IgnoreRequest, call: strims_rpc_Call): Promise<IgnoreResponse> | IgnoreResponse { throw new Error("not implemented"); }
  unignore(req: UnignoreRequest, call: strims_rpc_Call): Promise<UnignoreResponse> | UnignoreResponse { throw new Error("not implemented"); }
  highlight(req: HighlightRequest, call: strims_rpc_Call): Promise<HighlightResponse> | HighlightResponse { throw new Error("not implemented"); }
  unhighlight(req: UnhighlightRequest, call: strims_rpc_Call): Promise<UnhighlightResponse> | UnhighlightResponse { throw new Error("not implemented"); }
  tag(req: TagRequest, call: strims_rpc_Call): Promise<TagResponse> | TagResponse { throw new Error("not implemented"); }
  untag(req: UntagRequest, call: strims_rpc_Call): Promise<UntagResponse> | UntagResponse { throw new Error("not implemented"); }
}

export const registerChatFrontendService = (host: strims_rpc_Service, service: ChatFrontendService): void => {
  host.registerMethod<OpenClientRequest, OpenClientResponse>("strims.chat.v1.ChatFrontend.OpenClient", service.openClient.bind(service), OpenClientRequest);
  host.registerMethod<ClientSendMessageRequest, ClientSendMessageResponse>("strims.chat.v1.ChatFrontend.ClientSendMessage", service.clientSendMessage.bind(service), ClientSendMessageRequest);
  host.registerMethod<ClientMuteRequest, ClientMuteResponse>("strims.chat.v1.ChatFrontend.ClientMute", service.clientMute.bind(service), ClientMuteRequest);
  host.registerMethod<ClientUnmuteRequest, ClientUnmuteResponse>("strims.chat.v1.ChatFrontend.ClientUnmute", service.clientUnmute.bind(service), ClientUnmuteRequest);
  host.registerMethod<ClientGetMuteRequest, ClientGetMuteResponse>("strims.chat.v1.ChatFrontend.ClientGetMute", service.clientGetMute.bind(service), ClientGetMuteRequest);
  host.registerMethod<WhisperRequest, WhisperResponse>("strims.chat.v1.ChatFrontend.Whisper", service.whisper.bind(service), WhisperRequest);
  host.registerMethod<SetUIConfigRequest, SetUIConfigResponse>("strims.chat.v1.ChatFrontend.SetUIConfig", service.setUIConfig.bind(service), SetUIConfigRequest);
  host.registerMethod<WatchUIConfigRequest, WatchUIConfigResponse>("strims.chat.v1.ChatFrontend.WatchUIConfig", service.watchUIConfig.bind(service), WatchUIConfigRequest);
  host.registerMethod<IgnoreRequest, IgnoreResponse>("strims.chat.v1.ChatFrontend.Ignore", service.ignore.bind(service), IgnoreRequest);
  host.registerMethod<UnignoreRequest, UnignoreResponse>("strims.chat.v1.ChatFrontend.Unignore", service.unignore.bind(service), UnignoreRequest);
  host.registerMethod<HighlightRequest, HighlightResponse>("strims.chat.v1.ChatFrontend.Highlight", service.highlight.bind(service), HighlightRequest);
  host.registerMethod<UnhighlightRequest, UnhighlightResponse>("strims.chat.v1.ChatFrontend.Unhighlight", service.unhighlight.bind(service), UnhighlightRequest);
  host.registerMethod<TagRequest, TagResponse>("strims.chat.v1.ChatFrontend.Tag", service.tag.bind(service), TagRequest);
  host.registerMethod<UntagRequest, UntagResponse>("strims.chat.v1.ChatFrontend.Untag", service.untag.bind(service), UntagRequest);
}

export class ChatFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public openClient(req?: IOpenClientRequest): GenericReadable<OpenClientResponse> {
    return this.host.expectMany(this.host.call("strims.chat.v1.ChatFrontend.OpenClient", new OpenClientRequest(req)), OpenClientResponse);
  }

  public clientSendMessage(req?: IClientSendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ClientSendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientSendMessage", new ClientSendMessageRequest(req)), ClientSendMessageResponse, opts);
  }

  public clientMute(req?: IClientMuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ClientMuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientMute", new ClientMuteRequest(req)), ClientMuteResponse, opts);
  }

  public clientUnmute(req?: IClientUnmuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ClientUnmuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientUnmute", new ClientUnmuteRequest(req)), ClientUnmuteResponse, opts);
  }

  public clientGetMute(req?: IClientGetMuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ClientGetMuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientGetMute", new ClientGetMuteRequest(req)), ClientGetMuteResponse, opts);
  }

  public whisper(req?: IWhisperRequest, opts?: strims_rpc_UnaryCallOptions): Promise<WhisperResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Whisper", new WhisperRequest(req)), WhisperResponse, opts);
  }

  public setUIConfig(req?: ISetUIConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SetUIConfigResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.SetUIConfig", new SetUIConfigRequest(req)), SetUIConfigResponse, opts);
  }

  public watchUIConfig(req?: IWatchUIConfigRequest): GenericReadable<WatchUIConfigResponse> {
    return this.host.expectMany(this.host.call("strims.chat.v1.ChatFrontend.WatchUIConfig", new WatchUIConfigRequest(req)), WatchUIConfigResponse);
  }

  public ignore(req?: IIgnoreRequest, opts?: strims_rpc_UnaryCallOptions): Promise<IgnoreResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Ignore", new IgnoreRequest(req)), IgnoreResponse, opts);
  }

  public unignore(req?: IUnignoreRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UnignoreResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Unignore", new UnignoreRequest(req)), UnignoreResponse, opts);
  }

  public highlight(req?: IHighlightRequest, opts?: strims_rpc_UnaryCallOptions): Promise<HighlightResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Highlight", new HighlightRequest(req)), HighlightResponse, opts);
  }

  public unhighlight(req?: IUnhighlightRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UnhighlightResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Unhighlight", new UnhighlightRequest(req)), UnhighlightResponse, opts);
  }

  public tag(req?: ITagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<TagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Tag", new TagRequest(req)), TagResponse, opts);
  }

  public untag(req?: IUntagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UntagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Untag", new UntagRequest(req)), UntagResponse, opts);
  }
}

export interface ChatService {
  sendMessage(req: SendMessageRequest, call: strims_rpc_Call): Promise<SendMessageResponse> | SendMessageResponse;
  mute(req: MuteRequest, call: strims_rpc_Call): Promise<MuteResponse> | MuteResponse;
  unmute(req: UnmuteRequest, call: strims_rpc_Call): Promise<UnmuteResponse> | UnmuteResponse;
  getMute(req: GetMuteRequest, call: strims_rpc_Call): Promise<GetMuteResponse> | GetMuteResponse;
}

export class UnimplementedChatService implements ChatService {
  sendMessage(req: SendMessageRequest, call: strims_rpc_Call): Promise<SendMessageResponse> | SendMessageResponse { throw new Error("not implemented"); }
  mute(req: MuteRequest, call: strims_rpc_Call): Promise<MuteResponse> | MuteResponse { throw new Error("not implemented"); }
  unmute(req: UnmuteRequest, call: strims_rpc_Call): Promise<UnmuteResponse> | UnmuteResponse { throw new Error("not implemented"); }
  getMute(req: GetMuteRequest, call: strims_rpc_Call): Promise<GetMuteResponse> | GetMuteResponse { throw new Error("not implemented"); }
}

export const registerChatService = (host: strims_rpc_Service, service: ChatService): void => {
  host.registerMethod<SendMessageRequest, SendMessageResponse>("strims.chat.v1.Chat.SendMessage", service.sendMessage.bind(service), SendMessageRequest);
  host.registerMethod<MuteRequest, MuteResponse>("strims.chat.v1.Chat.Mute", service.mute.bind(service), MuteRequest);
  host.registerMethod<UnmuteRequest, UnmuteResponse>("strims.chat.v1.Chat.Unmute", service.unmute.bind(service), UnmuteRequest);
  host.registerMethod<GetMuteRequest, GetMuteResponse>("strims.chat.v1.Chat.GetMute", service.getMute.bind(service), GetMuteRequest);
}

export class ChatClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public sendMessage(req?: ISendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.SendMessage", new SendMessageRequest(req)), SendMessageResponse, opts);
  }

  public mute(req?: IMuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<MuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.Mute", new MuteRequest(req)), MuteResponse, opts);
  }

  public unmute(req?: IUnmuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UnmuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.Unmute", new UnmuteRequest(req)), UnmuteResponse, opts);
  }

  public getMute(req?: IGetMuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetMuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.GetMute", new GetMuteRequest(req)), GetMuteResponse, opts);
  }
}

export interface WhisperService {
  sendMessage(req: WhisperSendMessageRequest, call: strims_rpc_Call): Promise<WhisperSendMessageResponse> | WhisperSendMessageResponse;
}

export class UnimplementedWhisperService implements WhisperService {
  sendMessage(req: WhisperSendMessageRequest, call: strims_rpc_Call): Promise<WhisperSendMessageResponse> | WhisperSendMessageResponse { throw new Error("not implemented"); }
}

export const registerWhisperService = (host: strims_rpc_Service, service: WhisperService): void => {
  host.registerMethod<WhisperSendMessageRequest, WhisperSendMessageResponse>("strims.chat.v1.Whisper.SendMessage", service.sendMessage.bind(service), WhisperSendMessageRequest);
}

export class WhisperClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public sendMessage(req?: IWhisperSendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<WhisperSendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Whisper.SendMessage", new WhisperSendMessageRequest(req)), WhisperSendMessageResponse, opts);
  }
}

