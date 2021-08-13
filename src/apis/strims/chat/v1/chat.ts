import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Key as strims_type_Key,
  IKey as strims_type_IKey,
} from "../../type/key";

export type ICreateServerRequest = {
  networkKey?: Uint8Array;
  room?: IRoom | undefined;
}

export class CreateServerRequest {
  networkKey: Uint8Array;
  room: Room | undefined;

  constructor(v?: ICreateServerRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.room = v?.room && new Room(v.room);
  }

  static encode(m: CreateServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey) w.uint32(18).bytes(m.networkKey);
    if (m.room) Room.encode(m.room, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 2:
        m.networkKey = r.bytes();
        break;
        case 3:
        m.room = Room.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateServerResponse = {
  server?: IServer | undefined;
}

export class CreateServerResponse {
  server: Server | undefined;

  constructor(v?: ICreateServerResponse) {
    this.server = v?.server && new Server(v.server);
  }

  static encode(m: CreateServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateServerRequest = {
  id?: bigint;
  networkKey?: Uint8Array;
  room?: IRoom | undefined;
}

export class UpdateServerRequest {
  id: bigint;
  networkKey: Uint8Array;
  room: Room | undefined;

  constructor(v?: IUpdateServerRequest) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.room = v?.room && new Room(v.room);
  }

  static encode(m: UpdateServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey) w.uint32(18).bytes(m.networkKey);
    if (m.room) Room.encode(m.room, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateServerRequest();
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
        m.room = Room.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateServerResponse = {
  server?: IServer | undefined;
}

export class UpdateServerResponse {
  server: Server | undefined;

  constructor(v?: IUpdateServerResponse) {
    this.server = v?.server && new Server(v.server);
  }

  static encode(m: UpdateServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteServerRequest = {
  id?: bigint;
}

export class DeleteServerRequest {
  id: bigint;

  constructor(v?: IDeleteServerRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteServerRequest();
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

export type IDeleteServerResponse = {
}

export class DeleteServerResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteServerResponse) {
  }

  static encode(m: DeleteServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteServerResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteServerResponse();
  }
}

export type IGetServerRequest = {
  id?: bigint;
}

export class GetServerRequest {
  id: bigint;

  constructor(v?: IGetServerRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetServerRequest();
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

export type IGetServerResponse = {
  server?: IServer | undefined;
}

export class GetServerResponse {
  server: Server | undefined;

  constructor(v?: IGetServerResponse) {
    this.server = v?.server && new Server(v.server);
  }

  static encode(m: GetServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListServersRequest = {
}

export class ListServersRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IListServersRequest) {
  }

  static encode(m: ListServersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListServersRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListServersRequest();
  }
}

export type IListServersResponse = {
  servers?: IServer[];
}

export class ListServersResponse {
  servers: Server[];

  constructor(v?: IListServersResponse) {
    this.servers = v?.servers ? v.servers.map(v => new Server(v)) : [];
  }

  static encode(m: ListServersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.servers) Server.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListServersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListServersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.servers.push(Server.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateEmoteRequest = {
  serverId?: bigint;
  name?: string;
  images?: IEmoteImage[];
  css?: string;
  animation?: IEmoteAnimation | undefined;
}

export class CreateEmoteRequest {
  serverId: bigint;
  name: string;
  images: EmoteImage[];
  css: string;
  animation: EmoteAnimation | undefined;

  constructor(v?: ICreateEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.animation = v?.animation && new EmoteAnimation(v.animation);
  }

  static encode(m: CreateEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.name) w.uint32(18).string(m.name);
    for (const v of m.images) EmoteImage.encode(v, w.uint32(26).fork()).ldelim();
    if (m.css) w.uint32(34).string(m.css);
    if (m.animation) EmoteAnimation.encode(m.animation, w.uint32(42).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateEmoteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.name = r.string();
        break;
        case 3:
        m.images.push(EmoteImage.decode(r, r.uint32()));
        break;
        case 4:
        m.css = r.string();
        break;
        case 5:
        m.animation = EmoteAnimation.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateEmoteResponse = {
  emote?: IEmote | undefined;
}

export class CreateEmoteResponse {
  emote: Emote | undefined;

  constructor(v?: ICreateEmoteResponse) {
    this.emote = v?.emote && new Emote(v.emote);
  }

  static encode(m: CreateEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateEmoteResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateEmoteRequest = {
  serverId?: bigint;
  id?: bigint;
  name?: string;
  images?: IEmoteImage[];
  css?: string;
  animation?: IEmoteAnimation | undefined;
}

export class UpdateEmoteRequest {
  serverId: bigint;
  id: bigint;
  name: string;
  images: EmoteImage[];
  css: string;
  animation: EmoteAnimation | undefined;

  constructor(v?: IUpdateEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.animation = v?.animation && new EmoteAnimation(v.animation);
  }

  static encode(m: UpdateEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.name) w.uint32(26).string(m.name);
    for (const v of m.images) EmoteImage.encode(v, w.uint32(34).fork()).ldelim();
    if (m.css) w.uint32(42).string(m.css);
    if (m.animation) EmoteAnimation.encode(m.animation, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateEmoteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.id = r.uint64();
        break;
        case 3:
        m.name = r.string();
        break;
        case 4:
        m.images.push(EmoteImage.decode(r, r.uint32()));
        break;
        case 5:
        m.css = r.string();
        break;
        case 6:
        m.animation = EmoteAnimation.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateEmoteResponse = {
  emote?: IEmote | undefined;
}

export class UpdateEmoteResponse {
  emote: Emote | undefined;

  constructor(v?: IUpdateEmoteResponse) {
    this.emote = v?.emote && new Emote(v.emote);
  }

  static encode(m: UpdateEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateEmoteResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteEmoteRequest = {
  serverId?: bigint;
  id?: bigint;
}

export class DeleteEmoteRequest {
  serverId: bigint;
  id: bigint;

  constructor(v?: IDeleteEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteEmoteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
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

export type IDeleteEmoteResponse = {
}

export class DeleteEmoteResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteEmoteResponse) {
  }

  static encode(m: DeleteEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteEmoteResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteEmoteResponse();
  }
}

export type IGetEmoteRequest = {
  id?: bigint;
}

export class GetEmoteRequest {
  id: bigint;

  constructor(v?: IGetEmoteRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetEmoteRequest();
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

export type IGetEmoteResponse = {
  emote?: IEmote | undefined;
}

export class GetEmoteResponse {
  emote: Emote | undefined;

  constructor(v?: IGetEmoteResponse) {
    this.emote = v?.emote && new Emote(v.emote);
  }

  static encode(m: GetEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetEmoteResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListEmotesRequest = {
  serverId?: bigint;
}

export class ListEmotesRequest {
  serverId: bigint;

  constructor(v?: IListEmotesRequest) {
    this.serverId = v?.serverId || BigInt(0);
  }

  static encode(m: ListEmotesRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListEmotesRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListEmotesRequest();
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

export type IListEmotesResponse = {
  emotes?: IEmote[];
}

export class ListEmotesResponse {
  emotes: Emote[];

  constructor(v?: IListEmotesResponse) {
    this.emotes = v?.emotes ? v.emotes.map(v => new Emote(v)) : [];
  }

  static encode(m: ListEmotesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.emotes) Emote.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListEmotesResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListEmotesResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emotes.push(Emote.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IOpenClientRequest = {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
}

export class OpenClientRequest {
  networkKey: Uint8Array;
  serverKey: Uint8Array;

  constructor(v?: IOpenClientRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
  }

  static encode(m: OpenClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
    if (m.serverKey) w.uint32(18).bytes(m.serverKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new OpenClientRequest();
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

export type IOpenClientResponse = {
  body?: OpenClientResponse.IBody
}

export class OpenClientResponse {
  body: OpenClientResponse.TBody;

  constructor(v?: IOpenClientResponse) {
    this.body = new OpenClientResponse.Body(v?.body);
  }

  static encode(m: OpenClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case OpenClientResponse.BodyCase.OPEN:
      OpenClientResponse.Open.encode(m.body.open, w.uint32(8010).fork()).ldelim();
      break;
      case OpenClientResponse.BodyCase.MESSAGE:
      Message.encode(m.body.message, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new OpenClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new OpenClientResponse.Body({ open: OpenClientResponse.Open.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new OpenClientResponse.Body({ message: Message.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace OpenClientResponse {
  export enum BodyCase {
    NOT_SET = 0,
    OPEN = 1001,
    MESSAGE = 1002,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.OPEN, open: OpenClientResponse.IOpen }
  |{ case?: BodyCase.MESSAGE, message: IMessage }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: OpenClientResponse.Open }
  |{ case: BodyCase.MESSAGE, message: Message }
  >;

  class BodyImpl {
    open: OpenClientResponse.Open;
    message: Message;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new OpenClientResponse.Open(v.open);
      } else
      if (v && "message" in v) {
        this.case = BodyCase.MESSAGE;
        this.message = new Message(v.message);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: OpenClientResponse.IOpen } ? { case: BodyCase.OPEN, open: OpenClientResponse.Open } :
    T extends { message: IMessage } ? { case: BodyCase.MESSAGE, message: Message } :
    never
    >;
  };

  export type IOpen = {
    clientId?: bigint;
  }

  export class Open {
    clientId: bigint;

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

}

export type IClientSendMessageRequest = {
  clientId?: bigint;
  body?: string;
}

export class ClientSendMessageRequest {
  clientId: bigint;
  body: string;

  constructor(v?: IClientSendMessageRequest) {
    this.clientId = v?.clientId || BigInt(0);
    this.body = v?.body || "";
  }

  static encode(m: ClientSendMessageRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.clientId) w.uint32(8).uint64(m.clientId);
    if (m.body) w.uint32(18).string(m.body);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientSendMessageRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ClientSendMessageRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.clientId = r.uint64();
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

export type IClientSendMessageResponse = {
}

export class ClientSendMessageResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IClientSendMessageResponse) {
  }

  static encode(m: ClientSendMessageResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientSendMessageResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new ClientSendMessageResponse();
  }
}

export type ICallClientResponse = {
}

export class CallClientResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ICallClientResponse) {
  }

  static encode(m: CallClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CallClientResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new CallClientResponse();
  }
}

export type IServerEvent = {
  body?: ServerEvent.IBody
}

export class ServerEvent {
  body: ServerEvent.TBody;

  constructor(v?: IServerEvent) {
    this.body = new ServerEvent.Body(v?.body);
  }

  static encode(m: ServerEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case ServerEvent.BodyCase.MESSAGE:
      Message.encode(m.body.message, w.uint32(8010).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ServerEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new ServerEvent.Body({ message: Message.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ServerEvent {
  export enum BodyCase {
    NOT_SET = 0,
    MESSAGE = 1001,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.MESSAGE, message: IMessage }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.MESSAGE, message: Message }
  >;

  class BodyImpl {
    message: Message;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "message" in v) {
        this.case = BodyCase.MESSAGE;
        this.message = new Message(v.message);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { message: IMessage } ? { case: BodyCase.MESSAGE, message: Message } :
    never
    >;
  };

}

export type IRoom = {
  name?: string;
}

export class Room {
  name: string;

  constructor(v?: IRoom) {
    this.name = v?.name || "";
  }

  static encode(m: Room, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name) w.uint32(10).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Room {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Room();
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

export type IServer = {
  id?: bigint;
  networkKey?: Uint8Array;
  key?: strims_type_IKey | undefined;
  room?: IRoom | undefined;
}

export class Server {
  id: bigint;
  networkKey: Uint8Array;
  key: strims_type_Key | undefined;
  room: Room | undefined;

  constructor(v?: IServer) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.key = v?.key && new strims_type_Key(v.key);
    this.room = v?.room && new Room(v.room);
  }

  static encode(m: Server, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey) w.uint32(18).bytes(m.networkKey);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(26).fork()).ldelim();
    if (m.room) Room.encode(m.room, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Server {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Server();
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
        m.room = Room.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEmoteImage = {
  data?: Uint8Array;
  fileType?: EmoteFileType;
  height?: number;
  width?: number;
  scale?: EmoteScale;
}

export class EmoteImage {
  data: Uint8Array;
  fileType: EmoteFileType;
  height: number;
  width: number;
  scale: EmoteScale;

  constructor(v?: IEmoteImage) {
    this.data = v?.data || new Uint8Array();
    this.fileType = v?.fileType || 0;
    this.height = v?.height || 0;
    this.width = v?.width || 0;
    this.scale = v?.scale || 0;
  }

  static encode(m: EmoteImage, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.data) w.uint32(26).bytes(m.data);
    if (m.fileType) w.uint32(32).uint32(m.fileType);
    if (m.height) w.uint32(40).uint32(m.height);
    if (m.width) w.uint32(48).uint32(m.width);
    if (m.scale) w.uint32(56).uint32(m.scale);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteImage {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EmoteImage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 3:
        m.data = r.bytes();
        break;
        case 4:
        m.fileType = r.uint32();
        break;
        case 5:
        m.height = r.uint32();
        break;
        case 6:
        m.width = r.uint32();
        break;
        case 7:
        m.scale = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEmoteAnimation = {
  frameCount?: number;
  durationMs?: number;
  iterationCount?: number;
  endOnLastFrame?: boolean;
  loopForever?: boolean;
}

export class EmoteAnimation {
  frameCount: number;
  durationMs: number;
  iterationCount: number;
  endOnLastFrame: boolean;
  loopForever: boolean;

  constructor(v?: IEmoteAnimation) {
    this.frameCount = v?.frameCount || 0;
    this.durationMs = v?.durationMs || 0;
    this.iterationCount = v?.iterationCount || 0;
    this.endOnLastFrame = v?.endOnLastFrame || false;
    this.loopForever = v?.loopForever || false;
  }

  static encode(m: EmoteAnimation, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.frameCount) w.uint32(8).uint32(m.frameCount);
    if (m.durationMs) w.uint32(16).uint32(m.durationMs);
    if (m.iterationCount) w.uint32(24).uint32(m.iterationCount);
    if (m.endOnLastFrame) w.uint32(32).bool(m.endOnLastFrame);
    if (m.loopForever) w.uint32(40).bool(m.loopForever);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteAnimation {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EmoteAnimation();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.frameCount = r.uint32();
        break;
        case 2:
        m.durationMs = r.uint32();
        break;
        case 3:
        m.iterationCount = r.uint32();
        break;
        case 4:
        m.endOnLastFrame = r.bool();
        break;
        case 5:
        m.loopForever = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEmote = {
  id?: bigint;
  name?: string;
  images?: IEmoteImage[];
  css?: string;
  animation?: IEmoteAnimation | undefined;
}

export class Emote {
  id: bigint;
  name: string;
  images: EmoteImage[];
  css: string;
  animation: EmoteAnimation | undefined;

  constructor(v?: IEmote) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.animation = v?.animation && new EmoteAnimation(v.animation);
  }

  static encode(m: Emote, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name) w.uint32(18).string(m.name);
    for (const v of m.images) EmoteImage.encode(v, w.uint32(26).fork()).ldelim();
    if (m.css) w.uint32(34).string(m.css);
    if (m.animation) EmoteAnimation.encode(m.animation, w.uint32(42).fork()).ldelim();
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
        m.id = r.uint64();
        break;
        case 2:
        m.name = r.string();
        break;
        case 3:
        m.images.push(EmoteImage.decode(r, r.uint32()));
        break;
        case 4:
        m.css = r.string();
        break;
        case 5:
        m.animation = EmoteAnimation.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IAssetBundle = {
  emotes?: IEmote[];
}

export class AssetBundle {
  emotes: Emote[];

  constructor(v?: IAssetBundle) {
    this.emotes = v?.emotes ? v.emotes.map(v => new Emote(v)) : [];
  }

  static encode(m: AssetBundle, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.emotes) Emote.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): AssetBundle {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new AssetBundle();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emotes.push(Emote.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMessage = {
  serverTime?: bigint;
  hostId?: Uint8Array;
  nick?: string;
  body?: string;
  entities?: Message.IEntities | undefined;
}

export class Message {
  serverTime: bigint;
  hostId: Uint8Array;
  nick: string;
  body: string;
  entities: Message.Entities | undefined;

  constructor(v?: IMessage) {
    this.serverTime = v?.serverTime || BigInt(0);
    this.hostId = v?.hostId || new Uint8Array();
    this.nick = v?.nick || "";
    this.body = v?.body || "";
    this.entities = v?.entities && new Message.Entities(v.entities);
  }

  static encode(m: Message, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverTime) w.uint32(8).int64(m.serverTime);
    if (m.hostId) w.uint32(18).bytes(m.hostId);
    if (m.nick) w.uint32(26).string(m.nick);
    if (m.body) w.uint32(34).string(m.body);
    if (m.entities) Message.Entities.encode(m.entities, w.uint32(42).fork()).ldelim();
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
        m.serverTime = r.int64();
        break;
        case 2:
        m.hostId = r.bytes();
        break;
        case 3:
        m.nick = r.string();
        break;
        case 4:
        m.body = r.string();
        break;
        case 5:
        m.entities = Message.Entities.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Message {
  export type IEntities = {
    links?: Message.Entities.ILink[];
    emotes?: Message.Entities.IEmote[];
    nicks?: Message.Entities.INick[];
    tags?: Message.Entities.ITag[];
    codeBlocks?: Message.Entities.ICodeBlock[];
    spoilers?: Message.Entities.ISpoiler[];
    greenText?: Message.Entities.IGenericEntity | undefined;
    selfMessage?: Message.Entities.IGenericEntity | undefined;
  }

  export class Entities {
    links: Message.Entities.Link[];
    emotes: Message.Entities.Emote[];
    nicks: Message.Entities.Nick[];
    tags: Message.Entities.Tag[];
    codeBlocks: Message.Entities.CodeBlock[];
    spoilers: Message.Entities.Spoiler[];
    greenText: Message.Entities.GenericEntity | undefined;
    selfMessage: Message.Entities.GenericEntity | undefined;

    constructor(v?: IEntities) {
      this.links = v?.links ? v.links.map(v => new Message.Entities.Link(v)) : [];
      this.emotes = v?.emotes ? v.emotes.map(v => new Message.Entities.Emote(v)) : [];
      this.nicks = v?.nicks ? v.nicks.map(v => new Message.Entities.Nick(v)) : [];
      this.tags = v?.tags ? v.tags.map(v => new Message.Entities.Tag(v)) : [];
      this.codeBlocks = v?.codeBlocks ? v.codeBlocks.map(v => new Message.Entities.CodeBlock(v)) : [];
      this.spoilers = v?.spoilers ? v.spoilers.map(v => new Message.Entities.Spoiler(v)) : [];
      this.greenText = v?.greenText && new Message.Entities.GenericEntity(v.greenText);
      this.selfMessage = v?.selfMessage && new Message.Entities.GenericEntity(v.selfMessage);
    }

    static encode(m: Entities, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.links) Message.Entities.Link.encode(v, w.uint32(10).fork()).ldelim();
      for (const v of m.emotes) Message.Entities.Emote.encode(v, w.uint32(18).fork()).ldelim();
      for (const v of m.nicks) Message.Entities.Nick.encode(v, w.uint32(26).fork()).ldelim();
      for (const v of m.tags) Message.Entities.Tag.encode(v, w.uint32(34).fork()).ldelim();
      for (const v of m.codeBlocks) Message.Entities.CodeBlock.encode(v, w.uint32(42).fork()).ldelim();
      for (const v of m.spoilers) Message.Entities.Spoiler.encode(v, w.uint32(50).fork()).ldelim();
      if (m.greenText) Message.Entities.GenericEntity.encode(m.greenText, w.uint32(58).fork()).ldelim();
      if (m.selfMessage) Message.Entities.GenericEntity.encode(m.selfMessage, w.uint32(66).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Entities {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Entities();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.links.push(Message.Entities.Link.decode(r, r.uint32()));
          break;
          case 2:
          m.emotes.push(Message.Entities.Emote.decode(r, r.uint32()));
          break;
          case 3:
          m.nicks.push(Message.Entities.Nick.decode(r, r.uint32()));
          break;
          case 4:
          m.tags.push(Message.Entities.Tag.decode(r, r.uint32()));
          break;
          case 5:
          m.codeBlocks.push(Message.Entities.CodeBlock.decode(r, r.uint32()));
          break;
          case 6:
          m.spoilers.push(Message.Entities.Spoiler.decode(r, r.uint32()));
          break;
          case 7:
          m.greenText = Message.Entities.GenericEntity.decode(r, r.uint32());
          break;
          case 8:
          m.selfMessage = Message.Entities.GenericEntity.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace Entities {
    export type IBounds = {
      start?: number;
      end?: number;
    }

    export class Bounds {
      start: number;
      end: number;

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

    export type ILink = {
      bounds?: Message.Entities.IBounds | undefined;
      url?: string;
    }

    export class Link {
      bounds: Message.Entities.Bounds | undefined;
      url: string;

      constructor(v?: ILink) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
        this.url = v?.url || "";
      }

      static encode(m: Link, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
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

    export type IEmote = {
      bounds?: Message.Entities.IBounds | undefined;
      name?: string;
      modifiers?: string[];
      combo?: number;
    }

    export class Emote {
      bounds: Message.Entities.Bounds | undefined;
      name: string;
      modifiers: string[];
      combo: number;

      constructor(v?: IEmote) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
        this.name = v?.name || "";
        this.modifiers = v?.modifiers ? v.modifiers : [];
        this.combo = v?.combo || 0;
      }

      static encode(m: Emote, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
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

    export type INick = {
      bounds?: Message.Entities.IBounds | undefined;
      nick?: string;
    }

    export class Nick {
      bounds: Message.Entities.Bounds | undefined;
      nick: string;

      constructor(v?: INick) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
        this.nick = v?.nick || "";
      }

      static encode(m: Nick, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
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

    export type ITag = {
      bounds?: Message.Entities.IBounds | undefined;
      name?: string;
    }

    export class Tag {
      bounds: Message.Entities.Bounds | undefined;
      name: string;

      constructor(v?: ITag) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
        this.name = v?.name || "";
      }

      static encode(m: Tag, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
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

    export type ICodeBlock = {
      bounds?: Message.Entities.IBounds | undefined;
    }

    export class CodeBlock {
      bounds: Message.Entities.Bounds | undefined;

      constructor(v?: ICodeBlock) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
      }

      static encode(m: CodeBlock, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type ISpoiler = {
      bounds?: Message.Entities.IBounds | undefined;
    }

    export class Spoiler {
      bounds: Message.Entities.Bounds | undefined;

      constructor(v?: ISpoiler) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
      }

      static encode(m: Spoiler, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type IGenericEntity = {
      bounds?: Message.Entities.IBounds | undefined;
    }

    export class GenericEntity {
      bounds: Message.Entities.Bounds | undefined;

      constructor(v?: IGenericEntity) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
      }

      static encode(m: GenericEntity, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
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

}

export type ISendMessageRequest = {
  body?: string;
}

export class SendMessageRequest {
  body: string;

  constructor(v?: ISendMessageRequest) {
    this.body = v?.body || "";
  }

  static encode(m: SendMessageRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.body) w.uint32(10).string(m.body);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SendMessageRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SendMessageRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type ISendMessageResponse = {
}

export class SendMessageResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ISendMessageResponse) {
  }

  static encode(m: SendMessageResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SendMessageResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new SendMessageResponse();
  }
}

export enum EmoteFileType {
  FILE_TYPE_UNDEFINED = 0,
  FILE_TYPE_PNG = 1,
  FILE_TYPE_GIF = 2,
}
export enum EmoteScale {
  EMOTE_SCALE_1X = 0,
  EMOTE_SCALE_2X = 1,
  EMOTE_SCALE_4X = 2,
}
