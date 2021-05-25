import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  ICreateChatServerRequest,
  CreateChatServerRequest,
  CreateChatServerResponse,
  IUpdateChatServerRequest,
  UpdateChatServerRequest,
  UpdateChatServerResponse,
  IDeleteChatServerRequest,
  DeleteChatServerRequest,
  DeleteChatServerResponse,
  IGetChatServerRequest,
  GetChatServerRequest,
  GetChatServerResponse,
  IListChatServersRequest,
  ListChatServersRequest,
  ListChatServersResponse,
  IOpenChatServerRequest,
  OpenChatServerRequest,
  ChatServerEvent,
  IOpenChatClientRequest,
  OpenChatClientRequest,
  ChatClientEvent,
  ICallChatClientRequest,
  CallChatClientRequest,
  CallChatClientResponse,
} from "./chat";

registerType("strims.chat.v1.CreateChatServerRequest", CreateChatServerRequest);
registerType("strims.chat.v1.CreateChatServerResponse", CreateChatServerResponse);
registerType("strims.chat.v1.UpdateChatServerRequest", UpdateChatServerRequest);
registerType("strims.chat.v1.UpdateChatServerResponse", UpdateChatServerResponse);
registerType("strims.chat.v1.DeleteChatServerRequest", DeleteChatServerRequest);
registerType("strims.chat.v1.DeleteChatServerResponse", DeleteChatServerResponse);
registerType("strims.chat.v1.GetChatServerRequest", GetChatServerRequest);
registerType("strims.chat.v1.GetChatServerResponse", GetChatServerResponse);
registerType("strims.chat.v1.ListChatServersRequest", ListChatServersRequest);
registerType("strims.chat.v1.ListChatServersResponse", ListChatServersResponse);
registerType("strims.chat.v1.OpenChatServerRequest", OpenChatServerRequest);
registerType("strims.chat.v1.ChatServerEvent", ChatServerEvent);
registerType("strims.chat.v1.OpenChatClientRequest", OpenChatClientRequest);
registerType("strims.chat.v1.ChatClientEvent", ChatClientEvent);
registerType("strims.chat.v1.CallChatClientRequest", CallChatClientRequest);
registerType("strims.chat.v1.CallChatClientResponse", CallChatClientResponse);

export interface ChatService {
  createServer(req: CreateChatServerRequest, call: strims_rpc_Call): Promise<CreateChatServerResponse> | CreateChatServerResponse;
  updateServer(req: UpdateChatServerRequest, call: strims_rpc_Call): Promise<UpdateChatServerResponse> | UpdateChatServerResponse;
  deleteServer(req: DeleteChatServerRequest, call: strims_rpc_Call): Promise<DeleteChatServerResponse> | DeleteChatServerResponse;
  getServer(req: GetChatServerRequest, call: strims_rpc_Call): Promise<GetChatServerResponse> | GetChatServerResponse;
  listServers(req: ListChatServersRequest, call: strims_rpc_Call): Promise<ListChatServersResponse> | ListChatServersResponse;
  openServer(req: OpenChatServerRequest, call: strims_rpc_Call): GenericReadable<ChatServerEvent>;
  openClient(req: OpenChatClientRequest, call: strims_rpc_Call): GenericReadable<ChatClientEvent>;
  callClient(req: CallChatClientRequest, call: strims_rpc_Call): Promise<CallChatClientResponse> | CallChatClientResponse;
}

export const registerChatService = (host: strims_rpc_Service, service: ChatService): void => {
  host.registerMethod<CreateChatServerRequest, CreateChatServerResponse>("strims.chat.v1.Chat.CreateServer", service.createServer.bind(service));
  host.registerMethod<UpdateChatServerRequest, UpdateChatServerResponse>("strims.chat.v1.Chat.UpdateServer", service.updateServer.bind(service));
  host.registerMethod<DeleteChatServerRequest, DeleteChatServerResponse>("strims.chat.v1.Chat.DeleteServer", service.deleteServer.bind(service));
  host.registerMethod<GetChatServerRequest, GetChatServerResponse>("strims.chat.v1.Chat.GetServer", service.getServer.bind(service));
  host.registerMethod<ListChatServersRequest, ListChatServersResponse>("strims.chat.v1.Chat.ListServers", service.listServers.bind(service));
  host.registerMethod<OpenChatServerRequest, ChatServerEvent>("strims.chat.v1.Chat.OpenServer", service.openServer.bind(service));
  host.registerMethod<OpenChatClientRequest, ChatClientEvent>("strims.chat.v1.Chat.OpenClient", service.openClient.bind(service));
  host.registerMethod<CallChatClientRequest, CallChatClientResponse>("strims.chat.v1.Chat.CallClient", service.callClient.bind(service));
}

export class ChatClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createServer(req?: ICreateChatServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CreateChatServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.CreateServer", new CreateChatServerRequest(req)), opts);
  }

  public updateServer(req?: IUpdateChatServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<UpdateChatServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.UpdateServer", new UpdateChatServerRequest(req)), opts);
  }

  public deleteServer(req?: IDeleteChatServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DeleteChatServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.DeleteServer", new DeleteChatServerRequest(req)), opts);
  }

  public getServer(req?: IGetChatServerRequest, opts?: strims_rpc_UnaryCallOptions): Promise<GetChatServerResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.GetServer", new GetChatServerRequest(req)), opts);
  }

  public listServers(req?: IListChatServersRequest, opts?: strims_rpc_UnaryCallOptions): Promise<ListChatServersResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.ListServers", new ListChatServersRequest(req)), opts);
  }

  public openServer(req?: IOpenChatServerRequest): GenericReadable<ChatServerEvent> {
    return this.host.expectMany(this.host.call("strims.chat.v1.Chat.OpenServer", new OpenChatServerRequest(req)));
  }

  public openClient(req?: IOpenChatClientRequest): GenericReadable<ChatClientEvent> {
    return this.host.expectMany(this.host.call("strims.chat.v1.Chat.OpenClient", new OpenChatClientRequest(req)));
  }

  public callClient(req?: ICallChatClientRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CallChatClientResponse> {
    return this.host.expectOne(this.host.call("strims.chat.v1.Chat.CallClient", new CallChatClientRequest(req)), opts);
  }
}

