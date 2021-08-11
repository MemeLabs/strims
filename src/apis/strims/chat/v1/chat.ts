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

export type IOpenServerRequest = {
  server?: IServer | undefined;
}

export class OpenServerRequest {
  server: Server | undefined;

  constructor(v?: IOpenServerRequest) {
    this.server = v?.server && new Server(v.server);
  }

  static encode(m: OpenServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new OpenServerRequest();
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
      case ServerEvent.BodyCase.OPEN:
      ServerEvent.Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case ServerEvent.BodyCase.CLOSE:
      ServerEvent.Close.encode(m.body.close, w.uint32(18).fork()).ldelim();
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
        case 1:
        m.body = new ServerEvent.Body({ open: ServerEvent.Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new ServerEvent.Body({ close: ServerEvent.Close.decode(r, r.uint32()) });
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
    OPEN = 1,
    CLOSE = 2,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.OPEN, open: ServerEvent.IOpen }
  |{ case?: BodyCase.CLOSE, close: ServerEvent.IClose }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: ServerEvent.Open }
  |{ case: BodyCase.CLOSE, close: ServerEvent.Close }
  >;

  class BodyImpl {
    open: ServerEvent.Open;
    close: ServerEvent.Close;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new ServerEvent.Open(v.open);
      } else
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new ServerEvent.Close(v.close);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: ServerEvent.IOpen } ? { case: BodyCase.OPEN, open: ServerEvent.Open } :
    T extends { close: ServerEvent.IClose } ? { case: BodyCase.CLOSE, close: ServerEvent.Close } :
    never
    >;
  };

  export type IOpen = {
    serverId?: bigint;
  }

  export class Open {
    serverId: bigint;

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

  export type IClose = {
  }

  export class Close {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IClose) {
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

export type ICallServerRequest = {
  serverId?: bigint;
  body?: CallServerRequest.IBody
}

export class CallServerRequest {
  serverId: bigint;
  body: CallServerRequest.TBody;

  constructor(v?: ICallServerRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.body = new CallServerRequest.Body(v?.body);
  }

  static encode(m: CallServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    switch (m.body.case) {
      case CallServerRequest.BodyCase.CLOSE:
      CallServerRequest.Close.encode(m.body.close, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CallServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CallServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.body = new CallServerRequest.Body({ close: CallServerRequest.Close.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CallServerRequest {
  export enum BodyCase {
    NOT_SET = 0,
    CLOSE = 2,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.CLOSE, close: CallServerRequest.IClose }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.CLOSE, close: CallServerRequest.Close }
  >;

  class BodyImpl {
    close: CallServerRequest.Close;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new CallServerRequest.Close(v.close);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { close: CallServerRequest.IClose } ? { case: BodyCase.CLOSE, close: CallServerRequest.Close } :
    never
    >;
  };

  export type IClose = {
  }

  export class Close {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IClose) {
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

export type IClientEvent = {
  body?: ClientEvent.IBody
}

export class ClientEvent {
  body: ClientEvent.TBody;

  constructor(v?: IClientEvent) {
    this.body = new ClientEvent.Body(v?.body);
  }

  static encode(m: ClientEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case ClientEvent.BodyCase.OPEN:
      ClientEvent.Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case ClientEvent.BodyCase.MESSAGE:
      ClientEvent.Message.encode(m.body.message, w.uint32(18).fork()).ldelim();
      break;
      case ClientEvent.BodyCase.CLOSE:
      ClientEvent.Close.encode(m.body.close, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ClientEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new ClientEvent.Body({ open: ClientEvent.Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new ClientEvent.Body({ message: ClientEvent.Message.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new ClientEvent.Body({ close: ClientEvent.Close.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ClientEvent {
  export enum BodyCase {
    NOT_SET = 0,
    OPEN = 1,
    MESSAGE = 2,
    CLOSE = 3,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.OPEN, open: ClientEvent.IOpen }
  |{ case?: BodyCase.MESSAGE, message: ClientEvent.IMessage }
  |{ case?: BodyCase.CLOSE, close: ClientEvent.IClose }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: ClientEvent.Open }
  |{ case: BodyCase.MESSAGE, message: ClientEvent.Message }
  |{ case: BodyCase.CLOSE, close: ClientEvent.Close }
  >;

  class BodyImpl {
    open: ClientEvent.Open;
    message: ClientEvent.Message;
    close: ClientEvent.Close;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new ClientEvent.Open(v.open);
      } else
      if (v && "message" in v) {
        this.case = BodyCase.MESSAGE;
        this.message = new ClientEvent.Message(v.message);
      } else
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new ClientEvent.Close(v.close);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: ClientEvent.IOpen } ? { case: BodyCase.OPEN, open: ClientEvent.Open } :
    T extends { message: ClientEvent.IMessage } ? { case: BodyCase.MESSAGE, message: ClientEvent.Message } :
    T extends { close: ClientEvent.IClose } ? { case: BodyCase.CLOSE, close: ClientEvent.Close } :
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

  export type IMessage = {
    sentTime?: bigint;
    serverTime?: bigint;
    nick?: string;
    body?: string;
    entities?: IMessageEntities | undefined;
  }

  export class Message {
    sentTime: bigint;
    serverTime: bigint;
    nick: string;
    body: string;
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

  export type IClose = {
  }

  export class Close {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IClose) {
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
  duration?: number;
  iterationCount?: number;
}

export class EmoteAnimation {
  frameCount: number;
  duration: number;
  iterationCount: number;

  constructor(v?: IEmoteAnimation) {
    this.frameCount = v?.frameCount || 0;
    this.duration = v?.duration || 0;
    this.iterationCount = v?.iterationCount || 0;
  }

  static encode(m: EmoteAnimation, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.frameCount) w.uint32(80).uint32(m.frameCount);
    if (m.duration) w.uint32(88).uint32(m.duration);
    if (m.iterationCount) w.uint32(96).uint32(m.iterationCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteAnimation {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EmoteAnimation();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 10:
        m.frameCount = r.uint32();
        break;
        case 11:
        m.duration = r.uint32();
        break;
        case 12:
        m.iterationCount = r.uint32();
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

export type IMessageEntities = {
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
  links: MessageEntities.Link[];
  emotes: MessageEntities.Emote[];
  nicks: MessageEntities.Nick[];
  tags: MessageEntities.Tag[];
  codeBlocks: MessageEntities.CodeBlock[];
  spoilers: MessageEntities.Spoiler[];
  greenText: MessageEntities.GenericEntity | undefined;
  selfMessage: MessageEntities.GenericEntity | undefined;

  constructor(v?: IMessageEntities) {
    this.links = v?.links ? v.links.map(v => new MessageEntities.Link(v)) : [];
    this.emotes = v?.emotes ? v.emotes.map(v => new MessageEntities.Emote(v)) : [];
    this.nicks = v?.nicks ? v.nicks.map(v => new MessageEntities.Nick(v)) : [];
    this.tags = v?.tags ? v.tags.map(v => new MessageEntities.Tag(v)) : [];
    this.codeBlocks = v?.codeBlocks ? v.codeBlocks.map(v => new MessageEntities.CodeBlock(v)) : [];
    this.spoilers = v?.spoilers ? v.spoilers.map(v => new MessageEntities.Spoiler(v)) : [];
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
    bounds?: MessageEntities.IBounds | undefined;
    url?: string;
  }

  export class Link {
    bounds: MessageEntities.Bounds | undefined;
    url: string;

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

  export type IEmote = {
    bounds?: MessageEntities.IBounds | undefined;
    name?: string;
    modifiers?: string[];
    combo?: number;
  }

  export class Emote {
    bounds: MessageEntities.Bounds | undefined;
    name: string;
    modifiers: string[];
    combo: number;

    constructor(v?: IEmote) {
      this.bounds = v?.bounds && new MessageEntities.Bounds(v.bounds);
      this.name = v?.name || "";
      this.modifiers = v?.modifiers ? v.modifiers : [];
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

  export type INick = {
    bounds?: MessageEntities.IBounds | undefined;
    nick?: string;
  }

  export class Nick {
    bounds: MessageEntities.Bounds | undefined;
    nick: string;

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

  export type ITag = {
    bounds?: MessageEntities.IBounds | undefined;
    name?: string;
  }

  export class Tag {
    bounds: MessageEntities.Bounds | undefined;
    name: string;

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

  export type ICodeBlock = {
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

  export type ISpoiler = {
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

  export type IGenericEntity = {
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

export type ICallClientRequest = {
  clientId?: bigint;
  body?: CallClientRequest.IBody
}

export class CallClientRequest {
  clientId: bigint;
  body: CallClientRequest.TBody;

  constructor(v?: ICallClientRequest) {
    this.clientId = v?.clientId || BigInt(0);
    this.body = new CallClientRequest.Body(v?.body);
  }

  static encode(m: CallClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.clientId) w.uint32(8).uint64(m.clientId);
    switch (m.body.case) {
      case CallClientRequest.BodyCase.MESSAGE:
      CallClientRequest.Message.encode(m.body.message, w.uint32(18).fork()).ldelim();
      break;
      case CallClientRequest.BodyCase.CLOSE:
      CallClientRequest.Close.encode(m.body.close, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CallClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CallClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.clientId = r.uint64();
        break;
        case 2:
        m.body = new CallClientRequest.Body({ message: CallClientRequest.Message.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new CallClientRequest.Body({ close: CallClientRequest.Close.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CallClientRequest {
  export enum BodyCase {
    NOT_SET = 0,
    MESSAGE = 2,
    CLOSE = 3,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.MESSAGE, message: CallClientRequest.IMessage }
  |{ case?: BodyCase.CLOSE, close: CallClientRequest.IClose }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.MESSAGE, message: CallClientRequest.Message }
  |{ case: BodyCase.CLOSE, close: CallClientRequest.Close }
  >;

  class BodyImpl {
    message: CallClientRequest.Message;
    close: CallClientRequest.Close;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "message" in v) {
        this.case = BodyCase.MESSAGE;
        this.message = new CallClientRequest.Message(v.message);
      } else
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new CallClientRequest.Close(v.close);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { message: CallClientRequest.IMessage } ? { case: BodyCase.MESSAGE, message: CallClientRequest.Message } :
    T extends { close: CallClientRequest.IClose } ? { case: BodyCase.CLOSE, close: CallClientRequest.Close } :
    never
    >;
  };

  export type IMessage = {
    time?: bigint;
    body?: string;
  }

  export class Message {
    time: bigint;
    body: string;

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

  export type IClose = {
  }

  export class Close {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IClose) {
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
