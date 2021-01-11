import { RPCHost } from "../../../../../rpc/host";
import { registerType } from "../../../../../pb/registry";
import { Readable as GenericReadable } from "../../../../../rpc/stream";

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

registerType(".strims.chat.v1.CreateChatServerRequest", CreateChatServerRequest);
registerType(".strims.chat.v1.CreateChatServerResponse", CreateChatServerResponse);
registerType(".strims.chat.v1.UpdateChatServerRequest", UpdateChatServerRequest);
registerType(".strims.chat.v1.UpdateChatServerResponse", UpdateChatServerResponse);
registerType(".strims.chat.v1.DeleteChatServerRequest", DeleteChatServerRequest);
registerType(".strims.chat.v1.DeleteChatServerResponse", DeleteChatServerResponse);
registerType(".strims.chat.v1.GetChatServerRequest", GetChatServerRequest);
registerType(".strims.chat.v1.GetChatServerResponse", GetChatServerResponse);
registerType(".strims.chat.v1.ListChatServersRequest", ListChatServersRequest);
registerType(".strims.chat.v1.ListChatServersResponse", ListChatServersResponse);
registerType(".strims.chat.v1.OpenChatServerRequest", OpenChatServerRequest);
registerType(".strims.chat.v1.ChatServerEvent", ChatServerEvent);
registerType(".strims.chat.v1.OpenChatClientRequest", OpenChatClientRequest);
registerType(".strims.chat.v1.ChatClientEvent", ChatClientEvent);
registerType(".strims.chat.v1.CallChatClientRequest", CallChatClientRequest);
registerType(".strims.chat.v1.CallChatClientResponse", CallChatClientResponse);

export class ChatClient {
  constructor(private readonly host: RPCHost) {}

  public createServer(arg: ICreateChatServerRequest = new CreateChatServerRequest()): Promise<CreateChatServerResponse> {
    return this.host.expectOne(this.host.call(".strims.chat.v1.Chat.CreateServer", new CreateChatServerRequest(arg)));
  }

  public updateServer(arg: IUpdateChatServerRequest = new UpdateChatServerRequest()): Promise<UpdateChatServerResponse> {
    return this.host.expectOne(this.host.call(".strims.chat.v1.Chat.UpdateServer", new UpdateChatServerRequest(arg)));
  }

  public deleteServer(arg: IDeleteChatServerRequest = new DeleteChatServerRequest()): Promise<DeleteChatServerResponse> {
    return this.host.expectOne(this.host.call(".strims.chat.v1.Chat.DeleteServer", new DeleteChatServerRequest(arg)));
  }

  public getServer(arg: IGetChatServerRequest = new GetChatServerRequest()): Promise<GetChatServerResponse> {
    return this.host.expectOne(this.host.call(".strims.chat.v1.Chat.GetServer", new GetChatServerRequest(arg)));
  }

  public listServers(arg: IListChatServersRequest = new ListChatServersRequest()): Promise<ListChatServersResponse> {
    return this.host.expectOne(this.host.call(".strims.chat.v1.Chat.ListServers", new ListChatServersRequest(arg)));
  }

  public openServer(arg: IOpenChatServerRequest = new OpenChatServerRequest()): GenericReadable<ChatServerEvent> {
    return this.host.expectMany(this.host.call(".strims.chat.v1.Chat.OpenServer", new OpenChatServerRequest(arg)));
  }

  public openClient(arg: IOpenChatClientRequest = new OpenChatClientRequest()): GenericReadable<ChatClientEvent> {
    return this.host.expectMany(this.host.call(".strims.chat.v1.Chat.OpenClient", new OpenChatClientRequest(arg)));
  }

  public callClient(arg: ICallChatClientRequest = new CallChatClientRequest()): Promise<CallChatClientResponse> {
    return this.host.expectOne(this.host.call(".strims.chat.v1.Chat.CallClient", new CallChatClientRequest(arg)));
  }
}

