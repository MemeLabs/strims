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
  setUIConfig(req: SetUIConfigRequest, call: strims_rpc_Call): Promise<SetUIConfigResponse> | SetUIConfigResponse;
  getUIConfig(req: GetUIConfigRequest, call: strims_rpc_Call): Promise<GetUIConfigResponse> | GetUIConfigResponse;
}

export const registerChatFrontendService = (host: strims_rpc_Service, service: ChatFrontendService): void => {
  host.registerMethod<OpenClientRequest, OpenClientResponse>("strims.chat.v1.ChatFrontend.OpenClient", service.openClient.bind(service), OpenClientRequest);
  host.registerMethod<ClientSendMessageRequest, ClientSendMessageResponse>("strims.chat.v1.ChatFrontend.ClientSendMessage", service.clientSendMessage.bind(service), ClientSendMessageRequest);
  host.registerMethod<SetUIConfigRequest, SetUIConfigResponse>("strims.chat.v1.ChatFrontend.SetUIConfig", service.setUIConfig.bind(service), SetUIConfigRequest);
  host.registerMethod<GetUIConfigRequest, GetUIConfigResponse>("strims.chat.v1.ChatFrontend.GetUIConfig", service.getUIConfig.bind(service), GetUIConfigRequest);
}

export class ChatFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public openClient(req?: IOpenClientRequest): GenericReadable<OpenClientResponse> {
    return this.host.expectMany(this.host.call("strims.chat.v1.ChatFrontend.OpenClient", new OpenClientRequest(req)), OpenClientResponse);
  }

  public clientSendMessage(req?: IClientSendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ClientSendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientSendMessage", new ClientSendMessageRequest(req)), ClientSendMessageResponse, opts);
  }

  public setUIConfig(req?: ISetUIConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SetUIConfigResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.SetUIConfig", new SetUIConfigRequest(req)), SetUIConfigResponse, opts);
  }

  public getUIConfig(req?: IGetUIConfigRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetUIConfigResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.GetUIConfig", new GetUIConfigRequest(req)), GetUIConfigResponse, opts);
  }
}

export interface ChatService {
  sendMessage(req: SendMessageRequest, call: strims_rpc_Call): Promise<SendMessageResponse> | SendMessageResponse;
}

export const registerChatService = (host: strims_rpc_Service, service: ChatService): void => {
  host.registerMethod<SendMessageRequest, SendMessageResponse>("strims.chat.v1.Chat.SendMessage", service.sendMessage.bind(service), SendMessageRequest);
}

export class ChatClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public sendMessage(req?: ISendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.SendMessage", new SendMessageRequest(req)), SendMessageResponse, opts);
  }
}

