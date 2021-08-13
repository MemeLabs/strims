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
  IOpenClientRequest,
  OpenClientRequest,
  OpenClientResponse,
  IClientSendMessageRequest,
  ClientSendMessageRequest,
  ClientSendMessageResponse,
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
registerType("strims.chat.v1.OpenClientRequest", OpenClientRequest);
registerType("strims.chat.v1.OpenClientResponse", OpenClientResponse);
registerType("strims.chat.v1.ClientSendMessageRequest", ClientSendMessageRequest);
registerType("strims.chat.v1.ClientSendMessageResponse", ClientSendMessageResponse);
registerType("strims.chat.v1.SendMessageRequest", SendMessageRequest);
registerType("strims.chat.v1.SendMessageResponse", SendMessageResponse);

export interface ChatFrontendService {
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
  openClient(req: OpenClientRequest, call: strims_rpc_Call): GenericReadable<OpenClientResponse>;
  clientSendMessage(req: ClientSendMessageRequest, call: strims_rpc_Call): Promise<ClientSendMessageResponse> | ClientSendMessageResponse;
}

export const registerChatFrontendService = (host: strims_rpc_Service, service: ChatFrontendService): void => {
  host.registerMethod<CreateServerRequest, CreateServerResponse>("strims.chat.v1.ChatFrontend.CreateServer", service.createServer.bind(service));
  host.registerMethod<UpdateServerRequest, UpdateServerResponse>("strims.chat.v1.ChatFrontend.UpdateServer", service.updateServer.bind(service));
  host.registerMethod<DeleteServerRequest, DeleteServerResponse>("strims.chat.v1.ChatFrontend.DeleteServer", service.deleteServer.bind(service));
  host.registerMethod<GetServerRequest, GetServerResponse>("strims.chat.v1.ChatFrontend.GetServer", service.getServer.bind(service));
  host.registerMethod<ListServersRequest, ListServersResponse>("strims.chat.v1.ChatFrontend.ListServers", service.listServers.bind(service));
  host.registerMethod<CreateEmoteRequest, CreateEmoteResponse>("strims.chat.v1.ChatFrontend.CreateEmote", service.createEmote.bind(service));
  host.registerMethod<UpdateEmoteRequest, UpdateEmoteResponse>("strims.chat.v1.ChatFrontend.UpdateEmote", service.updateEmote.bind(service));
  host.registerMethod<DeleteEmoteRequest, DeleteEmoteResponse>("strims.chat.v1.ChatFrontend.DeleteEmote", service.deleteEmote.bind(service));
  host.registerMethod<GetEmoteRequest, GetEmoteResponse>("strims.chat.v1.ChatFrontend.GetEmote", service.getEmote.bind(service));
  host.registerMethod<ListEmotesRequest, ListEmotesResponse>("strims.chat.v1.ChatFrontend.ListEmotes", service.listEmotes.bind(service));
  host.registerMethod<OpenClientRequest, OpenClientResponse>("strims.chat.v1.ChatFrontend.OpenClient", service.openClient.bind(service));
  host.registerMethod<ClientSendMessageRequest, ClientSendMessageResponse>("strims.chat.v1.ChatFrontend.ClientSendMessage", service.clientSendMessage.bind(service));
}

export class ChatFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createServer(req?: ICreateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.CreateServer", new CreateServerRequest(req)), opts);
  }

  public updateServer(req?: IUpdateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.UpdateServer", new UpdateServerRequest(req)), opts);
  }

  public deleteServer(req?: IDeleteServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.DeleteServer", new DeleteServerRequest(req)), opts);
  }

  public getServer(req?: IGetServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.GetServer", new GetServerRequest(req)), opts);
  }

  public listServers(req?: IListServersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListServersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ListServers", new ListServersRequest(req)), opts);
  }

  public createEmote(req?: ICreateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.CreateEmote", new CreateEmoteRequest(req)), opts);
  }

  public updateEmote(req?: IUpdateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.UpdateEmote", new UpdateEmoteRequest(req)), opts);
  }

  public deleteEmote(req?: IDeleteEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.DeleteEmote", new DeleteEmoteRequest(req)), opts);
  }

  public getEmote(req?: IGetEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.GetEmote", new GetEmoteRequest(req)), opts);
  }

  public listEmotes(req?: IListEmotesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListEmotesResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ListEmotes", new ListEmotesRequest(req)), opts);
  }

  public openClient(req?: IOpenClientRequest): GenericReadable<OpenClientResponse> {
    return this.host.expectMany(this.host.call("strims.chat.v1.ChatFrontend.OpenClient", new OpenClientRequest(req)));
  }

  public clientSendMessage(req?: IClientSendMessageRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ClientSendMessageResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.ChatFrontend.ClientSendMessage", new ClientSendMessageRequest(req)), opts);
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

