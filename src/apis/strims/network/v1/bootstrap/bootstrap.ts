import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate,
} from "../../../type/certificate";
import {
  Network as strims_network_v1_Network,
  INetwork as strims_network_v1_INetwork,
} from "..//network";

export type IBootstrapClient = {
  id?: bigint;
  clientOptions?: BootstrapClient.IClientOptions
}

export class BootstrapClient {
  id: bigint;
  clientOptions: BootstrapClient.TClientOptions;

  constructor(v?: IBootstrapClient) {
    this.id = v?.id || BigInt(0);
    this.clientOptions = new BootstrapClient.ClientOptions(v?.clientOptions);
  }

  static encode(m: BootstrapClient, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    switch (m.clientOptions.case) {
      case BootstrapClient.ClientOptionsCase.WEBSOCKET_OPTIONS:
      BootstrapClientWebSocketOptions.encode(m.clientOptions.websocketOptions, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapClient {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BootstrapClient();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.clientOptions = new BootstrapClient.ClientOptions({ websocketOptions: BootstrapClientWebSocketOptions.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace BootstrapClient {
  export enum ClientOptionsCase {
    NOT_SET = 0,
    WEBSOCKET_OPTIONS = 2,
  }

  export type IClientOptions =
  { case?: ClientOptionsCase.NOT_SET }
  |{ case?: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: IBootstrapClientWebSocketOptions }
  ;

  export type TClientOptions = Readonly<
  { case: ClientOptionsCase.NOT_SET }
  |{ case: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions }
  >;

  class ClientOptionsImpl {
    websocketOptions: BootstrapClientWebSocketOptions;
    case: ClientOptionsCase = ClientOptionsCase.NOT_SET;

    constructor(v?: IClientOptions) {
      if (v && "websocketOptions" in v) {
        this.case = ClientOptionsCase.WEBSOCKET_OPTIONS;
        this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
      }
    }
  }

  export const ClientOptions = ClientOptionsImpl as {
    new (): Readonly<{ case: ClientOptionsCase.NOT_SET }>;
    new <T extends IClientOptions>(v: T): Readonly<
    T extends { websocketOptions: IBootstrapClientWebSocketOptions } ? { case: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions } :
    never
    >;
  };

}

export type IBootstrapClientWebSocketOptions = {
  url?: string;
  insecureSkipVerifyTls?: boolean;
}

export class BootstrapClientWebSocketOptions {
  url: string;
  insecureSkipVerifyTls: boolean;

  constructor(v?: IBootstrapClientWebSocketOptions) {
    this.url = v?.url || "";
    this.insecureSkipVerifyTls = v?.insecureSkipVerifyTls || false;
  }

  static encode(m: BootstrapClientWebSocketOptions, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.url) w.uint32(10).string(m.url);
    if (m.insecureSkipVerifyTls) w.uint32(16).bool(m.insecureSkipVerifyTls);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapClientWebSocketOptions {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BootstrapClientWebSocketOptions();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.url = r.string();
        break;
        case 2:
        m.insecureSkipVerifyTls = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateBootstrapClientRequest = {
  clientOptions?: CreateBootstrapClientRequest.IClientOptions
}

export class CreateBootstrapClientRequest {
  clientOptions: CreateBootstrapClientRequest.TClientOptions;

  constructor(v?: ICreateBootstrapClientRequest) {
    this.clientOptions = new CreateBootstrapClientRequest.ClientOptions(v?.clientOptions);
  }

  static encode(m: CreateBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.clientOptions.case) {
      case CreateBootstrapClientRequest.ClientOptionsCase.WEBSOCKET_OPTIONS:
      BootstrapClientWebSocketOptions.encode(m.clientOptions.websocketOptions, w.uint32(10).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateBootstrapClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateBootstrapClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.clientOptions = new CreateBootstrapClientRequest.ClientOptions({ websocketOptions: BootstrapClientWebSocketOptions.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CreateBootstrapClientRequest {
  export enum ClientOptionsCase {
    NOT_SET = 0,
    WEBSOCKET_OPTIONS = 1,
  }

  export type IClientOptions =
  { case?: ClientOptionsCase.NOT_SET }
  |{ case?: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: IBootstrapClientWebSocketOptions }
  ;

  export type TClientOptions = Readonly<
  { case: ClientOptionsCase.NOT_SET }
  |{ case: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions }
  >;

  class ClientOptionsImpl {
    websocketOptions: BootstrapClientWebSocketOptions;
    case: ClientOptionsCase = ClientOptionsCase.NOT_SET;

    constructor(v?: IClientOptions) {
      if (v && "websocketOptions" in v) {
        this.case = ClientOptionsCase.WEBSOCKET_OPTIONS;
        this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
      }
    }
  }

  export const ClientOptions = ClientOptionsImpl as {
    new (): Readonly<{ case: ClientOptionsCase.NOT_SET }>;
    new <T extends IClientOptions>(v: T): Readonly<
    T extends { websocketOptions: IBootstrapClientWebSocketOptions } ? { case: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions } :
    never
    >;
  };

}

export type ICreateBootstrapClientResponse = {
  bootstrapClient?: IBootstrapClient;
}

export class CreateBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: ICreateBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: CreateBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bootstrapClient) BootstrapClient.encode(m.bootstrapClient, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateBootstrapClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateBootstrapClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClient = BootstrapClient.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateBootstrapClientRequest = {
  id?: bigint;
  clientOptions?: UpdateBootstrapClientRequest.IClientOptions
}

export class UpdateBootstrapClientRequest {
  id: bigint;
  clientOptions: UpdateBootstrapClientRequest.TClientOptions;

  constructor(v?: IUpdateBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
    this.clientOptions = new UpdateBootstrapClientRequest.ClientOptions(v?.clientOptions);
  }

  static encode(m: UpdateBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    switch (m.clientOptions.case) {
      case UpdateBootstrapClientRequest.ClientOptionsCase.WEBSOCKET_OPTIONS:
      BootstrapClientWebSocketOptions.encode(m.clientOptions.websocketOptions, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateBootstrapClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateBootstrapClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.clientOptions = new UpdateBootstrapClientRequest.ClientOptions({ websocketOptions: BootstrapClientWebSocketOptions.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace UpdateBootstrapClientRequest {
  export enum ClientOptionsCase {
    NOT_SET = 0,
    WEBSOCKET_OPTIONS = 2,
  }

  export type IClientOptions =
  { case?: ClientOptionsCase.NOT_SET }
  |{ case?: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: IBootstrapClientWebSocketOptions }
  ;

  export type TClientOptions = Readonly<
  { case: ClientOptionsCase.NOT_SET }
  |{ case: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions }
  >;

  class ClientOptionsImpl {
    websocketOptions: BootstrapClientWebSocketOptions;
    case: ClientOptionsCase = ClientOptionsCase.NOT_SET;

    constructor(v?: IClientOptions) {
      if (v && "websocketOptions" in v) {
        this.case = ClientOptionsCase.WEBSOCKET_OPTIONS;
        this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
      }
    }
  }

  export const ClientOptions = ClientOptionsImpl as {
    new (): Readonly<{ case: ClientOptionsCase.NOT_SET }>;
    new <T extends IClientOptions>(v: T): Readonly<
    T extends { websocketOptions: IBootstrapClientWebSocketOptions } ? { case: ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions } :
    never
    >;
  };

}

export type IUpdateBootstrapClientResponse = {
  bootstrapClient?: IBootstrapClient;
}

export class UpdateBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: IUpdateBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: UpdateBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bootstrapClient) BootstrapClient.encode(m.bootstrapClient, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateBootstrapClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateBootstrapClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClient = BootstrapClient.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteBootstrapClientRequest = {
  id?: bigint;
}

export class DeleteBootstrapClientRequest {
  id: bigint;

  constructor(v?: IDeleteBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteBootstrapClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteBootstrapClientRequest();
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

export type IDeleteBootstrapClientResponse = {
}

export class DeleteBootstrapClientResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteBootstrapClientResponse) {
  }

  static encode(m: DeleteBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteBootstrapClientResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteBootstrapClientResponse();
  }
}

export type IGetBootstrapClientRequest = {
  id?: bigint;
}

export class GetBootstrapClientRequest {
  id: bigint;

  constructor(v?: IGetBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetBootstrapClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetBootstrapClientRequest();
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

export type IGetBootstrapClientResponse = {
  bootstrapClient?: IBootstrapClient;
}

export class GetBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: IGetBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: GetBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bootstrapClient) BootstrapClient.encode(m.bootstrapClient, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetBootstrapClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetBootstrapClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClient = BootstrapClient.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListBootstrapClientsRequest = {
}

export class ListBootstrapClientsRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IListBootstrapClientsRequest) {
  }

  static encode(m: ListBootstrapClientsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapClientsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListBootstrapClientsRequest();
  }
}

export type IListBootstrapClientsResponse = {
  bootstrapClients?: IBootstrapClient[];
}

export class ListBootstrapClientsResponse {
  bootstrapClients: BootstrapClient[];

  constructor(v?: IListBootstrapClientsResponse) {
    this.bootstrapClients = v?.bootstrapClients ? v.bootstrapClients.map(v => new BootstrapClient(v)) : [];
  }

  static encode(m: ListBootstrapClientsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.bootstrapClients) BootstrapClient.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapClientsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListBootstrapClientsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClients.push(BootstrapClient.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListBootstrapPeersRequest = {
}

export class ListBootstrapPeersRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IListBootstrapPeersRequest) {
  }

  static encode(m: ListBootstrapPeersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapPeersRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListBootstrapPeersRequest();
  }
}

export type IListBootstrapPeersResponse = {
  peers?: IBootstrapPeer[];
}

export class ListBootstrapPeersResponse {
  peers: BootstrapPeer[];

  constructor(v?: IListBootstrapPeersResponse) {
    this.peers = v?.peers ? v.peers.map(v => new BootstrapPeer(v)) : [];
  }

  static encode(m: ListBootstrapPeersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.peers) BootstrapPeer.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapPeersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListBootstrapPeersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peers.push(BootstrapPeer.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBootstrapPeer = {
  peerId?: bigint;
  label?: string;
}

export class BootstrapPeer {
  peerId: bigint;
  label: string;

  constructor(v?: IBootstrapPeer) {
    this.peerId = v?.peerId || BigInt(0);
    this.label = v?.label || "";
  }

  static encode(m: BootstrapPeer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerId) w.uint32(8).uint64(m.peerId);
    if (m.label) w.uint32(18).string(m.label);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeer {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BootstrapPeer();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerId = r.uint64();
        break;
        case 2:
        m.label = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBootstrapServiceMessage = {
  body?: BootstrapServiceMessage.IBody
}

export class BootstrapServiceMessage {
  body: BootstrapServiceMessage.TBody;

  constructor(v?: IBootstrapServiceMessage) {
    this.body = new BootstrapServiceMessage.Body(v?.body);
  }

  static encode(m: BootstrapServiceMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case BootstrapServiceMessage.BodyCase.BROKER_OFFER:
      BootstrapServiceMessage.BrokerOffer.encode(m.body.brokerOffer, w.uint32(10).fork()).ldelim();
      break;
      case BootstrapServiceMessage.BodyCase.PUBLISH_REQUEST:
      BootstrapServiceMessage.PublishRequest.encode(m.body.publishRequest, w.uint32(18).fork()).ldelim();
      break;
      case BootstrapServiceMessage.BodyCase.PUBLISH_RESPONSE:
      BootstrapServiceMessage.PublishResponse.encode(m.body.publishResponse, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapServiceMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new BootstrapServiceMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new BootstrapServiceMessage.Body({ brokerOffer: BootstrapServiceMessage.BrokerOffer.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new BootstrapServiceMessage.Body({ publishRequest: BootstrapServiceMessage.PublishRequest.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new BootstrapServiceMessage.Body({ publishResponse: BootstrapServiceMessage.PublishResponse.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace BootstrapServiceMessage {
  export enum BodyCase {
    NOT_SET = 0,
    BROKER_OFFER = 1,
    PUBLISH_REQUEST = 2,
    PUBLISH_RESPONSE = 3,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.BROKER_OFFER, brokerOffer: BootstrapServiceMessage.IBrokerOffer }
  |{ case?: BodyCase.PUBLISH_REQUEST, publishRequest: BootstrapServiceMessage.IPublishRequest }
  |{ case?: BodyCase.PUBLISH_RESPONSE, publishResponse: BootstrapServiceMessage.IPublishResponse }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.BROKER_OFFER, brokerOffer: BootstrapServiceMessage.BrokerOffer }
  |{ case: BodyCase.PUBLISH_REQUEST, publishRequest: BootstrapServiceMessage.PublishRequest }
  |{ case: BodyCase.PUBLISH_RESPONSE, publishResponse: BootstrapServiceMessage.PublishResponse }
  >;

  class BodyImpl {
    brokerOffer: BootstrapServiceMessage.BrokerOffer;
    publishRequest: BootstrapServiceMessage.PublishRequest;
    publishResponse: BootstrapServiceMessage.PublishResponse;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "brokerOffer" in v) {
        this.case = BodyCase.BROKER_OFFER;
        this.brokerOffer = new BootstrapServiceMessage.BrokerOffer(v.brokerOffer);
      } else
      if (v && "publishRequest" in v) {
        this.case = BodyCase.PUBLISH_REQUEST;
        this.publishRequest = new BootstrapServiceMessage.PublishRequest(v.publishRequest);
      } else
      if (v && "publishResponse" in v) {
        this.case = BodyCase.PUBLISH_RESPONSE;
        this.publishResponse = new BootstrapServiceMessage.PublishResponse(v.publishResponse);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { brokerOffer: BootstrapServiceMessage.IBrokerOffer } ? { case: BodyCase.BROKER_OFFER, brokerOffer: BootstrapServiceMessage.BrokerOffer } :
    T extends { publishRequest: BootstrapServiceMessage.IPublishRequest } ? { case: BodyCase.PUBLISH_REQUEST, publishRequest: BootstrapServiceMessage.PublishRequest } :
    T extends { publishResponse: BootstrapServiceMessage.IPublishResponse } ? { case: BodyCase.PUBLISH_RESPONSE, publishResponse: BootstrapServiceMessage.PublishResponse } :
    never
    >;
  };

  export type IBrokerOffer = {
  }

  export class BrokerOffer {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IBrokerOffer) {
    }

    static encode(m: BrokerOffer, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): BrokerOffer {
      if (r instanceof Reader && length) r.skip(length);
      return new BrokerOffer();
    }
  }

  export type IPublishRequest = {
    name?: string;
    certificate?: strims_type_ICertificate;
  }

  export class PublishRequest {
    name: string;
    certificate: strims_type_Certificate | undefined;

    constructor(v?: IPublishRequest) {
      this.name = v?.name || "";
      this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    }

    static encode(m: PublishRequest, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.name) w.uint32(10).string(m.name);
      if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): PublishRequest {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new PublishRequest();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.name = r.string();
          break;
          case 2:
          m.certificate = strims_type_Certificate.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IPublishResponse = {
    body?: PublishResponse.IBody
  }

  export class PublishResponse {
    body: PublishResponse.TBody;

    constructor(v?: IPublishResponse) {
      this.body = new PublishResponse.Body(v?.body);
    }

    static encode(m: PublishResponse, w?: Writer): Writer {
      if (!w) w = new Writer();
      switch (m.body.case) {
        case PublishResponse.BodyCase.ERROR:
        w.uint32(10).string(m.body.error);
        break;
      }
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): PublishResponse {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new PublishResponse();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.body = new PublishResponse.Body({ error: r.string() });
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace PublishResponse {
    export enum BodyCase {
      NOT_SET = 0,
      ERROR = 1,
    }

    export type IBody =
    { case?: BodyCase.NOT_SET }
    |{ case?: BodyCase.ERROR, error: string }
    ;

    export type TBody = Readonly<
    { case: BodyCase.NOT_SET }
    |{ case: BodyCase.ERROR, error: string }
    >;

    class BodyImpl {
      error: string;
      case: BodyCase = BodyCase.NOT_SET;

      constructor(v?: IBody) {
        if (v && "error" in v) {
          this.case = BodyCase.ERROR;
          this.error = v.error;
        }
      }
    }

    export const Body = BodyImpl as {
      new (): Readonly<{ case: BodyCase.NOT_SET }>;
      new <T extends IBody>(v: T): Readonly<
      T extends { error: string } ? { case: BodyCase.ERROR, error: string } :
      never
      >;
    };

  }

}

export type IPublishNetworkToBootstrapPeerRequest = {
  peerId?: bigint;
  network?: strims_network_v1_INetwork;
}

export class PublishNetworkToBootstrapPeerRequest {
  peerId: bigint;
  network: strims_network_v1_Network | undefined;

  constructor(v?: IPublishNetworkToBootstrapPeerRequest) {
    this.peerId = v?.peerId || BigInt(0);
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: PublishNetworkToBootstrapPeerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerId) w.uint32(8).uint64(m.peerId);
    if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PublishNetworkToBootstrapPeerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PublishNetworkToBootstrapPeerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerId = r.uint64();
        break;
        case 2:
        m.network = strims_network_v1_Network.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPublishNetworkToBootstrapPeerResponse = {
}

export class PublishNetworkToBootstrapPeerResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IPublishNetworkToBootstrapPeerResponse) {
  }

  static encode(m: PublishNetworkToBootstrapPeerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PublishNetworkToBootstrapPeerResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new PublishNetworkToBootstrapPeerResponse();
  }
}
