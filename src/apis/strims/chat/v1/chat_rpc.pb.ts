// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import type { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  type ICreateServerRequest as strims_chat_v1_ICreateServerRequest,
  CreateServerRequest as strims_chat_v1_CreateServerRequest,
  CreateServerResponse as strims_chat_v1_CreateServerResponse,
  type IUpdateServerRequest as strims_chat_v1_IUpdateServerRequest,
  UpdateServerRequest as strims_chat_v1_UpdateServerRequest,
  UpdateServerResponse as strims_chat_v1_UpdateServerResponse,
  type IDeleteServerRequest as strims_chat_v1_IDeleteServerRequest,
  DeleteServerRequest as strims_chat_v1_DeleteServerRequest,
  DeleteServerResponse as strims_chat_v1_DeleteServerResponse,
  type IGetServerRequest as strims_chat_v1_IGetServerRequest,
  GetServerRequest as strims_chat_v1_GetServerRequest,
  GetServerResponse as strims_chat_v1_GetServerResponse,
  type IListServersRequest as strims_chat_v1_IListServersRequest,
  ListServersRequest as strims_chat_v1_ListServersRequest,
  ListServersResponse as strims_chat_v1_ListServersResponse,
  type IUpdateServerIconRequest as strims_chat_v1_IUpdateServerIconRequest,
  UpdateServerIconRequest as strims_chat_v1_UpdateServerIconRequest,
  UpdateServerIconResponse as strims_chat_v1_UpdateServerIconResponse,
  type IGetServerIconRequest as strims_chat_v1_IGetServerIconRequest,
  GetServerIconRequest as strims_chat_v1_GetServerIconRequest,
  GetServerIconResponse as strims_chat_v1_GetServerIconResponse,
  type ICreateEmoteRequest as strims_chat_v1_ICreateEmoteRequest,
  CreateEmoteRequest as strims_chat_v1_CreateEmoteRequest,
  CreateEmoteResponse as strims_chat_v1_CreateEmoteResponse,
  type IUpdateEmoteRequest as strims_chat_v1_IUpdateEmoteRequest,
  UpdateEmoteRequest as strims_chat_v1_UpdateEmoteRequest,
  UpdateEmoteResponse as strims_chat_v1_UpdateEmoteResponse,
  type IDeleteEmoteRequest as strims_chat_v1_IDeleteEmoteRequest,
  DeleteEmoteRequest as strims_chat_v1_DeleteEmoteRequest,
  DeleteEmoteResponse as strims_chat_v1_DeleteEmoteResponse,
  type IGetEmoteRequest as strims_chat_v1_IGetEmoteRequest,
  GetEmoteRequest as strims_chat_v1_GetEmoteRequest,
  GetEmoteResponse as strims_chat_v1_GetEmoteResponse,
  type IListEmotesRequest as strims_chat_v1_IListEmotesRequest,
  ListEmotesRequest as strims_chat_v1_ListEmotesRequest,
  ListEmotesResponse as strims_chat_v1_ListEmotesResponse,
  type IUpdateEmotesRequest as strims_chat_v1_IUpdateEmotesRequest,
  UpdateEmotesRequest as strims_chat_v1_UpdateEmotesRequest,
  UpdateEmotesResponse as strims_chat_v1_UpdateEmotesResponse,
  type IListEmoteLabelsRequest as strims_chat_v1_IListEmoteLabelsRequest,
  ListEmoteLabelsRequest as strims_chat_v1_ListEmoteLabelsRequest,
  ListEmoteLabelsResponse as strims_chat_v1_ListEmoteLabelsResponse,
  type ICreateModifierRequest as strims_chat_v1_ICreateModifierRequest,
  CreateModifierRequest as strims_chat_v1_CreateModifierRequest,
  CreateModifierResponse as strims_chat_v1_CreateModifierResponse,
  type IUpdateModifierRequest as strims_chat_v1_IUpdateModifierRequest,
  UpdateModifierRequest as strims_chat_v1_UpdateModifierRequest,
  UpdateModifierResponse as strims_chat_v1_UpdateModifierResponse,
  type IDeleteModifierRequest as strims_chat_v1_IDeleteModifierRequest,
  DeleteModifierRequest as strims_chat_v1_DeleteModifierRequest,
  DeleteModifierResponse as strims_chat_v1_DeleteModifierResponse,
  type IGetModifierRequest as strims_chat_v1_IGetModifierRequest,
  GetModifierRequest as strims_chat_v1_GetModifierRequest,
  GetModifierResponse as strims_chat_v1_GetModifierResponse,
  type IListModifiersRequest as strims_chat_v1_IListModifiersRequest,
  ListModifiersRequest as strims_chat_v1_ListModifiersRequest,
  ListModifiersResponse as strims_chat_v1_ListModifiersResponse,
  type ICreateTagRequest as strims_chat_v1_ICreateTagRequest,
  CreateTagRequest as strims_chat_v1_CreateTagRequest,
  CreateTagResponse as strims_chat_v1_CreateTagResponse,
  type IUpdateTagRequest as strims_chat_v1_IUpdateTagRequest,
  UpdateTagRequest as strims_chat_v1_UpdateTagRequest,
  UpdateTagResponse as strims_chat_v1_UpdateTagResponse,
  type IDeleteTagRequest as strims_chat_v1_IDeleteTagRequest,
  DeleteTagRequest as strims_chat_v1_DeleteTagRequest,
  DeleteTagResponse as strims_chat_v1_DeleteTagResponse,
  type IGetTagRequest as strims_chat_v1_IGetTagRequest,
  GetTagRequest as strims_chat_v1_GetTagRequest,
  GetTagResponse as strims_chat_v1_GetTagResponse,
  type IListTagsRequest as strims_chat_v1_IListTagsRequest,
  ListTagsRequest as strims_chat_v1_ListTagsRequest,
  ListTagsResponse as strims_chat_v1_ListTagsResponse,
  type ISyncAssetsRequest as strims_chat_v1_ISyncAssetsRequest,
  SyncAssetsRequest as strims_chat_v1_SyncAssetsRequest,
  SyncAssetsResponse as strims_chat_v1_SyncAssetsResponse,
  type IOpenClientRequest as strims_chat_v1_IOpenClientRequest,
  OpenClientRequest as strims_chat_v1_OpenClientRequest,
  OpenClientResponse as strims_chat_v1_OpenClientResponse,
  type IClientSendMessageRequest as strims_chat_v1_IClientSendMessageRequest,
  ClientSendMessageRequest as strims_chat_v1_ClientSendMessageRequest,
  ClientSendMessageResponse as strims_chat_v1_ClientSendMessageResponse,
  type IClientMuteRequest as strims_chat_v1_IClientMuteRequest,
  ClientMuteRequest as strims_chat_v1_ClientMuteRequest,
  ClientMuteResponse as strims_chat_v1_ClientMuteResponse,
  type IClientUnmuteRequest as strims_chat_v1_IClientUnmuteRequest,
  ClientUnmuteRequest as strims_chat_v1_ClientUnmuteRequest,
  ClientUnmuteResponse as strims_chat_v1_ClientUnmuteResponse,
  type IClientGetMuteRequest as strims_chat_v1_IClientGetMuteRequest,
  ClientGetMuteRequest as strims_chat_v1_ClientGetMuteRequest,
  ClientGetMuteResponse as strims_chat_v1_ClientGetMuteResponse,
  type IWhisperRequest as strims_chat_v1_IWhisperRequest,
  WhisperRequest as strims_chat_v1_WhisperRequest,
  WhisperResponse as strims_chat_v1_WhisperResponse,
  type IListWhispersRequest as strims_chat_v1_IListWhispersRequest,
  ListWhispersRequest as strims_chat_v1_ListWhispersRequest,
  ListWhispersResponse as strims_chat_v1_ListWhispersResponse,
  type IWatchWhispersRequest as strims_chat_v1_IWatchWhispersRequest,
  WatchWhispersRequest as strims_chat_v1_WatchWhispersRequest,
  WatchWhispersResponse as strims_chat_v1_WatchWhispersResponse,
  type IMarkWhispersReadRequest as strims_chat_v1_IMarkWhispersReadRequest,
  MarkWhispersReadRequest as strims_chat_v1_MarkWhispersReadRequest,
  MarkWhispersReadResponse as strims_chat_v1_MarkWhispersReadResponse,
  type IDeleteWhisperThreadRequest as strims_chat_v1_IDeleteWhisperThreadRequest,
  DeleteWhisperThreadRequest as strims_chat_v1_DeleteWhisperThreadRequest,
  DeleteWhisperThreadResponse as strims_chat_v1_DeleteWhisperThreadResponse,
  type ISetUIConfigRequest as strims_chat_v1_ISetUIConfigRequest,
  SetUIConfigRequest as strims_chat_v1_SetUIConfigRequest,
  SetUIConfigResponse as strims_chat_v1_SetUIConfigResponse,
  type IWatchUIConfigRequest as strims_chat_v1_IWatchUIConfigRequest,
  WatchUIConfigRequest as strims_chat_v1_WatchUIConfigRequest,
  WatchUIConfigResponse as strims_chat_v1_WatchUIConfigResponse,
  type IIgnoreRequest as strims_chat_v1_IIgnoreRequest,
  IgnoreRequest as strims_chat_v1_IgnoreRequest,
  IgnoreResponse as strims_chat_v1_IgnoreResponse,
  type IUnignoreRequest as strims_chat_v1_IUnignoreRequest,
  UnignoreRequest as strims_chat_v1_UnignoreRequest,
  UnignoreResponse as strims_chat_v1_UnignoreResponse,
  type IHighlightRequest as strims_chat_v1_IHighlightRequest,
  HighlightRequest as strims_chat_v1_HighlightRequest,
  HighlightResponse as strims_chat_v1_HighlightResponse,
  type IUnhighlightRequest as strims_chat_v1_IUnhighlightRequest,
  UnhighlightRequest as strims_chat_v1_UnhighlightRequest,
  UnhighlightResponse as strims_chat_v1_UnhighlightResponse,
  type ITagRequest as strims_chat_v1_ITagRequest,
  TagRequest as strims_chat_v1_TagRequest,
  TagResponse as strims_chat_v1_TagResponse,
  type IUntagRequest as strims_chat_v1_IUntagRequest,
  UntagRequest as strims_chat_v1_UntagRequest,
  UntagResponse as strims_chat_v1_UntagResponse,
  type ISendMessageRequest as strims_chat_v1_ISendMessageRequest,
  SendMessageRequest as strims_chat_v1_SendMessageRequest,
  SendMessageResponse as strims_chat_v1_SendMessageResponse,
  type IMuteRequest as strims_chat_v1_IMuteRequest,
  MuteRequest as strims_chat_v1_MuteRequest,
  MuteResponse as strims_chat_v1_MuteResponse,
  type IUnmuteRequest as strims_chat_v1_IUnmuteRequest,
  UnmuteRequest as strims_chat_v1_UnmuteRequest,
  UnmuteResponse as strims_chat_v1_UnmuteResponse,
  type IGetMuteRequest as strims_chat_v1_IGetMuteRequest,
  GetMuteRequest as strims_chat_v1_GetMuteRequest,
  GetMuteResponse as strims_chat_v1_GetMuteResponse,
  type IWhisperSendMessageRequest as strims_chat_v1_IWhisperSendMessageRequest,
  WhisperSendMessageRequest as strims_chat_v1_WhisperSendMessageRequest,
  WhisperSendMessageResponse as strims_chat_v1_WhisperSendMessageResponse,
} from "./chat.pb";

