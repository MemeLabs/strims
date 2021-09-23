import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
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
  ISetUIConfigRequest,
  SetUIConfigRequest,
  SetUIConfigResponse,
  IGetUIConfigRequest,
  GetUIConfigRequest,
  GetUIConfigResponse,
  ISendMessageRequest,
  SendMessageRequest,
  SendMessageResponse,
} from "./chat";

registerType("strims.chat.v1.CreateServerRequest", CreateServerRequest);
registerType("strims.chat.v1.CreateServerResponse", CreateServerResponse);
registerType("strims.chat.v1.UpdateServerRequest", UpdateServerRequest);
registerType("strims.chat.v1.UpdateServerResponse", UpdateServerResponse);
registerType("strims.chat.v1.DeleteServerRequest", DeleteServerRequest);
registerType("strims.chat.v1.DeleteServerResponse", DeleteServerResponse);
registerType("strims.chat.v1.GetServerRequest", GetServerRequest);
registerType("strims.chat.v1.GetServerResponse", GetServerResponse);
registerType("strims.chat.v1.ListServersRequest", ListServersRequest);
registerType("strims.chat.v1.ListServersResponse", ListServersResponse);
registerType("strims.chat.v1.CreateEmoteRequest", CreateEmoteRequest);
registerType("strims.chat.v1.CreateEmoteResponse", CreateEmoteResponse);
registerType("strims.chat.v1.UpdateEmoteRequest", UpdateEmoteRequest);
registerType("strims.chat.v1.UpdateEmoteResponse", UpdateEmoteResponse);
registerType("strims.chat.v1.DeleteEmoteRequest", DeleteEmoteRequest);
registerType("strims.chat.v1.DeleteEmoteResponse", DeleteEmoteResponse);
registerType("strims.chat.v1.GetEmoteRequest", GetEmoteRequest);
registerType("strims.chat.v1.GetEmoteResponse", GetEmoteResponse);
registerType("strims.chat.v1.ListEmotesRequest", ListEmotesRequest);
registerType("strims.chat.v1.ListEmotesResponse", ListEmotesResponse);
registerType("strims.chat.v1.CreateModifierRequest", CreateModifierRequest);
registerType("strims.chat.v1.CreateModifierResponse", CreateModifierResponse);
registerType("strims.chat.v1.UpdateModifierRequest", UpdateModifierRequest);
registerType("strims.chat.v1.UpdateModifierResponse", UpdateModifierResponse);
registerType("strims.chat.v1.DeleteModifierRequest", DeleteModifierRequest);
registerType("strims.chat.v1.DeleteModifierResponse", DeleteModifierResponse);
registerType("strims.chat.v1.GetModifierRequest", GetModifierRequest);
registerType("strims.chat.v1.GetModifierResponse", GetModifierResponse);
registerType("strims.chat.v1.ListModifiersRequest", ListModifiersRequest);
registerType("strims.chat.v1.ListModifiersResponse", ListModifiersResponse);
registerType("strims.chat.v1.CreateTagRequest", CreateTagRequest);
registerType("strims.chat.v1.CreateTagResponse", CreateTagResponse);
registerType("strims.chat.v1.UpdateTagRequest", UpdateTagRequest);
registerType("strims.chat.v1.UpdateTagResponse", UpdateTagResponse);
registerType("strims.chat.v1.DeleteTagRequest", DeleteTagRequest);
registerType("strims.chat.v1.DeleteTagResponse", DeleteTagResponse);
registerType("strims.chat.v1.GetTagRequest", GetTagRequest);
registerType("strims.chat.v1.GetTagResponse", GetTagResponse);
registerType("strims.chat.v1.ListTagsRequest", ListTagsRequest);
registerType("strims.chat.v1.ListTagsResponse", ListTagsResponse);
registerType("strims.chat.v1.SyncAssetsRequest", SyncAssetsRequest);
registerType("strims.chat.v1.SyncAssetsResponse", SyncAssetsResponse);
registerType("strims.chat.v1.OpenClientRequest", OpenClientRequest);
registerType("strims.chat.v1.OpenClientResponse", OpenClientResponse);
registerType("strims.chat.v1.ClientSendMessageRequest", ClientSendMessageRequest);
registerType("strims.chat.v1.ClientSendMessageResponse", ClientSendMessageResponse);
registerType("strims.chat.v1.SetUIConfigRequest", SetUIConfigRequest);
registerType("strims.chat.v1.SetUIConfigResponse", SetUIConfigResponse);
registerType("strims.chat.v1.GetUIConfigRequest", GetUIConfigRequest);
registerType("strims.chat.v1.GetUIConfigResponse", GetUIConfigResponse);
registerType("strims.chat.v1.SendMessageRequest", SendMessageRequest);
registerType("strims.chat.v1.SendMessageResponse", SendMessageResponse);

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

