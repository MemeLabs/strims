import Reader from "../../../../lib/pb/reader";
import Writer from "../../../../lib/pb/writer";

import {
  Key as strims_type_Key,
  IKey as strims_type_IKey,
} from "../../type/key";

export interface ICreateChatServerRequest {
  networkKey?: Uint8Array;
  chatRoom?: IChatRoom | undefined;
}

export class CreateChatServerRequest {
  networkKey: Uint8Array = new Uint8Array();
  chatRoom: ChatRoom | undefined;

  constructor(v?: ICreateChatServerRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.chatRoom = v?.chatRoom && new ChatRoom(v.chatRoom);
  }

  static encode(m: CreateChatServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey) w.uint32(18).bytes(m.networkKey);
    if (m.chatRoom) ChatRoom.encode(m.chatRoom, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateChatServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateChatServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 2:
        m.networkKey = r.bytes();
        break;
        case 3:
        m.chatRoom = ChatRoom.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ICreateChatServerResponse {
  chatServer?: IChatServer | undefined;
}

export class CreateChatServerResponse {
  chatServer: ChatServer | undefined;

  constructor(v?: ICreateChatServerResponse) {
    this.chatServer = v?.chatServer && new ChatServer(v.chatServer);
  }

  static encode(m: CreateChatServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.chatServer) ChatServer.encode(m.chatServer, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateChatServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateChatServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.chatServer = ChatServer.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IUpdateChatServerRequest {
  id?: bigint;
  networkKey?: Uint8Array;
  serverKey?: IChatRoom | undefined;
}

export class UpdateChatServerRequest {
  id: bigint = BigInt(0);
  networkKey: Uint8Array = new Uint8Array();
  serverKey: ChatRoom | undefined;

  constructor(v?: IUpdateChatServerRequest) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey && new ChatRoom(v.serverKey);
  }

  static encode(m: UpdateChatServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey) w.uint32(18).bytes(m.networkKey);
    if (m.serverKey) ChatRoom.encode(m.serverKey, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateChatServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateChatServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.networkKey = r.bytes();
        break;
        case 3:
        m.serverKey = ChatRoom.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IUpdateChatServerResponse {
  chatServer?: IChatServer | undefined;
}

export class UpdateChatServerResponse {
  chatServer: ChatServer | undefined;

  constructor(v?: IUpdateChatServerResponse) {
    this.chatServer = v?.chatServer && new ChatServer(v.chatServer);
  }

  static encode(m: UpdateChatServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.chatServer) ChatServer.encode(m.chatServer, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateChatServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateChatServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.chatServer = ChatServer.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDeleteChatServerRequest {
  id?: bigint;
}

export class DeleteChatServerRequest {
  id: bigint = BigInt(0);

  constructor(v?: IDeleteChatServerRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteChatServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteChatServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteChatServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDeleteChatServerResponse {
}

export class DeleteChatServerResponse {

  constructor(v?: IDeleteChatServerResponse) {
    // noop
  }

  static encode(m: DeleteChatServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteChatServerResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteChatServerResponse();
  }
}

export interface IGetChatServerRequest {
  id?: bigint;
}

export class GetChatServerRequest {
  id: bigint = BigInt(0);

  constructor(v?: IGetChatServerRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetChatServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetChatServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetChatServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IGetChatServerResponse {
  chatServer?: IChatServer | undefined;
}

export class GetChatServerResponse {
  chatServer: ChatServer | undefined;

  constructor(v?: IGetChatServerResponse) {
    this.chatServer = v?.chatServer && new ChatServer(v.chatServer);
  }

  static encode(m: GetChatServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.chatServer) ChatServer.encode(m.chatServer, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetChatServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetChatServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.chatServer = ChatServer.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IListChatServersRequest {
}

export class ListChatServersRequest {

  constructor(v?: IListChatServersRequest) {
    // noop
  }

  static encode(m: ListChatServersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListChatServersRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListChatServersRequest();
  }
}

export interface IListChatServersResponse {
  chatServers?: IChatServer[];
}

export class ListChatServersResponse {
  chatServers: ChatServer[] = [];

  constructor(v?: IListChatServersResponse) {
    if (v?.chatServers) this.chatServers = v.chatServers.map(v => new ChatServer(v));
  }

  static encode(m: ListChatServersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.chatServers) ChatServer.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListChatServersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListChatServersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.chatServers.push(ChatServer.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IOpenChatServerRequest {
  server?: IChatServer | undefined;
}

export class OpenChatServerRequest {
  server: ChatServer | undefined;

  constructor(v?: IOpenChatServerRequest) {
    this.server = v?.server && new ChatServer(v.server);
  }

  static encode(m: OpenChatServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) ChatServer.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenChatServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new OpenChatServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = ChatServer.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IChatServerEvent {
  body?: ChatServerEvent.IBody
}

export class ChatServerEvent {
  body: ChatServerEvent.TBody;

  constructor(v?: IChatServerEvent) {
    this.body = new ChatServerEvent.Body(v?.body);
  }

  static encode(m: ChatServerEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case ChatServerEvent.BodyCase.OPEN:
      ChatServerEvent.Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case ChatServerEvent.BodyCase.CLOSE:
      ChatServerEvent.Close.encode(m.body.close, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChatServerEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChatServerEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new ChatServerEvent.Body({ open: ChatServerEvent.Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new ChatServerEvent.Body({ close: ChatServerEvent.Close.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ChatServerEvent {
  export enum BodyCase {
    NOT_SET = 0,
    OPEN = 1,
    CLOSE = 2,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.OPEN, open: ChatServerEvent.IOpen }
  |{ case?: BodyCase.CLOSE, close: ChatServerEvent.IClose }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: ChatServerEvent.Open }
  |{ case: BodyCase.CLOSE, close: ChatServerEvent.Close }
  >;

  class BodyImpl {
    open: ChatServerEvent.Open;
    close: ChatServerEvent.Close;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new ChatServerEvent.Open(v.open);
      } else
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new ChatServerEvent.Close(v.close);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: ChatServerEvent.IOpen } ? { case: BodyCase.OPEN, open: ChatServerEvent.Open } :
    T extends { close: ChatServerEvent.IClose } ? { case: BodyCase.CLOSE, close: ChatServerEvent.Close } :
    never
    >;
  };

  export interface IOpen {
    serverId?: bigint;
  }

  export class Open {
    serverId: bigint = BigInt(0);

    constructor(v?: IOpen) {
      this.serverId = v?.serverId || BigInt(0);
    }

    static encode(m: Open, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.serverId) w.uint32(8).uint64(m.serverId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Open {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Open();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.serverId = r.uint64();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IClose {
  }

  export class Close {

    constructor(v?: IClose) {
      // noop
    }

    static encode(m: Close, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Close {
      if (r instanceof Reader && length) r.skip(length);
      return new Close();
    }
  }

}

export interface ICallChatServerRequest {
  serverId?: bigint;
  body?: CallChatServerRequest.IBody
}

export class CallChatServerRequest {
  serverId: bigint = BigInt(0);
  body: CallChatServerRequest.TBody;

  constructor(v?: ICallChatServerRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.body = new CallChatServerRequest.Body(v?.body);
  }

  static encode(m: CallChatServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    switch (m.body.case) {
      case CallChatServerRequest.BodyCase.CLOSE:
      CallChatServerRequest.Close.encode(m.body.close, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CallChatServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CallChatServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.body = new CallChatServerRequest.Body({ close: CallChatServerRequest.Close.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CallChatServerRequest {
  export enum BodyCase {
    NOT_SET = 0,
    CLOSE = 2,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.CLOSE, close: CallChatServerRequest.IClose }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.CLOSE, close: CallChatServerRequest.Close }
  >;

  class BodyImpl {
    close: CallChatServerRequest.Close;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new CallChatServerRequest.Close(v.close);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { close: CallChatServerRequest.IClose } ? { case: BodyCase.CLOSE, close: CallChatServerRequest.Close } :
    never
    >;
  };

  export interface IClose {
  }

  export class Close {

    constructor(v?: IClose) {
      // noop
    }

    static encode(m: Close, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Close {
      if (r instanceof Reader && length) r.skip(length);
      return new Close();
    }
  }

}

export interface IOpenChatClientRequest {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
}

export class OpenChatClientRequest {
  networkKey: Uint8Array = new Uint8Array();
  serverKey: Uint8Array = new Uint8Array();

  constructor(v?: IOpenChatClientRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
  }

  static encode(m: OpenChatClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
    if (m.serverKey) w.uint32(18).bytes(m.serverKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenChatClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new OpenChatClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.serverKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IChatClientEvent {
  body?: ChatClientEvent.IBody
}

export class ChatClientEvent {
  body: ChatClientEvent.TBody;

  constructor(v?: IChatClientEvent) {
    this.body = new ChatClientEvent.Body(v?.body);
  }

  static encode(m: ChatClientEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case ChatClientEvent.BodyCase.OPEN:
      ChatClientEvent.Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case ChatClientEvent.BodyCase.MESSAGE:
      ChatClientEvent.Message.encode(m.body.message, w.uint32(18).fork()).ldelim();
      break;
      case ChatClientEvent.BodyCase.CLOSE:
      ChatClientEvent.Close.encode(m.body.close, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChatClientEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChatClientEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new ChatClientEvent.Body({ open: ChatClientEvent.Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new ChatClientEvent.Body({ message: ChatClientEvent.Message.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new ChatClientEvent.Body({ close: ChatClientEvent.Close.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ChatClientEvent {
  export enum BodyCase {
    NOT_SET = 0,
    OPEN = 1,
    MESSAGE = 2,
    CLOSE = 3,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.OPEN, open: ChatClientEvent.IOpen }
  |{ case?: BodyCase.MESSAGE, message: ChatClientEvent.IMessage }
  |{ case?: BodyCase.CLOSE, close: ChatClientEvent.IClose }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: ChatClientEvent.Open }
  |{ case: BodyCase.MESSAGE, message: ChatClientEvent.Message }
  |{ case: BodyCase.CLOSE, close: ChatClientEvent.Close }
  >;

  class BodyImpl {
    open: ChatClientEvent.Open;
    message: ChatClientEvent.Message;
    close: ChatClientEvent.Close;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new ChatClientEvent.Open(v.open);
      } else
      if (v && "message" in v) {
        this.case = BodyCase.MESSAGE;
        this.message = new ChatClientEvent.Message(v.message);
      } else
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new ChatClientEvent.Close(v.close);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: ChatClientEvent.IOpen } ? { case: BodyCase.OPEN, open: ChatClientEvent.Open } :
    T extends { message: ChatClientEvent.IMessage } ? { case: BodyCase.MESSAGE, message: ChatClientEvent.Message } :
    T extends { close: ChatClientEvent.IClose } ? { case: BodyCase.CLOSE, close: ChatClientEvent.Close } :
    never
    >;
  };

  export interface IOpen {
    clientId?: bigint;
  }

  export class Open {
    clientId: bigint = BigInt(0);

    constructor(v?: IOpen) {
      this.clientId = v?.clientId || BigInt(0);
    }

    static encode(m: Open, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.clientId) w.uint32(8).uint64(m.clientId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Open {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Open();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.clientId = r.uint64();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IMessage {
    sentTime?: bigint;
    serverTime?: bigint;
    nick?: string;
    body?: string;
    entities?: IMessageEntities | undefined;
  }

  export class Message {
    sentTime: bigint = BigInt(0);
    serverTime: bigint = BigInt(0);
    nick: string = "";
    body: string = "";
    entities: MessageEntities | undefined;

    constructor(v?: IMessage) {
      this.sentTime = v?.sentTime || BigInt(0);
      this.serverTime = v?.serverTime || BigInt(0);
      this.nick = v?.nick || "";
      this.body = v?.body || "";
      this.entities = v?.entities && new MessageEntities(v.entities);
    }

    static encode(m: Message, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.sentTime) w.uint32(8).int64(m.sentTime);
      if (m.serverTime) w.uint32(16).int64(m.serverTime);
      if (m.nick) w.uint32(26).string(m.nick);
      if (m.body) w.uint32(34).string(m.body);
      if (m.entities) MessageEntities.encode(m.entities, w.uint32(42).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Message {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Message();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.sentTime = r.int64();
          break;
          case 2:
          m.serverTime = r.int64();
          break;
          case 3:
          m.nick = r.string();
          break;
          case 4:
          m.body = r.string();
          break;
          case 5:
          m.entities = MessageEntities.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IClose {
  }

  export class Close {

    constructor(v?: IClose) {
      // noop
    }

    static encode(m: Close, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Close {
      if (r instanceof Reader && length) r.skip(length);
      return new Close();
    }
  }

}

export interface IChatRoom {
  name?: string;
}

export class ChatRoom {
  name: string = "";

  constructor(v?: IChatRoom) {
    this.name = v?.name || "";
  }

  static encode(m: ChatRoom, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name) w.uint32(10).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChatRoom {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChatRoom();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IChatServer {
  id?: bigint;
  networkKey?: Uint8Array;
  key?: strims_type_IKey | undefined;
  chatRoom?: IChatRoom | undefined;
}

export class ChatServer {
  id: bigint = BigInt(0);
  networkKey: Uint8Array = new Uint8Array();
  key: strims_type_Key | undefined;
  chatRoom: ChatRoom | undefined;

  constructor(v?: IChatServer) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.key = v?.key && new strims_type_Key(v.key);
    this.chatRoom = v?.chatRoom && new ChatRoom(v.chatRoom);
  }

  static encode(m: ChatServer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey) w.uint32(18).bytes(m.networkKey);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(26).fork()).ldelim();
    if (m.chatRoom) ChatRoom.encode(m.chatRoom, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChatServer {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChatServer();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.networkKey = r.bytes();
        break;
        case 3:
        m.key = strims_type_Key.decode(r, r.uint32());
        break;
        case 4:
        m.chatRoom = ChatRoom.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IMessageEntities {
  links?: MessageEntities.ILink[];
  emotes?: MessageEntities.IEmote[];
  nicks?: MessageEntities.INick[];
  tags?: MessageEntities.ITag[];
  codeBlocks?: MessageEntities.ICodeBlock[];
  spoilers?: MessageEntities.ISpoiler[];
  greenText?: MessageEntities.IGenericEntity | undefined;
  selfMessage?: MessageEntities.IGenericEntity | undefined;
}

export class MessageEntities {
  links: MessageEntities.Link[] = [];
  emotes: MessageEntities.Emote[] = [];
  nicks: MessageEntities.Nick[] = [];
  tags: MessageEntities.Tag[] = [];
  codeBlocks: MessageEntities.CodeBlock[] = [];
  spoilers: MessageEntities.Spoiler[] = [];
  greenText: MessageEntities.GenericEntity | undefined;
  selfMessage: MessageEntities.GenericEntity | undefined;

  constructor(v?: IMessageEntities) {
    if (v?.links) this.links = v.links.map(v => new MessageEntities.Link(v));
    if (v?.emotes) this.emotes = v.emotes.map(v => new MessageEntities.Emote(v));
    if (v?.nicks) this.nicks = v.nicks.map(v => new MessageEntities.Nick(v));
    if (v?.tags) this.tags = v.tags.map(v => new MessageEntities.Tag(v));
    if (v?.codeBlocks) this.codeBlocks = v.codeBlocks.map(v => new MessageEntities.CodeBlock(v));
    if (v?.spoilers) this.spoilers = v.spoilers.map(v => new MessageEntities.Spoiler(v));
    this.greenText = v?.greenText && new MessageEntities.GenericEntity(v.greenText);
    this.selfMessage = v?.selfMessage && new MessageEntities.GenericEntity(v.selfMessage);
  }

  static encode(m: MessageEntities, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.links) MessageEntities.Link.encode(v, w.uint32(10).fork()).ldelim();
    for (const v of m.emotes) MessageEntities.Emote.encode(v, w.uint32(18).fork()).ldelim();
    for (const v of m.nicks) MessageEntities.Nick.encode(v, w.uint32(26).fork()).ldelim();
    for (const v of m.tags) MessageEntities.Tag.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.codeBlocks) MessageEntities.CodeBlock.encode(v, w.uint32(42).fork()).ldelim();
    for (const v of m.spoilers) MessageEntities.Spoiler.encode(v, w.uint32(50).fork()).ldelim();
    if (m.greenText) MessageEntities.GenericEntity.encode(m.greenText, w.uint32(58).fork()).ldelim();
    if (m.selfMessage) MessageEntities.GenericEntity.encode(m.selfMessage, w.uint32(66).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MessageEntities {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MessageEntities();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.links.push(MessageEntities.Link.decode(r, r.uint32()));
        break;
        case 2:
        m.emotes.push(MessageEntities.Emote.decode(r, r.uint32()));
        break;
        case 3:
        m.nicks.push(MessageEntities.Nick.decode(r, r.uint32()));
        break;
        case 4:
        m.tags.push(MessageEntities.Tag.decode(r, r.uint32()));
        break;
        case 5:
        m.codeBlocks.push(MessageEntities.CodeBlock.decode(r, r.uint32()));
        break;
        case 6:
        m.spoilers.push(MessageEntities.Spoiler.decode(r, r.uint32()));
        break;
        case 7:
        m.greenText = MessageEntities.GenericEntity.decode(r, r.uint32());
        break;
        case 8:
        m.selfMessage = MessageEntities.GenericEntity.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace MessageEntities {
  export interface IBounds {
    start?: number;
    end?: number;
  }

  export class Bounds {
    start: number = 0;
    end: number = 0;

    constructor(v?: IBounds) {
      this.start = v?.start || 0;
      this.end = v?.end || 0;
    }

    static encode(m: Bounds, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.start) w.uint32(8).uint32(m.start);
      if (m.end) w.uint32(16).uint32(m.end);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Bounds {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Bounds();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.start = r.uint32();
          break;
          case 2:
          m.end = r.uint32();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ILink {
    bounds?: MessageEntities.IBounds | undefined;
    url?: string;
  }

  export class Link {
    bounds: MessageEntities.Bounds | undefined;
    url: string = "";

    constructor(v?: ILink) {
      this.bounds = v?.bounds && new MessageEntities.Bounds(v.bounds);
      this.url = v?.url || "";
    }

    static encode(m: Link, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.bounds) MessageEntities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
      if (m.url) w.uint32(18).string(m.url);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Link {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Link();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.bounds = MessageEntities.Bounds.decode(r, r.uint32());
          break;
          case 2:
          m.url = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IEmote {
    bounds?: MessageEntities.IBounds | undefined;
    name?: string;
    modifiers?: string[];
    combo?: number;
  }

  export class Emote {
    bounds: MessageEntities.Bounds | undefined;
    name: string = "";
    modifiers: string[] = [];
    combo: number = 0;

    constructor(v?: IEmote) {
      this.bounds = v?.bounds && new MessageEntities.Bounds(v.bounds);
      this.name = v?.name || "";
      if (v?.modifiers) this.modifiers = v.modifiers;
      this.combo = v?.combo || 0;
    }

    static encode(m: Emote, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.bounds) MessageEntities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
      if (m.name) w.uint32(18).string(m.name);
      for (const v of m.modifiers) w.uint32(26).string(v);
      if (m.combo) w.uint32(32).uint32(m.combo);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Emote {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Emote();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.bounds = MessageEntities.Bounds.decode(r, r.uint32());
          break;
          case 2:
          m.name = r.string();
          break;
          case 3:
          m.modifiers.push(r.string())
          break;
          case 4:
          m.combo = r.uint32();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface INick {
    bounds?: MessageEntities.IBounds | undefined;
    nick?: string;
  }

  export class Nick {
    bounds: MessageEntities.Bounds | undefined;
    nick: string = "";

    constructor(v?: INick) {
      this.bounds = v?.bounds && new MessageEntities.Bounds(v.bounds);
      this.nick = v?.nick || "";
    }

    static encode(m: Nick, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.bounds) MessageEntities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
      if (m.nick) w.uint32(18).string(m.nick);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Nick {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Nick();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.bounds = MessageEntities.Bounds.decode(r, r.uint32());
          break;
          case 2:
          m.nick = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ITag {
    bounds?: MessageEntities.IBounds | undefined;
    name?: string;
  }

  export class Tag {
    bounds: MessageEntities.Bounds | undefined;
    name: string = "";

    constructor(v?: ITag) {
      this.bounds = v?.bounds && new MessageEntities.Bounds(v.bounds);
      this.name = v?.name || "";
    }

    static encode(m: Tag, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.bounds) MessageEntities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
      if (m.name) w.uint32(18).string(m.name);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Tag {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Tag();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.bounds = MessageEntities.Bounds.decode(r, r.uint32());
          break;
          case 2:
          m.name = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ICodeBlock {
    bounds?: MessageEntities.IBounds | undefined;
  }

  export class CodeBlock {
    bounds: MessageEntities.Bounds | undefined;

    constructor(v?: ICodeBlock) {
      this.bounds = v?.bounds && new MessageEntities.Bounds(v.bounds);
    }

    static encode(m: CodeBlock, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.bounds) MessageEntities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): CodeBlock {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new CodeBlock();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.bounds = MessageEntities.Bounds.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ISpoiler {
    bounds?: MessageEntities.IBounds | undefined;
  }

  export class Spoiler {
    bounds: MessageEntities.Bounds | undefined;

    constructor(v?: ISpoiler) {
      this.bounds = v?.bounds && new MessageEntities.Bounds(v.bounds);
    }

    static encode(m: Spoiler, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.bounds) MessageEntities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Spoiler {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Spoiler();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.bounds = MessageEntities.Bounds.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IGenericEntity {
    bounds?: MessageEntities.IBounds | undefined;
  }

  export class GenericEntity {
    bounds: MessageEntities.Bounds | undefined;

    constructor(v?: IGenericEntity) {
      this.bounds = v?.bounds && new MessageEntities.Bounds(v.bounds);
    }

    static encode(m: GenericEntity, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.bounds) MessageEntities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): GenericEntity {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new GenericEntity();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.bounds = MessageEntities.Bounds.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}

export interface ICallChatClientRequest {
  clientId?: bigint;
  body?: CallChatClientRequest.IBody
}

export class CallChatClientRequest {
  clientId: bigint = BigInt(0);
  body: CallChatClientRequest.TBody;

  constructor(v?: ICallChatClientRequest) {
    this.clientId = v?.clientId || BigInt(0);
    this.body = new CallChatClientRequest.Body(v?.body);
  }

  static encode(m: CallChatClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.clientId) w.uint32(8).uint64(m.clientId);
    switch (m.body.case) {
      case CallChatClientRequest.BodyCase.MESSAGE:
      CallChatClientRequest.Message.encode(m.body.message, w.uint32(18).fork()).ldelim();
      break;
      case CallChatClientRequest.BodyCase.CLOSE:
      CallChatClientRequest.Close.encode(m.body.close, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CallChatClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CallChatClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.clientId = r.uint64();
        break;
        case 2:
        m.body = new CallChatClientRequest.Body({ message: CallChatClientRequest.Message.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new CallChatClientRequest.Body({ close: CallChatClientRequest.Close.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CallChatClientRequest {
  export enum BodyCase {
    NOT_SET = 0,
    MESSAGE = 2,
    CLOSE = 3,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.MESSAGE, message: CallChatClientRequest.IMessage }
  |{ case?: BodyCase.CLOSE, close: CallChatClientRequest.IClose }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.MESSAGE, message: CallChatClientRequest.Message }
  |{ case: BodyCase.CLOSE, close: CallChatClientRequest.Close }
  >;

  class BodyImpl {
    message: CallChatClientRequest.Message;
    close: CallChatClientRequest.Close;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "message" in v) {
        this.case = BodyCase.MESSAGE;
        this.message = new CallChatClientRequest.Message(v.message);
      } else
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new CallChatClientRequest.Close(v.close);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { message: CallChatClientRequest.IMessage } ? { case: BodyCase.MESSAGE, message: CallChatClientRequest.Message } :
    T extends { close: CallChatClientRequest.IClose } ? { case: BodyCase.CLOSE, close: CallChatClientRequest.Close } :
    never
    >;
  };

  export interface IMessage {
    time?: bigint;
    body?: string;
  }

  export class Message {
    time: bigint = BigInt(0);
    body: string = "";

    constructor(v?: IMessage) {
      this.time = v?.time || BigInt(0);
      this.body = v?.body || "";
    }

    static encode(m: Message, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.time) w.uint32(8).int64(m.time);
      if (m.body) w.uint32(18).string(m.body);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Message {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Message();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.time = r.int64();
          break;
          case 2:
          m.body = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IClose {
  }

  export class Close {

    constructor(v?: IClose) {
      // noop
    }

    static encode(m: Close, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Close {
      if (r instanceof Reader && length) r.skip(length);
      return new Close();
    }
  }

}

export interface ICallChatClientResponse {
}

export class CallChatClientResponse {

  constructor(v?: ICallChatClientResponse) {
    // noop
  }

  static encode(m: CallChatClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CallChatClientResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new CallChatClientResponse();
  }
}