export interface ChatServerFrontendService {
  createServer(req: strims_chat_v1_CreateServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateServerResponse> | strims_chat_v1_CreateServerResponse;
  updateServer(req: strims_chat_v1_UpdateServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateServerResponse> | strims_chat_v1_UpdateServerResponse;
  deleteServer(req: strims_chat_v1_DeleteServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteServerResponse> | strims_chat_v1_DeleteServerResponse;
  getServer(req: strims_chat_v1_GetServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetServerResponse> | strims_chat_v1_GetServerResponse;
  listServers(req: strims_chat_v1_ListServersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListServersResponse> | strims_chat_v1_ListServersResponse;
  updateServerIcon(req: strims_chat_v1_UpdateServerIconRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateServerIconResponse> | strims_chat_v1_UpdateServerIconResponse;
  getServerIcon(req: strims_chat_v1_GetServerIconRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetServerIconResponse> | strims_chat_v1_GetServerIconResponse;
  createEmote(req: strims_chat_v1_CreateEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateEmoteResponse> | strims_chat_v1_CreateEmoteResponse;
  updateEmote(req: strims_chat_v1_UpdateEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateEmoteResponse> | strims_chat_v1_UpdateEmoteResponse;
  deleteEmote(req: strims_chat_v1_DeleteEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteEmoteResponse> | strims_chat_v1_DeleteEmoteResponse;
  getEmote(req: strims_chat_v1_GetEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetEmoteResponse> | strims_chat_v1_GetEmoteResponse;
  listEmotes(req: strims_chat_v1_ListEmotesRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListEmotesResponse> | strims_chat_v1_ListEmotesResponse;
  updateEmotes(req: strims_chat_v1_UpdateEmotesRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateEmotesResponse> | strims_chat_v1_UpdateEmotesResponse;
  listEmoteLabels(req: strims_chat_v1_ListEmoteLabelsRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListEmoteLabelsResponse> | strims_chat_v1_ListEmoteLabelsResponse;
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
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createServer(req: strims_chat_v1_CreateServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateServerResponse> | strims_chat_v1_CreateServerResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateServer(req: strims_chat_v1_UpdateServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateServerResponse> | strims_chat_v1_UpdateServerResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deleteServer(req: strims_chat_v1_DeleteServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteServerResponse> | strims_chat_v1_DeleteServerResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getServer(req: strims_chat_v1_GetServerRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetServerResponse> | strims_chat_v1_GetServerResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listServers(req: strims_chat_v1_ListServersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListServersResponse> | strims_chat_v1_ListServersResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateServerIcon(req: strims_chat_v1_UpdateServerIconRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateServerIconResponse> | strims_chat_v1_UpdateServerIconResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getServerIcon(req: strims_chat_v1_GetServerIconRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetServerIconResponse> | strims_chat_v1_GetServerIconResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createEmote(req: strims_chat_v1_CreateEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateEmoteResponse> | strims_chat_v1_CreateEmoteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateEmote(req: strims_chat_v1_UpdateEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateEmoteResponse> | strims_chat_v1_UpdateEmoteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deleteEmote(req: strims_chat_v1_DeleteEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteEmoteResponse> | strims_chat_v1_DeleteEmoteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getEmote(req: strims_chat_v1_GetEmoteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetEmoteResponse> | strims_chat_v1_GetEmoteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listEmotes(req: strims_chat_v1_ListEmotesRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListEmotesResponse> | strims_chat_v1_ListEmotesResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateEmotes(req: strims_chat_v1_UpdateEmotesRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateEmotesResponse> | strims_chat_v1_UpdateEmotesResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listEmoteLabels(req: strims_chat_v1_ListEmoteLabelsRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListEmoteLabelsResponse> | strims_chat_v1_ListEmoteLabelsResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createModifier(req: strims_chat_v1_CreateModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateModifierResponse> | strims_chat_v1_CreateModifierResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateModifier(req: strims_chat_v1_UpdateModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateModifierResponse> | strims_chat_v1_UpdateModifierResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deleteModifier(req: strims_chat_v1_DeleteModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteModifierResponse> | strims_chat_v1_DeleteModifierResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getModifier(req: strims_chat_v1_GetModifierRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetModifierResponse> | strims_chat_v1_GetModifierResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listModifiers(req: strims_chat_v1_ListModifiersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListModifiersResponse> | strims_chat_v1_ListModifiersResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createTag(req: strims_chat_v1_CreateTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_CreateTagResponse> | strims_chat_v1_CreateTagResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  updateTag(req: strims_chat_v1_UpdateTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UpdateTagResponse> | strims_chat_v1_UpdateTagResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deleteTag(req: strims_chat_v1_DeleteTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteTagResponse> | strims_chat_v1_DeleteTagResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getTag(req: strims_chat_v1_GetTagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetTagResponse> | strims_chat_v1_GetTagResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listTags(req: strims_chat_v1_ListTagsRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListTagsResponse> | strims_chat_v1_ListTagsResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  syncAssets(req: strims_chat_v1_SyncAssetsRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SyncAssetsResponse> | strims_chat_v1_SyncAssetsResponse { throw new Error("not implemented"); }
}

export const registerChatServerFrontendService = (host: strims_rpc_Service, service: ChatServerFrontendService): void => {
  host.registerMethod<strims_chat_v1_CreateServerRequest, strims_chat_v1_CreateServerResponse>("strims.chat.v1.ChatServerFrontend.CreateServer", (req, call) => service.createServer(req, call), strims_chat_v1_CreateServerRequest);
  host.registerMethod<strims_chat_v1_UpdateServerRequest, strims_chat_v1_UpdateServerResponse>("strims.chat.v1.ChatServerFrontend.UpdateServer", (req, call) => service.updateServer(req, call), strims_chat_v1_UpdateServerRequest);
  host.registerMethod<strims_chat_v1_DeleteServerRequest, strims_chat_v1_DeleteServerResponse>("strims.chat.v1.ChatServerFrontend.DeleteServer", (req, call) => service.deleteServer(req, call), strims_chat_v1_DeleteServerRequest);
  host.registerMethod<strims_chat_v1_GetServerRequest, strims_chat_v1_GetServerResponse>("strims.chat.v1.ChatServerFrontend.GetServer", (req, call) => service.getServer(req, call), strims_chat_v1_GetServerRequest);
  host.registerMethod<strims_chat_v1_ListServersRequest, strims_chat_v1_ListServersResponse>("strims.chat.v1.ChatServerFrontend.ListServers", (req, call) => service.listServers(req, call), strims_chat_v1_ListServersRequest);
  host.registerMethod<strims_chat_v1_UpdateServerIconRequest, strims_chat_v1_UpdateServerIconResponse>("strims.chat.v1.ChatServerFrontend.UpdateServerIcon", (req, call) => service.updateServerIcon(req, call), strims_chat_v1_UpdateServerIconRequest);
  host.registerMethod<strims_chat_v1_GetServerIconRequest, strims_chat_v1_GetServerIconResponse>("strims.chat.v1.ChatServerFrontend.GetServerIcon", (req, call) => service.getServerIcon(req, call), strims_chat_v1_GetServerIconRequest);
  host.registerMethod<strims_chat_v1_CreateEmoteRequest, strims_chat_v1_CreateEmoteResponse>("strims.chat.v1.ChatServerFrontend.CreateEmote", (req, call) => service.createEmote(req, call), strims_chat_v1_CreateEmoteRequest);
  host.registerMethod<strims_chat_v1_UpdateEmoteRequest, strims_chat_v1_UpdateEmoteResponse>("strims.chat.v1.ChatServerFrontend.UpdateEmote", (req, call) => service.updateEmote(req, call), strims_chat_v1_UpdateEmoteRequest);
  host.registerMethod<strims_chat_v1_DeleteEmoteRequest, strims_chat_v1_DeleteEmoteResponse>("strims.chat.v1.ChatServerFrontend.DeleteEmote", (req, call) => service.deleteEmote(req, call), strims_chat_v1_DeleteEmoteRequest);
  host.registerMethod<strims_chat_v1_GetEmoteRequest, strims_chat_v1_GetEmoteResponse>("strims.chat.v1.ChatServerFrontend.GetEmote", (req, call) => service.getEmote(req, call), strims_chat_v1_GetEmoteRequest);
  host.registerMethod<strims_chat_v1_ListEmotesRequest, strims_chat_v1_ListEmotesResponse>("strims.chat.v1.ChatServerFrontend.ListEmotes", (req, call) => service.listEmotes(req, call), strims_chat_v1_ListEmotesRequest);
  host.registerMethod<strims_chat_v1_UpdateEmotesRequest, strims_chat_v1_UpdateEmotesResponse>("strims.chat.v1.ChatServerFrontend.UpdateEmotes", (req, call) => service.updateEmotes(req, call), strims_chat_v1_UpdateEmotesRequest);
  host.registerMethod<strims_chat_v1_ListEmoteLabelsRequest, strims_chat_v1_ListEmoteLabelsResponse>("strims.chat.v1.ChatServerFrontend.ListEmoteLabels", (req, call) => service.listEmoteLabels(req, call), strims_chat_v1_ListEmoteLabelsRequest);
  host.registerMethod<strims_chat_v1_CreateModifierRequest, strims_chat_v1_CreateModifierResponse>("strims.chat.v1.ChatServerFrontend.CreateModifier", (req, call) => service.createModifier(req, call), strims_chat_v1_CreateModifierRequest);
  host.registerMethod<strims_chat_v1_UpdateModifierRequest, strims_chat_v1_UpdateModifierResponse>("strims.chat.v1.ChatServerFrontend.UpdateModifier", (req, call) => service.updateModifier(req, call), strims_chat_v1_UpdateModifierRequest);
  host.registerMethod<strims_chat_v1_DeleteModifierRequest, strims_chat_v1_DeleteModifierResponse>("strims.chat.v1.ChatServerFrontend.DeleteModifier", (req, call) => service.deleteModifier(req, call), strims_chat_v1_DeleteModifierRequest);
  host.registerMethod<strims_chat_v1_GetModifierRequest, strims_chat_v1_GetModifierResponse>("strims.chat.v1.ChatServerFrontend.GetModifier", (req, call) => service.getModifier(req, call), strims_chat_v1_GetModifierRequest);
  host.registerMethod<strims_chat_v1_ListModifiersRequest, strims_chat_v1_ListModifiersResponse>("strims.chat.v1.ChatServerFrontend.ListModifiers", (req, call) => service.listModifiers(req, call), strims_chat_v1_ListModifiersRequest);
  host.registerMethod<strims_chat_v1_CreateTagRequest, strims_chat_v1_CreateTagResponse>("strims.chat.v1.ChatServerFrontend.CreateTag", (req, call) => service.createTag(req, call), strims_chat_v1_CreateTagRequest);
  host.registerMethod<strims_chat_v1_UpdateTagRequest, strims_chat_v1_UpdateTagResponse>("strims.chat.v1.ChatServerFrontend.UpdateTag", (req, call) => service.updateTag(req, call), strims_chat_v1_UpdateTagRequest);
  host.registerMethod<strims_chat_v1_DeleteTagRequest, strims_chat_v1_DeleteTagResponse>("strims.chat.v1.ChatServerFrontend.DeleteTag", (req, call) => service.deleteTag(req, call), strims_chat_v1_DeleteTagRequest);
  host.registerMethod<strims_chat_v1_GetTagRequest, strims_chat_v1_GetTagResponse>("strims.chat.v1.ChatServerFrontend.GetTag", (req, call) => service.getTag(req, call), strims_chat_v1_GetTagRequest);
  host.registerMethod<strims_chat_v1_ListTagsRequest, strims_chat_v1_ListTagsResponse>("strims.chat.v1.ChatServerFrontend.ListTags", (req, call) => service.listTags(req, call), strims_chat_v1_ListTagsRequest);
  host.registerMethod<strims_chat_v1_SyncAssetsRequest, strims_chat_v1_SyncAssetsResponse>("strims.chat.v1.ChatServerFrontend.SyncAssets", (req, call) => service.syncAssets(req, call), strims_chat_v1_SyncAssetsRequest);
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

  public updateServerIcon(req?: strims_chat_v1_IUpdateServerIconRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UpdateServerIconResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateServerIcon", new strims_chat_v1_UpdateServerIconRequest(req)), strims_chat_v1_UpdateServerIconResponse, opts);
  }

  public getServerIcon(req?: strims_chat_v1_IGetServerIconRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_GetServerIconResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetServerIcon", new strims_chat_v1_GetServerIconRequest(req)), strims_chat_v1_GetServerIconResponse, opts);
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

  public updateEmotes(req?: strims_chat_v1_IUpdateEmotesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_UpdateEmotesResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateEmotes", new strims_chat_v1_UpdateEmotesRequest(req)), strims_chat_v1_UpdateEmotesResponse, opts);
  }

  public listEmoteLabels(req?: strims_chat_v1_IListEmoteLabelsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_ListEmoteLabelsResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListEmoteLabels", new strims_chat_v1_ListEmoteLabelsRequest(req)), strims_chat_v1_ListEmoteLabelsResponse, opts);
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
  deleteWhisperThread(req: strims_chat_v1_DeleteWhisperThreadRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteWhisperThreadResponse> | strims_chat_v1_DeleteWhisperThreadResponse;
  setUIConfig(req: strims_chat_v1_SetUIConfigRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SetUIConfigResponse> | strims_chat_v1_SetUIConfigResponse;
  watchUIConfig(req: strims_chat_v1_WatchUIConfigRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_WatchUIConfigResponse>;
  ignore(req: strims_chat_v1_IgnoreRequest, call: strims_rpc_Call): Promise<strims_chat_v1_IgnoreResponse> | strims_chat_v1_IgnoreResponse;
  unignore(req: strims_chat_v1_UnignoreRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnignoreResponse> | strims_chat_v1_UnignoreResponse;
  highlight(req: strims_chat_v1_HighlightRequest, call: strims_rpc_Call): Promise<strims_chat_v1_HighlightResponse> | strims_chat_v1_HighlightResponse;
  unhighlight(req: strims_chat_v1_UnhighlightRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnhighlightResponse> | strims_chat_v1_UnhighlightResponse;
  tag(req: strims_chat_v1_TagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_TagResponse> | strims_chat_v1_TagResponse;
  untag(req: strims_chat_v1_UntagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UntagResponse> | strims_chat_v1_UntagResponse;
}

export class UnimplementedChatFrontendService implements ChatFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  openClient(req: strims_chat_v1_OpenClientRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_OpenClientResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  clientSendMessage(req: strims_chat_v1_ClientSendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientSendMessageResponse> | strims_chat_v1_ClientSendMessageResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  clientMute(req: strims_chat_v1_ClientMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientMuteResponse> | strims_chat_v1_ClientMuteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  clientUnmute(req: strims_chat_v1_ClientUnmuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientUnmuteResponse> | strims_chat_v1_ClientUnmuteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  clientGetMute(req: strims_chat_v1_ClientGetMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ClientGetMuteResponse> | strims_chat_v1_ClientGetMuteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  whisper(req: strims_chat_v1_WhisperRequest, call: strims_rpc_Call): Promise<strims_chat_v1_WhisperResponse> | strims_chat_v1_WhisperResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listWhispers(req: strims_chat_v1_ListWhispersRequest, call: strims_rpc_Call): Promise<strims_chat_v1_ListWhispersResponse> | strims_chat_v1_ListWhispersResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watchWhispers(req: strims_chat_v1_WatchWhispersRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_WatchWhispersResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  markWhispersRead(req: strims_chat_v1_MarkWhispersReadRequest, call: strims_rpc_Call): Promise<strims_chat_v1_MarkWhispersReadResponse> | strims_chat_v1_MarkWhispersReadResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  deleteWhisperThread(req: strims_chat_v1_DeleteWhisperThreadRequest, call: strims_rpc_Call): Promise<strims_chat_v1_DeleteWhisperThreadResponse> | strims_chat_v1_DeleteWhisperThreadResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setUIConfig(req: strims_chat_v1_SetUIConfigRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SetUIConfigResponse> | strims_chat_v1_SetUIConfigResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watchUIConfig(req: strims_chat_v1_WatchUIConfigRequest, call: strims_rpc_Call): GenericReadable<strims_chat_v1_WatchUIConfigResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  ignore(req: strims_chat_v1_IgnoreRequest, call: strims_rpc_Call): Promise<strims_chat_v1_IgnoreResponse> | strims_chat_v1_IgnoreResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  unignore(req: strims_chat_v1_UnignoreRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnignoreResponse> | strims_chat_v1_UnignoreResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  highlight(req: strims_chat_v1_HighlightRequest, call: strims_rpc_Call): Promise<strims_chat_v1_HighlightResponse> | strims_chat_v1_HighlightResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  unhighlight(req: strims_chat_v1_UnhighlightRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnhighlightResponse> | strims_chat_v1_UnhighlightResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  tag(req: strims_chat_v1_TagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_TagResponse> | strims_chat_v1_TagResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  untag(req: strims_chat_v1_UntagRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UntagResponse> | strims_chat_v1_UntagResponse { throw new Error("not implemented"); }
}

export const registerChatFrontendService = (host: strims_rpc_Service, service: ChatFrontendService): void => {
  host.registerMethod<strims_chat_v1_OpenClientRequest, strims_chat_v1_OpenClientResponse>("strims.chat.v1.ChatFrontend.OpenClient", (req, call) => service.openClient(req, call), strims_chat_v1_OpenClientRequest);
  host.registerMethod<strims_chat_v1_ClientSendMessageRequest, strims_chat_v1_ClientSendMessageResponse>("strims.chat.v1.ChatFrontend.ClientSendMessage", (req, call) => service.clientSendMessage(req, call), strims_chat_v1_ClientSendMessageRequest);
  host.registerMethod<strims_chat_v1_ClientMuteRequest, strims_chat_v1_ClientMuteResponse>("strims.chat.v1.ChatFrontend.ClientMute", (req, call) => service.clientMute(req, call), strims_chat_v1_ClientMuteRequest);
  host.registerMethod<strims_chat_v1_ClientUnmuteRequest, strims_chat_v1_ClientUnmuteResponse>("strims.chat.v1.ChatFrontend.ClientUnmute", (req, call) => service.clientUnmute(req, call), strims_chat_v1_ClientUnmuteRequest);
  host.registerMethod<strims_chat_v1_ClientGetMuteRequest, strims_chat_v1_ClientGetMuteResponse>("strims.chat.v1.ChatFrontend.ClientGetMute", (req, call) => service.clientGetMute(req, call), strims_chat_v1_ClientGetMuteRequest);
  host.registerMethod<strims_chat_v1_WhisperRequest, strims_chat_v1_WhisperResponse>("strims.chat.v1.ChatFrontend.Whisper", (req, call) => service.whisper(req, call), strims_chat_v1_WhisperRequest);
  host.registerMethod<strims_chat_v1_ListWhispersRequest, strims_chat_v1_ListWhispersResponse>("strims.chat.v1.ChatFrontend.ListWhispers", (req, call) => service.listWhispers(req, call), strims_chat_v1_ListWhispersRequest);
  host.registerMethod<strims_chat_v1_WatchWhispersRequest, strims_chat_v1_WatchWhispersResponse>("strims.chat.v1.ChatFrontend.WatchWhispers", (req, call) => service.watchWhispers(req, call), strims_chat_v1_WatchWhispersRequest);
  host.registerMethod<strims_chat_v1_MarkWhispersReadRequest, strims_chat_v1_MarkWhispersReadResponse>("strims.chat.v1.ChatFrontend.MarkWhispersRead", (req, call) => service.markWhispersRead(req, call), strims_chat_v1_MarkWhispersReadRequest);
  host.registerMethod<strims_chat_v1_DeleteWhisperThreadRequest, strims_chat_v1_DeleteWhisperThreadResponse>("strims.chat.v1.ChatFrontend.DeleteWhisperThread", (req, call) => service.deleteWhisperThread(req, call), strims_chat_v1_DeleteWhisperThreadRequest);
  host.registerMethod<strims_chat_v1_SetUIConfigRequest, strims_chat_v1_SetUIConfigResponse>("strims.chat.v1.ChatFrontend.SetUIConfig", (req, call) => service.setUIConfig(req, call), strims_chat_v1_SetUIConfigRequest);
  host.registerMethod<strims_chat_v1_WatchUIConfigRequest, strims_chat_v1_WatchUIConfigResponse>("strims.chat.v1.ChatFrontend.WatchUIConfig", (req, call) => service.watchUIConfig(req, call), strims_chat_v1_WatchUIConfigRequest);
  host.registerMethod<strims_chat_v1_IgnoreRequest, strims_chat_v1_IgnoreResponse>("strims.chat.v1.ChatFrontend.Ignore", (req, call) => service.ignore(req, call), strims_chat_v1_IgnoreRequest);
  host.registerMethod<strims_chat_v1_UnignoreRequest, strims_chat_v1_UnignoreResponse>("strims.chat.v1.ChatFrontend.Unignore", (req, call) => service.unignore(req, call), strims_chat_v1_UnignoreRequest);
  host.registerMethod<strims_chat_v1_HighlightRequest, strims_chat_v1_HighlightResponse>("strims.chat.v1.ChatFrontend.Highlight", (req, call) => service.highlight(req, call), strims_chat_v1_HighlightRequest);
  host.registerMethod<strims_chat_v1_UnhighlightRequest, strims_chat_v1_UnhighlightResponse>("strims.chat.v1.ChatFrontend.Unhighlight", (req, call) => service.unhighlight(req, call), strims_chat_v1_UnhighlightRequest);
  host.registerMethod<strims_chat_v1_TagRequest, strims_chat_v1_TagResponse>("strims.chat.v1.ChatFrontend.Tag", (req, call) => service.tag(req, call), strims_chat_v1_TagRequest);
  host.registerMethod<strims_chat_v1_UntagRequest, strims_chat_v1_UntagResponse>("strims.chat.v1.ChatFrontend.Untag", (req, call) => service.untag(req, call), strims_chat_v1_UntagRequest);
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

  public deleteWhisperThread(req?: strims_chat_v1_IDeleteWhisperThreadRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_DeleteWhisperThreadResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.DeleteWhisperThread", new strims_chat_v1_DeleteWhisperThreadRequest(req)), strims_chat_v1_DeleteWhisperThreadResponse, opts);
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
}

export interface ChatService {
  sendMessage(req: strims_chat_v1_SendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SendMessageResponse> | strims_chat_v1_SendMessageResponse;
  mute(req: strims_chat_v1_MuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_MuteResponse> | strims_chat_v1_MuteResponse;
  unmute(req: strims_chat_v1_UnmuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnmuteResponse> | strims_chat_v1_UnmuteResponse;
  getMute(req: strims_chat_v1_GetMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetMuteResponse> | strims_chat_v1_GetMuteResponse;
}

export class UnimplementedChatService implements ChatService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  sendMessage(req: strims_chat_v1_SendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_SendMessageResponse> | strims_chat_v1_SendMessageResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  mute(req: strims_chat_v1_MuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_MuteResponse> | strims_chat_v1_MuteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  unmute(req: strims_chat_v1_UnmuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_UnmuteResponse> | strims_chat_v1_UnmuteResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getMute(req: strims_chat_v1_GetMuteRequest, call: strims_rpc_Call): Promise<strims_chat_v1_GetMuteResponse> | strims_chat_v1_GetMuteResponse { throw new Error("not implemented"); }
}

export const registerChatService = (host: strims_rpc_Service, service: ChatService): void => {
  host.registerMethod<strims_chat_v1_SendMessageRequest, strims_chat_v1_SendMessageResponse>("strims.chat.v1.Chat.SendMessage", (req, call) => service.sendMessage(req, call), strims_chat_v1_SendMessageRequest);
  host.registerMethod<strims_chat_v1_MuteRequest, strims_chat_v1_MuteResponse>("strims.chat.v1.Chat.Mute", (req, call) => service.mute(req, call), strims_chat_v1_MuteRequest);
  host.registerMethod<strims_chat_v1_UnmuteRequest, strims_chat_v1_UnmuteResponse>("strims.chat.v1.Chat.Unmute", (req, call) => service.unmute(req, call), strims_chat_v1_UnmuteRequest);
  host.registerMethod<strims_chat_v1_GetMuteRequest, strims_chat_v1_GetMuteResponse>("strims.chat.v1.Chat.GetMute", (req, call) => service.getMute(req, call), strims_chat_v1_GetMuteRequest);
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
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  sendMessage(req: strims_chat_v1_WhisperSendMessageRequest, call: strims_rpc_Call): Promise<strims_chat_v1_WhisperSendMessageResponse> | strims_chat_v1_WhisperSendMessageResponse { throw new Error("not implemented"); }
}

export const registerWhisperService = (host: strims_rpc_Service, service: WhisperService): void => {
  host.registerMethod<strims_chat_v1_WhisperSendMessageRequest, strims_chat_v1_WhisperSendMessageResponse>("strims.chat.v1.Whisper.SendMessage", (req, call) => service.sendMessage(req, call), strims_chat_v1_WhisperSendMessageRequest);
}

export class WhisperClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public sendMessage(req?: strims_chat_v1_IWhisperSendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_chat_v1_WhisperSendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Whisper.SendMessage", new strims_chat_v1_WhisperSendMessageRequest(req)), strims_chat_v1_WhisperSendMessageResponse, opts);
  }
}