export const registerChatServerFrontendService = (host: strims_rpc_Service, service: ChatServerFrontendService): void => {
  host.registerMethod<CreateServerRequest, CreateServerResponse>("strims.chat.v1.ChatServerFrontend.CreateServer", service.createServer.bind(service));
  host.registerMethod<UpdateServerRequest, UpdateServerResponse>("strims.chat.v1.ChatServerFrontend.UpdateServer", service.updateServer.bind(service));
  host.registerMethod<DeleteServerRequest, DeleteServerResponse>("strims.chat.v1.ChatServerFrontend.DeleteServer", service.deleteServer.bind(service));
  host.registerMethod<GetServerRequest, GetServerResponse>("strims.chat.v1.ChatServerFrontend.GetServer", service.getServer.bind(service));
  host.registerMethod<ListServersRequest, ListServersResponse>("strims.chat.v1.ChatServerFrontend.ListServers", service.listServers.bind(service));
  host.registerMethod<CreateEmoteRequest, CreateEmoteResponse>("strims.chat.v1.ChatServerFrontend.CreateEmote", service.createEmote.bind(service));
  host.registerMethod<UpdateEmoteRequest, UpdateEmoteResponse>("strims.chat.v1.ChatServerFrontend.UpdateEmote", service.updateEmote.bind(service));
  host.registerMethod<DeleteEmoteRequest, DeleteEmoteResponse>("strims.chat.v1.ChatServerFrontend.DeleteEmote", service.deleteEmote.bind(service));
  host.registerMethod<GetEmoteRequest, GetEmoteResponse>("strims.chat.v1.ChatServerFrontend.GetEmote", service.getEmote.bind(service));
  host.registerMethod<ListEmotesRequest, ListEmotesResponse>("strims.chat.v1.ChatServerFrontend.ListEmotes", service.listEmotes.bind(service));
  host.registerMethod<CreateModifierRequest, CreateModifierResponse>("strims.chat.v1.ChatServerFrontend.CreateModifier", service.createModifier.bind(service));
  host.registerMethod<UpdateModifierRequest, UpdateModifierResponse>("strims.chat.v1.ChatServerFrontend.UpdateModifier", service.updateModifier.bind(service));
  host.registerMethod<DeleteModifierRequest, DeleteModifierResponse>("strims.chat.v1.ChatServerFrontend.DeleteModifier", service.deleteModifier.bind(service));
  host.registerMethod<GetModifierRequest, GetModifierResponse>("strims.chat.v1.ChatServerFrontend.GetModifier", service.getModifier.bind(service));
  host.registerMethod<ListModifiersRequest, ListModifiersResponse>("strims.chat.v1.ChatServerFrontend.ListModifiers", service.listModifiers.bind(service));
  host.registerMethod<CreateTagRequest, CreateTagResponse>("strims.chat.v1.ChatServerFrontend.CreateTag", service.createTag.bind(service));
  host.registerMethod<UpdateTagRequest, UpdateTagResponse>("strims.chat.v1.ChatServerFrontend.UpdateTag", service.updateTag.bind(service));
  host.registerMethod<DeleteTagRequest, DeleteTagResponse>("strims.chat.v1.ChatServerFrontend.DeleteTag", service.deleteTag.bind(service));
  host.registerMethod<GetTagRequest, GetTagResponse>("strims.chat.v1.ChatServerFrontend.GetTag", service.getTag.bind(service));
  host.registerMethod<ListTagsRequest, ListTagsResponse>("strims.chat.v1.ChatServerFrontend.ListTags", service.listTags.bind(service));
  host.registerMethod<SyncAssetsRequest, SyncAssetsResponse>("strims.chat.v1.ChatServerFrontend.SyncAssets", service.syncAssets.bind(service));
}

