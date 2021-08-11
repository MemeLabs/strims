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
  IOpenServerRequest,
  OpenServerRequest,
  ServerEvent,
  IOpenClientRequest,
  OpenClientRequest,
  ClientEvent,
  ICallClientRequest,
  CallClientRequest,
  CallClientResponse,
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
registerType("strims.chat.v1.OpenServerRequest", OpenServerRequest);
registerType("strims.chat.v1.ServerEvent", ServerEvent);
registerType("strims.chat.v1.OpenClientRequest", OpenClientRequest);
registerType("strims.chat.v1.ClientEvent", ClientEvent);
registerType("strims.chat.v1.CallClientRequest", CallClientRequest);
registerType("strims.chat.v1.CallClientResponse", CallClientResponse);

export interface ChatService {
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
  openServer(req: OpenServerRequest, call: strims_rpc_Call): GenericReadable<ServerEvent>;
  openClient(req: OpenClientRequest, call: strims_rpc_Call): GenericReadable<ClientEvent>;
  callClient(req: CallClientRequest, call: strims_rpc_Call): Promise<CallClientResponse> | CallClientResponse;
}

export const registerChatService = (host: strims_rpc_Service, service: ChatService): void => {
  host.registerMethod<CreateServerRequest, CreateServerResponse>("strims.chat.v1.Chat.CreateServer", service.createServer.bind(service));
  host.registerMethod<UpdateServerRequest, UpdateServerResponse>("strims.chat.v1.Chat.UpdateServer", service.updateServer.bind(service));
  host.registerMethod<DeleteServerRequest, DeleteServerResponse>("strims.chat.v1.Chat.DeleteServer", service.deleteServer.bind(service));
  host.registerMethod<GetServerRequest, GetServerResponse>("strims.chat.v1.Chat.GetServer", service.getServer.bind(service));
  host.registerMethod<ListServersRequest, ListServersResponse>("strims.chat.v1.Chat.ListServers", service.listServers.bind(service));
  host.registerMethod<CreateEmoteRequest, CreateEmoteResponse>("strims.chat.v1.Chat.CreateEmote", service.createEmote.bind(service));
  host.registerMethod<UpdateEmoteRequest, UpdateEmoteResponse>("strims.chat.v1.Chat.UpdateEmote", service.updateEmote.bind(service));
  host.registerMethod<DeleteEmoteRequest, DeleteEmoteResponse>("strims.chat.v1.Chat.DeleteEmote", service.deleteEmote.bind(service));
  host.registerMethod<GetEmoteRequest, GetEmoteResponse>("strims.chat.v1.Chat.GetEmote", service.getEmote.bind(service));
  host.registerMethod<ListEmotesRequest, ListEmotesResponse>("strims.chat.v1.Chat.ListEmotes", service.listEmotes.bind(service));
  host.registerMethod<OpenServerRequest, ServerEvent>("strims.chat.v1.Chat.OpenServer", service.openServer.bind(service));
  host.registerMethod<OpenClientRequest, ClientEvent>("strims.chat.v1.Chat.OpenClient", service.openClient.bind(service));
  host.registerMethod<CallClientRequest, CallClientResponse>("strims.chat.v1.Chat.CallClient", service.callClient.bind(service));
}

export class ChatClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createServer(req?: ICreateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.CreateServer", new CreateServerRequest(req)), opts);
  }

  public updateServer(req?: IUpdateServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.UpdateServer", new UpdateServerRequest(req)), opts);
  }

  public deleteServer(req?: IDeleteServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.DeleteServer", new DeleteServerRequest(req)), opts);
  }

  public getServer(req?: IGetServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.GetServer", new GetServerRequest(req)), opts);
  }

  public listServers(req?: IListServersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListServersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.ListServers", new ListServersRequest(req)), opts);
  }

  public createEmote(req?: ICreateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.CreateEmote", new CreateEmoteRequest(req)), opts);
  }

  public updateEmote(req?: IUpdateEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.UpdateEmote", new UpdateEmoteRequest(req)), opts);
  }

  public deleteEmote(req?: IDeleteEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.DeleteEmote", new DeleteEmoteRequest(req)), opts);
  }

  public getEmote(req?: IGetEmoteRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetEmoteResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.GetEmote", new GetEmoteRequest(req)), opts);
  }

  public listEmotes(req?: IListEmotesRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListEmotesResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.ListEmotes", new ListEmotesRequest(req)), opts);
  }

  public openServer(req?: IOpenServerRequest): GenericReadable<ServerEvent> {
    return this.host.expectMany(this.host.call("strims.chat.v1.Chat.OpenServer", new OpenServerRequest(req)));
  }

  public openClient(req?: IOpenClientRequest): GenericReadable<ClientEvent> {
    return this.host.expectMany(this.host.call("strims.chat.v1.Chat.OpenClient", new OpenClientRequest(req)));
  }

  public callClient(req?: ICallClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CallClientResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.CallClient", new CallClientRequest(req)), opts);
  }
}

