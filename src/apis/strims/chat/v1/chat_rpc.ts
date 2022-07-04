import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  strims_chat_v1_ICreateServerRequest,
  strims_chat_v1_CreateServerRequest,
  strims_chat_v1_CreateServerResponse,
  strims_chat_v1_IUpdateServerRequest,
  strims_chat_v1_UpdateServerRequest,
  strims_chat_v1_UpdateServerResponse,
  strims_chat_v1_IDeleteServerRequest,
  strims_chat_v1_DeleteServerRequest,
  strims_chat_v1_DeleteServerResponse,
  strims_chat_v1_IGetServerRequest,
  strims_chat_v1_GetServerRequest,
  strims_chat_v1_GetServerResponse,
  strims_chat_v1_IListServersRequest,
  strims_chat_v1_ListServersRequest,
  strims_chat_v1_ListServersResponse,
  strims_chat_v1_ICreateEmoteRequest,
  strims_chat_v1_CreateEmoteRequest,
  strims_chat_v1_CreateEmoteResponse,
  strims_chat_v1_IUpdateEmoteRequest,
  strims_chat_v1_UpdateEmoteRequest,
  strims_chat_v1_UpdateEmoteResponse,
  strims_chat_v1_IDeleteEmoteRequest,
  strims_chat_v1_DeleteEmoteRequest,
  strims_chat_v1_DeleteEmoteResponse,
  strims_chat_v1_IGetEmoteRequest,
  strims_chat_v1_GetEmoteRequest,
  strims_chat_v1_GetEmoteResponse,
  strims_chat_v1_IListEmotesRequest,
  strims_chat_v1_ListEmotesRequest,
  strims_chat_v1_ListEmotesResponse,
  strims_chat_v1_ICreateModifierRequest,
  strims_chat_v1_CreateModifierRequest,
  strims_chat_v1_CreateModifierResponse,
  strims_chat_v1_IUpdateModifierRequest,
  strims_chat_v1_UpdateModifierRequest,
  strims_chat_v1_UpdateModifierResponse,
  strims_chat_v1_IDeleteModifierRequest,
  strims_chat_v1_DeleteModifierRequest,
  strims_chat_v1_DeleteModifierResponse,
  strims_chat_v1_IGetModifierRequest,
  strims_chat_v1_GetModifierRequest,
  strims_chat_v1_GetModifierResponse,
  strims_chat_v1_IListModifiersRequest,
  strims_chat_v1_ListModifiersRequest,
  strims_chat_v1_ListModifiersResponse,
  strims_chat_v1_ICreateTagRequest,
  strims_chat_v1_CreateTagRequest,
  strims_chat_v1_CreateTagResponse,
  strims_chat_v1_IUpdateTagRequest,
  strims_chat_v1_UpdateTagRequest,
  strims_chat_v1_UpdateTagResponse,
  strims_chat_v1_IDeleteTagRequest,
  strims_chat_v1_DeleteTagRequest,
  strims_chat_v1_DeleteTagResponse,
  strims_chat_v1_IGetTagRequest,
  strims_chat_v1_GetTagRequest,
  strims_chat_v1_GetTagResponse,
  strims_chat_v1_IListTagsRequest,
  strims_chat_v1_ListTagsRequest,
  strims_chat_v1_ListTagsResponse,
  strims_chat_v1_ISyncAssetsRequest,
  strims_chat_v1_SyncAssetsRequest,
  strims_chat_v1_SyncAssetsResponse,
  strims_chat_v1_IOpenClientRequest,
  strims_chat_v1_OpenClientRequest,
  strims_chat_v1_OpenClientResponse,
  strims_chat_v1_IClientSendMessageRequest,
  strims_chat_v1_ClientSendMessageRequest,
  strims_chat_v1_ClientSendMessageResponse,
  strims_chat_v1_IClientMuteRequest,
  strims_chat_v1_ClientMuteRequest,
  strims_chat_v1_ClientMuteResponse,
  strims_chat_v1_IClientUnmuteRequest,
  strims_chat_v1_ClientUnmuteRequest,
  strims_chat_v1_ClientUnmuteResponse,
  strims_chat_v1_IClientGetMuteRequest,
  strims_chat_v1_ClientGetMuteRequest,
  strims_chat_v1_ClientGetMuteResponse,
  strims_chat_v1_IWhisperRequest,
  strims_chat_v1_WhisperRequest,
  strims_chat_v1_WhisperResponse,
  strims_chat_v1_IListWhispersRequest,
  strims_chat_v1_ListWhispersRequest,
  strims_chat_v1_ListWhispersResponse,
  strims_chat_v1_IWatchWhispersRequest,
  strims_chat_v1_WatchWhispersRequest,
  strims_chat_v1_WatchWhispersResponse,
  strims_chat_v1_IMarkWhispersReadRequest,
  strims_chat_v1_MarkWhispersReadRequest,
  strims_chat_v1_MarkWhispersReadResponse,
  strims_chat_v1_ISetUIConfigRequest,
  strims_chat_v1_SetUIConfigRequest,
  strims_chat_v1_SetUIConfigResponse,
  strims_chat_v1_IWatchUIConfigRequest,
  strims_chat_v1_WatchUIConfigRequest,
  strims_chat_v1_WatchUIConfigResponse,
  strims_chat_v1_IIgnoreRequest,
  strims_chat_v1_IgnoreRequest,
  strims_chat_v1_IgnoreResponse,
  strims_chat_v1_IUnignoreRequest,
  strims_chat_v1_UnignoreRequest,
  strims_chat_v1_UnignoreResponse,
  strims_chat_v1_IHighlightRequest,
  strims_chat_v1_HighlightRequest,
  strims_chat_v1_HighlightResponse,
  strims_chat_v1_IUnhighlightRequest,
  strims_chat_v1_UnhighlightRequest,
  strims_chat_v1_UnhighlightResponse,
  strims_chat_v1_ITagRequest,
  strims_chat_v1_TagRequest,
  strims_chat_v1_TagResponse,
  strims_chat_v1_IUntagRequest,
  strims_chat_v1_UntagRequest,
  strims_chat_v1_UntagResponse,
  strims_chat_v1_IGetEmojiRequest,
  strims_chat_v1_GetEmojiRequest,
  strims_chat_v1_GetEmojiResponse,
  strims_chat_v1_ISendMessageRequest,
  strims_chat_v1_SendMessageRequest,
  strims_chat_v1_SendMessageResponse,
  strims_chat_v1_IMuteRequest,
  strims_chat_v1_MuteRequest,
  strims_chat_v1_MuteResponse,
  strims_chat_v1_IUnmuteRequest,
  strims_chat_v1_UnmuteRequest,
  strims_chat_v1_UnmuteResponse,
  strims_chat_v1_IGetMuteRequest,
  strims_chat_v1_GetMuteRequest,
  strims_chat_v1_GetMuteResponse,
  strims_chat_v1_IWhisperSendMessageRequest,
  strims_chat_v1_WhisperSendMessageRequest,
  strims_chat_v1_WhisperSendMessageResponse,
} from "./chat";