export class ChatServerFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createServer(req?: ICreateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateServer", new CreateServerRequest(req)), opts);
  }

  public updateServer(req?: IUpdateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateServer", new UpdateServerRequest(req)), opts);
  }

  public deleteServer(req?: IDeleteServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteServer", new DeleteServerRequest(req)), opts);
  }

  public getServer(req?: IGetServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetServer", new GetServerRequest(req)), opts);
  }

  public listServers(req?: IListServersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListServersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListServers", new ListServersRequest(req)), opts);
  }

  public createEmote(req?: ICreateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateEmote", new CreateEmoteRequest(req)), opts);
  }

  public updateEmote(req?: IUpdateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateEmote", new UpdateEmoteRequest(req)), opts);
  }

  public deleteEmote(req?: IDeleteEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteEmote", new DeleteEmoteRequest(req)), opts);
  }

  public getEmote(req?: IGetEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetEmote", new GetEmoteRequest(req)), opts);
  }

  public listEmotes(req?: IListEmotesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListEmotesResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListEmotes", new ListEmotesRequest(req)), opts);
  }

  public createModifier(req?: ICreateModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateModifier", new CreateModifierRequest(req)), opts);
  }

  public updateModifier(req?: IUpdateModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateModifier", new UpdateModifierRequest(req)), opts);
  }

  public deleteModifier(req?: IDeleteModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteModifier", new DeleteModifierRequest(req)), opts);
  }

  public getModifier(req?: IGetModifierRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetModifierResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetModifier", new GetModifierRequest(req)), opts);
  }

  public listModifiers(req?: IListModifiersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListModifiersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListModifiers", new ListModifiersRequest(req)), opts);
  }

  public createTag(req?: ICreateTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.CreateTag", new CreateTagRequest(req)), opts);
  }

  public updateTag(req?: IUpdateTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.UpdateTag", new UpdateTagRequest(req)), opts);
  }

  public deleteTag(req?: IDeleteTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.DeleteTag", new DeleteTagRequest(req)), opts);
  }

  public getTag(req?: IGetTagRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetTagResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.GetTag", new GetTagRequest(req)), opts);
  }

  public listTags(req?: IListTagsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListTagsResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.ListTags", new ListTagsRequest(req)), opts);
  }

  public syncAssets(req?: ISyncAssetsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SyncAssetsResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatServerFrontend.SyncAssets", new SyncAssetsRequest(req)), opts);
  }
}

export interface ChatFrontendService {
  openClient(req: OpenClientRequest, call: strims_rpc_Call): GenericReadable<OpenClientResponse>;
  clientSendMessage(req: ClientSendMessageRequest, call: strims_rpc_Call): Promise<ClientSendMessageResponse> | ClientSendMessageResponse;
  setUIConfig(req: SetUIConfigRequest, call: strims_rpc_Call): Promise<SetUIConfigResponse> | SetUIConfigResponse;
  getUIConfig(req: GetUIConfigRequest, call: strims_rpc_Call): Promise<GetUIConfigResponse> | GetUIConfigResponse;
}

export const registerChatFrontendService = (host: strims_rpc_Service, service: ChatFrontendService): void => {
  host.registerMethod<OpenClientRequest, OpenClientResponse>("strims.chat.v1.ChatFrontend.OpenClient", service.openClient.bind(service));
  host.registerMethod<ClientSendMessageRequest, ClientSendMessageResponse>("strims.chat.v1.ChatFrontend.ClientSendMessage", service.clientSendMessage.bind(service));
  host.registerMethod<SetUIConfigRequest, SetUIConfigResponse>("strims.chat.v1.ChatFrontend.SetUIConfig", service.setUIConfig.bind(service));
  host.registerMethod<GetUIConfigRequest, GetUIConfigResponse>("strims.chat.v1.ChatFrontend.GetUIConfig", service.getUIConfig.bind(service));
}

export class ChatFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public openClient(req?: IOpenClientRequest): GenericReadable<OpenClientResponse> {
    return this.host.expectMany(this.host.call("strims.chat.v1.ChatFrontend.OpenClient", new OpenClientRequest(req)));
  }

  public clientSendMessage(req?: IClientSendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ClientSendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientSendMessage", new ClientSendMessageRequest(req)), opts);
  }

  public setUIConfig(req?: ISetUIConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SetUIConfigResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.SetUIConfig", new SetUIConfigRequest(req)), opts);
  }

  public getUIConfig(req?: IGetUIConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetUIConfigResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.GetUIConfig", new GetUIConfigRequest(req)), opts);
  }
}

export interface ChatService {
  sendMessage(req: SendMessageRequest, call: strims_rpc_Call): Promise<SendMessageResponse> | SendMessageResponse;
}

export const registerChatService = (host: strims_rpc_Service, service: ChatService): void => {
  host.registerMethod<SendMessageRequest, SendMessageResponse>("strims.chat.v1.Chat.SendMessage", service.sendMessage.bind(service));
}

export class ChatClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public sendMessage(req?: ISendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.SendMessage", new SendMessageRequest(req)), opts);
  }
}