export interface ChatServerFrontendService {
  createServer(req: strims_chat_v1_CreateServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateServerResponse> | strims_chat_v1_CreateServerResponse;
  updateServer(req: strims_chat_v1_UpdateServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateServerResponse> | strims_chat_v1_UpdateServerResponse;
  deleteServer(req: strims_chat_v1_DeleteServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteServerResponse> | strims_chat_v1_DeleteServerResponse;
  getServer(req: strims_chat_v1_GetServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetServerResponse> | strims_chat_v1_GetServerResponse;
  listServers(req: strims_chat_v1_ListServersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListServersResponse> | strims_chat_v1_ListServersResponse;
  createEmote(req: strims_chat_v1_CreateEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateEmoteResponse> | strims_chat_v1_CreateEmoteResponse;
  updateEmote(req: strims_chat_v1_UpdateEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateEmoteResponse> | strims_chat_v1_UpdateEmoteResponse;
  deleteEmote(req: strims_chat_v1_DeleteEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteEmoteResponse> | strims_chat_v1_DeleteEmoteResponse;
  getEmote(req: strims_chat_v1_GetEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetEmoteResponse> | strims_chat_v1_GetEmoteResponse;
  listEmotes(req: strims_chat_v1_ListEmotesRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListEmotesResponse> | strims_chat_v1_ListEmotesResponse;
  createModifier(req: strims_chat_v1_CreateModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateModifierResponse> | strims_chat_v1_CreateModifierResponse;
  updateModifier(req: strims_chat_v1_UpdateModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateModifierResponse> | strims_chat_v1_UpdateModifierResponse;
  deleteModifier(req: strims_chat_v1_DeleteModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteModifierResponse> | strims_chat_v1_DeleteModifierResponse;
  getModifier(req: strims_chat_v1_GetModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetModifierResponse> | strims_chat_v1_GetModifierResponse;
  listModifiers(req: strims_chat_v1_ListModifiersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListModifiersResponse> | strims_chat_v1_ListModifiersResponse;
  createTag(req: strims_chat_v1_CreateTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateTagResponse> | strims_chat_v1_CreateTagResponse;
  updateTag(req: strims_chat_v1_UpdateTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateTagResponse> | strims_chat_v1_UpdateTagResponse;
  deleteTag(req: strims_chat_v1_DeleteTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteTagResponse> | strims_chat_v1_DeleteTagResponse;
  getTag(req: strims_chat_v1_GetTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetTagResponse> | strims_chat_v1_GetTagResponse;
  listTags(req: strims_chat_v1_ListTagsRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListTagsResponse> | strims_chat_v1_ListTagsResponse;
  syncAssets(req: strims_chat_v1_SyncAssetsRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SyncAssetsResponse> | strims_chat_v1_SyncAssetsResponse;
}

export class UnimplementedChatServerFrontendService implements ChatServerFrontendService {
  createServer(req: strims_chat_v1_CreateServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateServerResponse> | strims_chat_v1_CreateServerResponse { throw new Error("not implemented"); }
  updateServer(req: strims_chat_v1_UpdateServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateServerResponse> | strims_chat_v1_UpdateServerResponse { throw new Error("not implemented"); }
  deleteServer(req: strims_chat_v1_DeleteServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteServerResponse> | strims_chat_v1_DeleteServerResponse { throw new Error("not implemented"); }
  getServer(req: strims_chat_v1_GetServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetServerResponse> | strims_chat_v1_GetServerResponse { throw new Error("not implemented"); }
  listServers(req: strims_chat_v1_ListServersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListServersResponse> | strims_chat_v1_ListServersResponse { throw new Error("not implemented"); }
  createEmote(req: strims_chat_v1_CreateEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateEmoteResponse> | strims_chat_v1_CreateEmoteResponse { throw new Error("not implemented"); }
  updateEmote(req: strims_chat_v1_UpdateEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateEmoteResponse> | strims_chat_v1_UpdateEmoteResponse { throw new Error("not implemented"); }
  deleteEmote(req: strims_chat_v1_DeleteEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteEmoteResponse> | strims_chat_v1_DeleteEmoteResponse { throw new Error("not implemented"); }
  getEmote(req: strims_chat_v1_GetEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetEmoteResponse> | strims_chat_v1_GetEmoteResponse { throw new Error("not implemented"); }
  listEmotes(req: strims_chat_v1_ListEmotesRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListEmotesResponse> | strims_chat_v1_ListEmotesResponse { throw new Error("not implemented"); }
  createModifier(req: strims_chat_v1_CreateModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateModifierResponse> | strims_chat_v1_CreateModifierResponse { throw new Error("not implemented"); }
  updateModifier(req: strims_chat_v1_UpdateModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateModifierResponse> | strims_chat_v1_UpdateModifierResponse { throw new Error("not implemented"); }
  deleteModifier(req: strims_chat_v1_DeleteModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteModifierResponse> | strims_chat_v1_DeleteModifierResponse { throw new Error("not implemented"); }
  getModifier(req: strims_chat_v1_GetModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetModifierResponse> | strims_chat_v1_GetModifierResponse { throw new Error("not implemented"); }
  listModifiers(req: strims_chat_v1_ListModifiersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListModifiersResponse> | strims_chat_v1_ListModifiersResponse { throw new Error("not implemented"); }
  createTag(req: strims_chat_v1_CreateTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateTagResponse> | strims_chat_v1_CreateTagResponse { throw new Error("not implemented"); }
  updateTag(req: strims_chat_v1_UpdateTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateTagResponse> | strims_chat_v1_UpdateTagResponse { throw new Error("not implemented"); }
  deleteTag(req: strims_chat_v1_DeleteTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteTagResponse> | strims_chat_v1_DeleteTagResponse { throw new Error("not implemented"); }
  getTag(req: strims_chat_v1_GetTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetTagResponse> | strims_chat_v1_GetTagResponse { throw new Error("not implemented"); }
  listTags(req: strims_chat_v1_ListTagsRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListTagsResponse> | strims_chat_v1_ListTagsResponse { throw new Error("not implemented"); }
  syncAssets(req: strims_chat_v1_SyncAssetsRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SyncAssetsResponse> | strims_chat_v1_SyncAssetsResponse { throw new Error("not implemented"); }
}

export const registerChatServerFrontendService = (host: strims_rpc_Service, service: ChatServerFrontendService): void => {
  host.registerMethod<strims_chat_v1_CreateServerRequest, strims_chat_v1_CreateServerResponse>("strims.chat.v1.ChatServerFrontend.CreateServer", service.createServer.bind(service), strims_chat_v1_CreateServerRequest);
  host.registerMethod<strims_chat_v1_UpdateServerRequest, strims_chat_v1_UpdateServerResponse>("strims.chat.v1.ChatServerFrontend.UpdateServer", service.updateServer.bind(service), strims_chat_v1_UpdateServerRequest);
  host.registerMethod<strims_chat_v1_DeleteServerRequest, strims_chat_v1_DeleteServerResponse>("strims.chat.v1.ChatServerFrontend.DeleteServer", service.deleteServer.bind(service), strims_chat_v1_DeleteServerRequest);
  host.registerMethod<strims_chat_v1_GetServerRequest, strims_chat_v1_GetServerResponse>("strims.chat.v1.ChatServerFrontend.GetServer", service.getServer.bind(service), strims_chat_v1_GetServerRequest);
  host.registerMethod<strims_chat_v1_ListServersRequest, strims_chat_v1_ListServersResponse>("strims.chat.v1.ChatServerFrontend.ListServers", service.listServers.bind(service), strims_chat_v1_ListServersRequest);
  host.registerMethod<strims_chat_v1_CreateEmoteRequest, strims_chat_v1_CreateEmoteResponse>("strims.chat.v1.ChatServerFrontend.CreateEmote", service.createEmote.bind(service), strims_chat_v1_CreateEmoteRequest);
  host.registerMethod<strims_chat_v1_UpdateEmoteRequest, strims_chat_v1_UpdateEmoteResponse>("strims.chat.v1.ChatServerFrontend.UpdateEmote", service.updateEmote.bind(service), strims_chat_v1_UpdateEmoteRequest);
  host.registerMethod<strims_chat_v1_DeleteEmoteRequest, strims_chat_v1_DeleteEmoteResponse>("strims.chat.v1.ChatServerFrontend.DeleteEmote", service.deleteEmote.bind(service), strims_chat_v1_DeleteEmoteRequest);
  host.registerMethod<strims_chat_v1_GetEmoteRequest, strims_chat_v1_GetEmoteResponse>("strims.chat.v1.ChatServerFrontend.GetEmote", service.getEmote.bind(service), strims_chat_v1_GetEmoteRequest);
  host.registerMethod<strims_chat_v1_ListEmotesRequest, strims_chat_v1_ListEmotesResponse>("strims.chat.v1.ChatServerFrontend.ListEmotes", service.listEmotes.bind(service), strims_chat_v1_ListEmotesRequest);
  host.registerMethod<strims_chat_v1_CreateModifierRequest, strims_chat_v1_CreateModifierResponse>("strims.chat.v1.ChatServerFrontend.CreateModifier", service.createModifier.bind(service), strims_chat_v1_CreateModifierRequest);
  host.registerMethod<strims_chat_v1_UpdateModifierRequest, strims_chat_v1_UpdateModifierResponse>("strims.chat.v1.ChatServerFrontend.UpdateModifier", service.updateModifier.bind(service), strims_chat_v1_UpdateModifierRequest);
  host.registerMethod<strims_chat_v1_DeleteModifierRequest, strims_chat_v1_DeleteModifierResponse>("strims.chat.v1.ChatServerFrontend.DeleteModifier", service.deleteModifier.bind(service), strims_chat_v1_DeleteModifierRequest);
  host.registerMethod<strims_chat_v1_GetModifierRequest, strims_chat_v1_GetModifierResponse>("strims.chat.v1.ChatServerFrontend.GetModifier", service.getModifier.bind(service), strims_chat_v1_GetModifierRequest);
  host.registerMethod<strims_chat_v1_ListModifiersRequest, strims_chat_v1_ListModifiersResponse>("strims.chat.v1.ChatServerFrontend.ListModifiers", service.listModifiers.bind(service), strims_chat_v1_ListModifiersRequest);
  host.registerMethod<strims_chat_v1_CreateTagRequest, strims_chat_v1_CreateTagResponse>("strims.chat.v1.ChatServerFrontend.CreateTag", service.createTag.bind(service), strims_chat_v1_CreateTagRequest);
  host.registerMethod<strims_chat_v1_UpdateTagRequest, strims_chat_v1_UpdateTagResponse>("strims.chat.v1.ChatServerFrontend.UpdateTag", service.updateTag.bind(service), strims_chat_v1_UpdateTagRequest);
  host.registerMethod<strims_chat_v1_DeleteTagRequest, strims_chat_v1_DeleteTagResponse>("strims.chat.v1.ChatServerFrontend.DeleteTag", service.deleteTag.bind(service), strims_chat_v1_DeleteTagRequest);
  host.registerMethod<strims_chat_v1_GetTagRequest, strims_chat_v1_GetTagResponse>("strims.chat.v1.ChatServerFrontend.GetTag", service.getTag.bind(service), strims_chat_v1_GetTagRequest);
  host.registerMethod<strims_chat_v1_ListTagsRequest, strims_chat_v1_ListTagsResponse>("strims.chat.v1.ChatServerFrontend.ListTags", service.listTags.bind(service), strims_chat_v1_ListTagsRequest);
  host.registerMethod<strims_chat_v1_SyncAssetsRequest, strims_chat_v1_SyncAssetsResponse>("strims.chat.v1.ChatServerFrontend.SyncAssets", service.syncAssets.bind(service), strims_chat_v1_SyncAssetsRequest);
}

export class ChatServerFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createServer(req?: strims_chat_v1_ICreateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_CreateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateServer", new strims_chat_v1_CreateServerRequest(req)), strims_chat_v1_CreateServerResponse, opts);
  }

  public updateServer(req?: strims_chat_v1_IUpdateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UpdateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateServer", new strims_chat_v1_UpdateServerRequest(req)), strims_chat_v1_UpdateServerResponse, opts);
  }

  public deleteServer(req?: strims_chat_v1_IDeleteServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_DeleteServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteServer", new strims_chat_v1_DeleteServerRequest(req)), strims_chat_v1_DeleteServerResponse, opts);
  }

  public getServer(req?: strims_chat_v1_IGetServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_GetServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetServer", new strims_chat_v1_GetServerRequest(req)), strims_chat_v1_GetServerResponse, opts);
  }

  public listServers(req?: strims_chat_v1_IListServersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ListServersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListServers", new strims_chat_v1_ListServersRequest(req)), strims_chat_v1_ListServersResponse, opts);
  }

  public createEmote(req?: strims_chat_v1_ICreateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_CreateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateEmote", new strims_chat_v1_CreateEmoteRequest(req)), strims_chat_v1_CreateEmoteResponse, opts);
  }

  public updateEmote(req?: strims_chat_v1_IUpdateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UpdateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateEmote", new strims_chat_v1_UpdateEmoteRequest(req)), strims_chat_v1_UpdateEmoteResponse, opts);
  }

  public deleteEmote(req?: strims_chat_v1_IDeleteEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_DeleteEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteEmote", new strims_chat_v1_DeleteEmoteRequest(req)), strims_chat_v1_DeleteEmoteResponse, opts);
  }

  public getEmote(req?: strims_chat_v1_IGetEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_GetEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetEmote", new strims_chat_v1_GetEmoteRequest(req)), strims_chat_v1_GetEmoteResponse, opts);
  }

  public listEmotes(req?: strims_chat_v1_IListEmotesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ListEmotesResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListEmotes", new strims_chat_v1_ListEmotesRequest(req)), strims_chat_v1_ListEmotesResponse, opts);
  }

  public createModifier(req?: strims_chat_v1_ICreateModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_CreateModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateModifier", new strims_chat_v1_CreateModifierRequest(req)), strims_chat_v1_CreateModifierResponse, opts);
  }

  public updateModifier(req?: strims_chat_v1_IUpdateModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UpdateModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateModifier", new strims_chat_v1_UpdateModifierRequest(req)), strims_chat_v1_UpdateModifierResponse, opts);
  }

  public deleteModifier(req?: strims_chat_v1_IDeleteModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_DeleteModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteModifier", new strims_chat_v1_DeleteModifierRequest(req)), strims_chat_v1_DeleteModifierResponse, opts);
  }

  public getModifier(req?: strims_chat_v1_IGetModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_GetModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetModifier", new strims_chat_v1_GetModifierRequest(req)), strims_chat_v1_GetModifierResponse, opts);
  }

  public listModifiers(req?: strims_chat_v1_IListModifiersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ListModifiersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListModifiers", new strims_chat_v1_ListModifiersRequest(req)), strims_chat_v1_ListModifiersResponse, opts);
  }

  public createTag(req?: strims_chat_v1_ICreateTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_CreateTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateTag", new strims_chat_v1_CreateTagRequest(req)), strims_chat_v1_CreateTagResponse, opts);
  }

  public updateTag(req?: strims_chat_v1_IUpdateTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UpdateTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateTag", new strims_chat_v1_UpdateTagRequest(req)), strims_chat_v1_UpdateTagResponse, opts);
  }

  public deleteTag(req?: strims_chat_v1_IDeleteTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_DeleteTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteTag", new strims_chat_v1_DeleteTagRequest(req)), strims_chat_v1_DeleteTagResponse, opts);
  }

  public getTag(req?: strims_chat_v1_IGetTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_GetTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetTag", new strims_chat_v1_GetTagRequest(req)), strims_chat_v1_GetTagResponse, opts);
  }

  public listTags(req?: strims_chat_v1_IListTagsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ListTagsResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListTags", new strims_chat_v1_ListTagsRequest(req)), strims_chat_v1_ListTagsResponse, opts);
  }

  public syncAssets(req?: strims_chat_v1_ISyncAssetsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_SyncAssetsResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.SyncAssets", new strims_chat_v1_SyncAssetsRequest(req)), strims_chat_v1_SyncAssetsResponse, opts);
  }
}

export interface ChatFrontendService {
  openClient(req: strims_chat_v1_OpenClientRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_OpenClientResponse>;
  clientSendMessage(req: strims_chat_v1_ClientSendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientSendMessageResponse> | strims_chat_v1_ClientSendMessageResponse;
  clientMute(req: strims_chat_v1_ClientMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientMuteResponse> | strims_chat_v1_ClientMuteResponse;
  clientUnmute(req: strims_chat_v1_ClientUnmuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientUnmuteResponse> | strims_chat_v1_ClientUnmuteResponse;
  clientGetMute(req: strims_chat_v1_ClientGetMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientGetMuteResponse> | strims_chat_v1_ClientGetMuteResponse;
  whisper(req: strims_chat_v1_WhisperRequest, call: strims_rpc_Call): Promise<strims_chat_v1_WhisperResponse> | strims_chat_v1_WhisperResponse;
  listWhispers(req: strims_chat_v1_ListWhispersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListWhispersResponse> | strims_chat_v1_ListWhispersResponse;
  watchWhispers(req: strims_chat_v1_WatchWhispersRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_WatchWhispersResponse>;
  markWhispersRead(req: strims_chat_v1_MarkWhispersReadRequest, call: strims_rpc_Call): Promise<strims_chat_v1_MarkWhispersReadResponse> | strims_chat_v1_MarkWhispersReadResponse;
  setUIConfig(req: strims_chat_v1_SetUIConfigRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SetUIConfigResponse> | strims_chat_v1_SetUIConfigResponse;
  watchUIConfig(req: strims_chat_v1_WatchUIConfigRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_WatchUIConfigResponse>;
  ignore(req: strims_chat_v1_IgnoreRequest, call: strims_rpc_Call): Promise<strims_chat_v1_IgnoreResponse> | strims_chat_v1_IgnoreResponse;
  unignore(req: strims_chat_v1_UnignoreRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnignoreResponse> | strims_chat_v1_UnignoreResponse;
  highlight(req: strims_chat_v1_HighlightRequest, call: strims_rpc_Call): Promise<strims_chat_v1_HighlightResponse> | strims_chat_v1_HighlightResponse;
  unhighlight(req: strims_chat_v1_UnhighlightRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnhighlightResponse> | strims_chat_v1_UnhighlightResponse;
  tag(req: strims_chat_v1_TagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_TagResponse> | strims_chat_v1_TagResponse;
  untag(req: strims_chat_v1_UntagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UntagResponse> | strims_chat_v1_UntagResponse;
  getEmoji(req: strims_chat_v1_GetEmojiRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetEmojiResponse> | strims_chat_v1_GetEmojiResponse;
}

export class UnimplementedChatFrontendService implements ChatFrontendService {
  openClient(req: strims_chat_v1_OpenClientRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_OpenClientResponse> { throw new Error("not implemented"); }
  clientSendMessage(req: strims_chat_v1_ClientSendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientSendMessageResponse> | strims_chat_v1_ClientSendMessageResponse { throw new Error("not implemented"); }
  clientMute(req: strims_chat_v1_ClientMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientMuteResponse> | strims_chat_v1_ClientMuteResponse { throw new Error("not implemented"); }
  clientUnmute(req: strims_chat_v1_ClientUnmuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientUnmuteResponse> | strims_chat_v1_ClientUnmuteResponse { throw new Error("not implemented"); }
  clientGetMute(req: strims_chat_v1_ClientGetMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientGetMuteResponse> | strims_chat_v1_ClientGetMuteResponse { throw new Error("not implemented"); }
  whisper(req: strims_chat_v1_WhisperRequest, call: strims_rpc_Call): Promise<strims_chat_v1_WhisperResponse> | strims_chat_v1_WhisperResponse { throw new Error("not implemented"); }
  listWhispers(req: strims_chat_v1_ListWhispersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListWhispersResponse> | strims_chat_v1_ListWhispersResponse { throw new Error("not implemented"); }
  watchWhispers(req: strims_chat_v1_WatchWhispersRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_WatchWhispersResponse> { throw new Error("not implemented"); }
  markWhispersRead(req: strims_chat_v1_MarkWhispersReadRequest, call: strims_rpc_Call): Promise<strims_chat_v1_MarkWhispersReadResponse> | strims_chat_v1_MarkWhispersReadResponse { throw new Error("not implemented"); }
  setUIConfig(req: strims_chat_v1_SetUIConfigRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SetUIConfigResponse> | strims_chat_v1_SetUIConfigResponse { throw new Error("not implemented"); }
  watchUIConfig(req: strims_chat_v1_WatchUIConfigRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_WatchUIConfigResponse> { throw new Error("not implemented"); }
  ignore(req: strims_chat_v1_IgnoreRequest, call: strims_rpc_Call): Promise<strims_chat_v1_IgnoreResponse> | strims_chat_v1_IgnoreResponse { throw new Error("not implemented"); }
  unignore(req: strims_chat_v1_UnignoreRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnignoreResponse> | strims_chat_v1_UnignoreResponse { throw new Error("not implemented"); }
  highlight(req: strims_chat_v1_HighlightRequest, call: strims_rpc_Call): Promise<strims_chat_v1_HighlightResponse> | strims_chat_v1_HighlightResponse { throw new Error("not implemented"); }
  unhighlight(req: strims_chat_v1_UnhighlightRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnhighlightResponse> | strims_chat_v1_UnhighlightResponse { throw new Error("not implemented"); }
  tag(req: strims_chat_v1_TagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_TagResponse> | strims_chat_v1_TagResponse { throw new Error("not implemented"); }
  untag(req: strims_chat_v1_UntagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UntagResponse> | strims_chat_v1_UntagResponse { throw new Error("not implemented"); }
  getEmoji(req: strims_chat_v1_GetEmojiRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetEmojiResponse> | strims_chat_v1_GetEmojiResponse { throw new Error("not implemented"); }
}

export const registerChatFrontendService = (host: strims_rpc_Service, service: ChatFrontendService): void => {
  host.registerMethod<strims_chat_v1_OpenClientRequest, strims_chat_v1_OpenClientResponse>("strims.chat.v1.ChatFrontend.OpenClient", service.openClient.bind(service), strims_chat_v1_OpenClientRequest);
  host.registerMethod<strims_chat_v1_ClientSendMessageRequest, strims_chat_v1_ClientSendMessageResponse>("strims.chat.v1.ChatFrontend.ClientSendMessage", service.clientSendMessage.bind(service), strims_chat_v1_ClientSendMessageRequest);
  host.registerMethod<strims_chat_v1_ClientMuteRequest, strims_chat_v1_ClientMuteResponse>("strims.chat.v1.ChatFrontend.ClientMute", service.clientMute.bind(service), strims_chat_v1_ClientMuteRequest);
  host.registerMethod<strims_chat_v1_ClientUnmuteRequest, strims_chat_v1_ClientUnmuteResponse>("strims.chat.v1.ChatFrontend.ClientUnmute", service.clientUnmute.bind(service), strims_chat_v1_ClientUnmuteRequest);
  host.registerMethod<strims_chat_v1_ClientGetMuteRequest, strims_chat_v1_ClientGetMuteResponse>("strims.chat.v1.ChatFrontend.ClientGetMute", service.clientGetMute.bind(service), strims_chat_v1_ClientGetMuteRequest);
  host.registerMethod<strims_chat_v1_WhisperRequest, strims_chat_v1_WhisperResponse>("strims.chat.v1.ChatFrontend.Whisper", service.whisper.bind(service), strims_chat_v1_WhisperRequest);
  host.registerMethod<strims_chat_v1_ListWhispersRequest, strims_chat_v1_ListWhispersResponse>("strims.chat.v1.ChatFrontend.ListWhispers", service.listWhispers.bind(service), strims_chat_v1_ListWhispersRequest);
  host.registerMethod<strims_chat_v1_WatchWhispersRequest, strims_chat_v1_WatchWhispersResponse>("strims.chat.v1.ChatFrontend.WatchWhispers", service.watchWhispers.bind(service), strims_chat_v1_WatchWhispersRequest);
  host.registerMethod<strims_chat_v1_MarkWhispersReadRequest, strims_chat_v1_MarkWhispersReadResponse>("strims.chat.v1.ChatFrontend.MarkWhispersRead", service.markWhispersRead.bind(service), strims_chat_v1_MarkWhispersReadRequest);
  host.registerMethod<strims_chat_v1_SetUIConfigRequest, strims_chat_v1_SetUIConfigResponse>("strims.chat.v1.ChatFrontend.SetUIConfig", service.setUIConfig.bind(service), strims_chat_v1_SetUIConfigRequest);
  host.registerMethod<strims_chat_v1_WatchUIConfigRequest, strims_chat_v1_WatchUIConfigResponse>("strims.chat.v1.ChatFrontend.WatchUIConfig", service.watchUIConfig.bind(service), strims_chat_v1_WatchUIConfigRequest);
  host.registerMethod<strims_chat_v1_IgnoreRequest, strims_chat_v1_IgnoreResponse>("strims.chat.v1.ChatFrontend.Ignore", service.ignore.bind(service), strims_chat_v1_IgnoreRequest);
  host.registerMethod<strims_chat_v1_UnignoreRequest, strims_chat_v1_UnignoreResponse>("strims.chat.v1.ChatFrontend.Unignore", service.unignore.bind(service), strims_chat_v1_UnignoreRequest);
  host.registerMethod<strims_chat_v1_HighlightRequest, strims_chat_v1_HighlightResponse>("strims.chat.v1.ChatFrontend.Highlight", service.highlight.bind(service), strims_chat_v1_HighlightRequest);
  host.registerMethod<strims_chat_v1_UnhighlightRequest, strims_chat_v1_UnhighlightResponse>("strims.chat.v1.ChatFrontend.Unhighlight", service.unhighlight.bind(service), strims_chat_v1_UnhighlightRequest);
  host.registerMethod<strims_chat_v1_TagRequest, strims_chat_v1_TagResponse>("strims.chat.v1.ChatFrontend.Tag", service.tag.bind(service), strims_chat_v1_TagRequest);
  host.registerMethod<strims_chat_v1_UntagRequest, strims_chat_v1_UntagResponse>("strims.chat.v1.ChatFrontend.Untag", service.untag.bind(service), strims_chat_v1_UntagRequest);
  host.registerMethod<strims_chat_v1_GetEmojiRequest, strims_chat_v1_GetEmojiResponse>("strims.chat.v1.ChatFrontend.GetEmoji", service.getEmoji.bind(service), strims_chat_v1_GetEmojiRequest);
}

export class ChatFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public openClient(req?: strims_chat_v1_IOpenClientRequest): GenericReadable<strims_chat_v1_OpenClientResponse> {
    return this.host.expectMany(this.host.call("strims.chat.v1.ChatFrontend.OpenClient", new strims_chat_v1_OpenClientRequest(req)), strims_chat_v1_OpenClientResponse);
  }

  public clientSendMessage(req?: strims_chat_v1_IClientSendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ClientSendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientSendMessage", new strims_chat_v1_ClientSendMessageRequest(req)), strims_chat_v1_ClientSendMessageResponse, opts);
  }

  public clientMute(req?: strims_chat_v1_IClientMuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ClientMuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientMute", new strims_chat_v1_ClientMuteRequest(req)), strims_chat_v1_ClientMuteResponse, opts);
  }

  public clientUnmute(req?: strims_chat_v1_IClientUnmuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ClientUnmuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientUnmute", new strims_chat_v1_ClientUnmuteRequest(req)), strims_chat_v1_ClientUnmuteResponse, opts);
  }

  public clientGetMute(req?: strims_chat_v1_IClientGetMuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ClientGetMuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientGetMute", new strims_chat_v1_ClientGetMuteRequest(req)), strims_chat_v1_ClientGetMuteResponse, opts);
  }

  public whisper(req?: strims_chat_v1_IWhisperRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_WhisperResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Whisper", new strims_chat_v1_WhisperRequest(req)), strims_chat_v1_WhisperResponse, opts);
  }

  public listWhispers(req?: strims_chat_v1_IListWhispersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ListWhispersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ListWhispers", new strims_chat_v1_ListWhispersRequest(req)), strims_chat_v1_ListWhispersResponse, opts);
  }

  public watchWhispers(req?: strims_chat_v1_IWatchWhispersRequest): GenericReadable<strims_chat_v1_WatchWhispersResponse> {
    return this.host.expectMany(this.host.call("strims.chat.v1.ChatFrontend.WatchWhispers", new strims_chat_v1_WatchWhispersRequest(req)), strims_chat_v1_WatchWhispersResponse);
  }

  public markWhispersRead(req?: strims_chat_v1_IMarkWhispersReadRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_MarkWhispersReadResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.MarkWhispersRead", new strims_chat_v1_MarkWhispersReadRequest(req)), strims_chat_v1_MarkWhispersReadResponse, opts);
  }

  public setUIConfig(req?: strims_chat_v1_ISetUIConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_SetUIConfigResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.SetUIConfig", new strims_chat_v1_SetUIConfigRequest(req)), strims_chat_v1_SetUIConfigResponse, opts);
  }

  public watchUIConfig(req?: strims_chat_v1_IWatchUIConfigRequest): GenericReadable<strims_chat_v1_WatchUIConfigResponse> {
    return this.host.expectMany(this.host.call("strims.chat.v1.ChatFrontend.WatchUIConfig", new strims_chat_v1_WatchUIConfigRequest(req)), strims_chat_v1_WatchUIConfigResponse);
  }

  public ignore(req?: strims_chat_v1_IIgnoreRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_IgnoreResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Ignore", new strims_chat_v1_IgnoreRequest(req)), strims_chat_v1_IgnoreResponse, opts);
  }

  public unignore(req?: strims_chat_v1_IUnignoreRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UnignoreResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Unignore", new strims_chat_v1_UnignoreRequest(req)), strims_chat_v1_UnignoreResponse, opts);
  }

  public highlight(req?: strims_chat_v1_IHighlightRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_HighlightResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Highlight", new strims_chat_v1_HighlightRequest(req)), strims_chat_v1_HighlightResponse, opts);
  }

  public unhighlight(req?: strims_chat_v1_IUnhighlightRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UnhighlightResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Unhighlight", new strims_chat_v1_UnhighlightRequest(req)), strims_chat_v1_UnhighlightResponse, opts);
  }

  public tag(req?: strims_chat_v1_ITagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_TagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Tag", new strims_chat_v1_TagRequest(req)), strims_chat_v1_TagResponse, opts);
  }

  public untag(req?: strims_chat_v1_IUntagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UntagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.Untag", new strims_chat_v1_UntagRequest(req)), strims_chat_v1_UntagResponse, opts);
  }

  public getEmoji(req?: strims_chat_v1_IGetEmojiRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_GetEmojiResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.GetEmoji", new strims_chat_v1_GetEmojiRequest(req)), strims_chat_v1_GetEmojiResponse, opts);
  }
}

export interface ChatService {
  sendMessage(req: strims_chat_v1_SendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SendMessageResponse> | strims_chat_v1_SendMessageResponse;
  mute(req: strims_chat_v1_MuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_MuteResponse> | strims_chat_v1_MuteResponse;
  unmute(req: strims_chat_v1_UnmuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnmuteResponse> | strims_chat_v1_UnmuteResponse;
  getMute(req: strims_chat_v1_GetMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetMuteResponse> | strims_chat_v1_GetMuteResponse;
}

export class UnimplementedChatService implements ChatService {
  sendMessage(req: strims_chat_v1_SendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SendMessageResponse> | strims_chat_v1_SendMessageResponse { throw new Error("not implemented"); }
  mute(req: strims_chat_v1_MuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_MuteResponse> | strims_chat_v1_MuteResponse { throw new Error("not implemented"); }
  unmute(req: strims_chat_v1_UnmuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnmuteResponse> | strims_chat_v1_UnmuteResponse { throw new Error("not implemented"); }
  getMute(req: strims_chat_v1_GetMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetMuteResponse> | strims_chat_v1_GetMuteResponse { throw new Error("not implemented"); }
}

export const registerChatService = (host: strims_rpc_Service, service: ChatService): void => {
  host.registerMethod<strims_chat_v1_SendMessageRequest, strims_chat_v1_SendMessageResponse>("strims.chat.v1.Chat.SendMessage", service.sendMessage.bind(service), strims_chat_v1_SendMessageRequest);
  host.registerMethod<strims_chat_v1_MuteRequest, strims_chat_v1_MuteResponse>("strims.chat.v1.Chat.Mute", service.mute.bind(service), strims_chat_v1_MuteRequest);
  host.registerMethod<strims_chat_v1_UnmuteRequest, strims_chat_v1_UnmuteResponse>("strims.chat.v1.Chat.Unmute", service.unmute.bind(service), strims_chat_v1_UnmuteRequest);
  host.registerMethod<strims_chat_v1_GetMuteRequest, strims_chat_v1_GetMuteResponse>("strims.chat.v1.Chat.GetMute", service.getMute.bind(service), strims_chat_v1_GetMuteRequest);
}

export class ChatClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public sendMessage(req?: strims_chat_v1_ISendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_SendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.SendMessage", new strims_chat_v1_SendMessageRequest(req)), strims_chat_v1_SendMessageResponse, opts);
  }

  public mute(req?: strims_chat_v1_IMuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_MuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.Mute", new strims_chat_v1_MuteRequest(req)), strims_chat_v1_MuteResponse, opts);
  }

  public unmute(req?: strims_chat_v1_IUnmuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UnmuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.Unmute", new strims_chat_v1_UnmuteRequest(req)), strims_chat_v1_UnmuteResponse, opts);
  }

  public getMute(req?: strims_chat_v1_IGetMuteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_GetMuteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.GetMute", new strims_chat_v1_GetMuteRequest(req)), strims_chat_v1_GetMuteResponse, opts);
  }
}

export interface WhisperService {
  sendMessage(req: strims_chat_v1_WhisperSendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_WhisperSendMessageResponse> | strims_chat_v1_WhisperSendMessageResponse;
}

export class UnimplementedWhisperService implements WhisperService {
  sendMessage(req: strims_chat_v1_WhisperSendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_WhisperSendMessageResponse> | strims_chat_v1_WhisperSendMessageResponse { throw new Error("not implemented"); }
}

export const registerWhisperService = (host: strims_rpc_Service, service: WhisperService): void => {
  host.registerMethod<strims_chat_v1_WhisperSendMessageRequest, strims_chat_v1_WhisperSendMessageResponse>("strims.chat.v1.Whisper.SendMessage", service.sendMessage.bind(service), strims_chat_v1_WhisperSendMessageRequest);
}

export class WhisperClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public sendMessage(req?: strims_chat_v1_IWhisperSendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_WhisperSendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Whisper.SendMessage", new strims_chat_v1_WhisperSendMessageRequest(req)), strims_chat_v1_WhisperSendMessageResponse, opts);
  }
}

