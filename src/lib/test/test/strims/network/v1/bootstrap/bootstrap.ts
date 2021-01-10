import Reader from "../../../../../pb/reader";
import Writer from "../../../../../pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate
} from "../../../type/certificate";
import {
  Network as strims_network_v1_Network,
  INetwork as strims_network_v1_INetwork
} from "..//network";

export interface IBootstrapClient {
  id?: bigint;
  clientOptions?: BootstrapClient.IClientOptionsOneOf
}

export class BootstrapClient {
  id: bigint = BigInt(0);
  clientOptions: BootstrapClient.ClientOptionsOneOf;

  constructor(v?: IBootstrapClient) {
    this.id = v?.id || BigInt(0);
    this.clientOptions = new BootstrapClient.ClientOptionsOneOf(v?.clientOptions);
  }

  static encode(m: BootstrapClient, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.id) w.uint32(8).uint64(m.id);
    switch (m.clientOptions.case) {
      case 2:
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
        m.clientOptions.websocketOptions = BootstrapClientWebSocketOptions.decode(r, r.uint32());
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
  export type IClientOptionsOneOf =
  { websocketOptions: IBootstrapClientWebSocketOptions }
  ;

  export class ClientOptionsOneOf {
    private _websocketOptions: BootstrapClientWebSocketOptions | undefined;
    private _case: ClientOptionsCase = 0;

    constructor(v?: IClientOptionsOneOf) {
      if (v && "websocketOptions" in v) this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
    }

    public clear() {
      this._websocketOptions = undefined;
      this._case = ClientOptionsCase.NOT_SET;
    }

    get case(): ClientOptionsCase {
      return this._case;
    }

    set websocketOptions(v: BootstrapClientWebSocketOptions) {
      this.clear();
      this._websocketOptions = v;
      this._case = ClientOptionsCase.WEBSOCKET_OPTIONS;
    }

    get websocketOptions(): BootstrapClientWebSocketOptions {
      return this._websocketOptions;
    }
  }

  export enum ClientOptionsCase {
    NOT_SET = 0,
    WEBSOCKET_OPTIONS = 2,
  }

}

export interface IBootstrapClientWebSocketOptions {
  url?: string;
  insecureSkipVerifyTls?: boolean;
}

export class BootstrapClientWebSocketOptions {
  url: string = "";
  insecureSkipVerifyTls: boolean = false;

  constructor(v?: IBootstrapClientWebSocketOptions) {
    this.url = v?.url || "";
    this.insecureSkipVerifyTls = v?.insecureSkipVerifyTls || false;
  }

  static encode(m: BootstrapClientWebSocketOptions, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface ICreateBootstrapClientRequest {
  clientOptions?: CreateBootstrapClientRequest.IClientOptionsOneOf
}

export class CreateBootstrapClientRequest {
  clientOptions: CreateBootstrapClientRequest.ClientOptionsOneOf;

  constructor(v?: ICreateBootstrapClientRequest) {
    this.clientOptions = new CreateBootstrapClientRequest.ClientOptionsOneOf(v?.clientOptions);
  }

  static encode(m: CreateBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    switch (m.clientOptions.case) {
      case 1:
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
        m.clientOptions.websocketOptions = BootstrapClientWebSocketOptions.decode(r, r.uint32());
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
  export type IClientOptionsOneOf =
  { websocketOptions: IBootstrapClientWebSocketOptions }
  ;

  export class ClientOptionsOneOf {
    private _websocketOptions: BootstrapClientWebSocketOptions | undefined;
    private _case: ClientOptionsCase = 0;

    constructor(v?: IClientOptionsOneOf) {
      if (v && "websocketOptions" in v) this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
    }

    public clear() {
      this._websocketOptions = undefined;
      this._case = ClientOptionsCase.NOT_SET;
    }

    get case(): ClientOptionsCase {
      return this._case;
    }

    set websocketOptions(v: BootstrapClientWebSocketOptions) {
      this.clear();
      this._websocketOptions = v;
      this._case = ClientOptionsCase.WEBSOCKET_OPTIONS;
    }

    get websocketOptions(): BootstrapClientWebSocketOptions {
      return this._websocketOptions;
    }
  }

  export enum ClientOptionsCase {
    NOT_SET = 0,
    WEBSOCKET_OPTIONS = 1,
  }

}

export interface ICreateBootstrapClientResponse {
  bootstrapClient?: IBootstrapClient;
}

export class CreateBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: ICreateBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: CreateBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IUpdateBootstrapClientRequest {
  id?: bigint;
  clientOptions?: UpdateBootstrapClientRequest.IClientOptionsOneOf
}

export class UpdateBootstrapClientRequest {
  id: bigint = BigInt(0);
  clientOptions: UpdateBootstrapClientRequest.ClientOptionsOneOf;

  constructor(v?: IUpdateBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
    this.clientOptions = new UpdateBootstrapClientRequest.ClientOptionsOneOf(v?.clientOptions);
  }

  static encode(m: UpdateBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.id) w.uint32(8).uint64(m.id);
    switch (m.clientOptions.case) {
      case 2:
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
        m.clientOptions.websocketOptions = BootstrapClientWebSocketOptions.decode(r, r.uint32());
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
  export type IClientOptionsOneOf =
  { websocketOptions: IBootstrapClientWebSocketOptions }
  ;

  export class ClientOptionsOneOf {
    private _websocketOptions: BootstrapClientWebSocketOptions | undefined;
    private _case: ClientOptionsCase = 0;

    constructor(v?: IClientOptionsOneOf) {
      if (v && "websocketOptions" in v) this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
    }

    public clear() {
      this._websocketOptions = undefined;
      this._case = ClientOptionsCase.NOT_SET;
    }

    get case(): ClientOptionsCase {
      return this._case;
    }

    set websocketOptions(v: BootstrapClientWebSocketOptions) {
      this.clear();
      this._websocketOptions = v;
      this._case = ClientOptionsCase.WEBSOCKET_OPTIONS;
    }

    get websocketOptions(): BootstrapClientWebSocketOptions {
      return this._websocketOptions;
    }
  }

  export enum ClientOptionsCase {
    NOT_SET = 0,
    WEBSOCKET_OPTIONS = 2,
  }

}

export interface IUpdateBootstrapClientResponse {
  bootstrapClient?: IBootstrapClient;
}

export class UpdateBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: IUpdateBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: UpdateBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IDeleteBootstrapClientRequest {
  id?: bigint;
}

export class DeleteBootstrapClientRequest {
  id: bigint = BigInt(0);

  constructor(v?: IDeleteBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IDeleteBootstrapClientResponse {
}

export class DeleteBootstrapClientResponse {

  constructor(v?: IDeleteBootstrapClientResponse) {
    // noop
  }

  static encode(m: DeleteBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteBootstrapClientResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteBootstrapClientResponse();
  }
}

export interface IGetBootstrapClientRequest {
  id?: bigint;
}

export class GetBootstrapClientRequest {
  id: bigint = BigInt(0);

  constructor(v?: IGetBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IGetBootstrapClientResponse {
  bootstrapClient?: IBootstrapClient;
}

export class GetBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: IGetBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: GetBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IListBootstrapClientsRequest {
}

export class ListBootstrapClientsRequest {

  constructor(v?: IListBootstrapClientsRequest) {
    // noop
  }

  static encode(m: ListBootstrapClientsRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapClientsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListBootstrapClientsRequest();
  }
}

export interface IListBootstrapClientsResponse {
  bootstrapClients?: IBootstrapClient[];
}

export class ListBootstrapClientsResponse {
  bootstrapClients: BootstrapClient[] = [];

  constructor(v?: IListBootstrapClientsResponse) {
    if (v?.bootstrapClients) this.bootstrapClients = v.bootstrapClients.map(v => new BootstrapClient(v));
  }

  static encode(m: ListBootstrapClientsResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IListBootstrapPeersRequest {
}

export class ListBootstrapPeersRequest {

  constructor(v?: IListBootstrapPeersRequest) {
    // noop
  }

  static encode(m: ListBootstrapPeersRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapPeersRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListBootstrapPeersRequest();
  }
}

export interface IListBootstrapPeersResponse {
  peers?: IBootstrapPeer[];
}

export class ListBootstrapPeersResponse {
  peers: BootstrapPeer[] = [];

  constructor(v?: IListBootstrapPeersResponse) {
    if (v?.peers) this.peers = v.peers.map(v => new BootstrapPeer(v));
  }

  static encode(m: ListBootstrapPeersResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IBootstrapPeer {
  peerId?: bigint;
  label?: string;
}

export class BootstrapPeer {
  peerId: bigint = BigInt(0);
  label: string = "";

  constructor(v?: IBootstrapPeer) {
    this.peerId = v?.peerId || BigInt(0);
    this.label = v?.label || "";
  }

  static encode(m: BootstrapPeer, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IBootstrapServiceMessage {
  body?: BootstrapServiceMessage.IBodyOneOf
}

export class BootstrapServiceMessage {
  body: BootstrapServiceMessage.BodyOneOf;

  constructor(v?: IBootstrapServiceMessage) {
    this.body = new BootstrapServiceMessage.BodyOneOf(v?.body);
  }

  static encode(m: BootstrapServiceMessage, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    switch (m.body.case) {
      case 1:
      BootstrapServiceMessage.BrokerOffer.encode(m.body.brokerOffer, w.uint32(10).fork()).ldelim();
      break;
      case 2:
      BootstrapServiceMessage.PublishRequest.encode(m.body.publishRequest, w.uint32(18).fork()).ldelim();
      break;
      case 3:
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
        m.body.brokerOffer = BootstrapServiceMessage.BrokerOffer.decode(r, r.uint32());
        break;
        case 2:
        m.body.publishRequest = BootstrapServiceMessage.PublishRequest.decode(r, r.uint32());
        break;
        case 3:
        m.body.publishResponse = BootstrapServiceMessage.PublishResponse.decode(r, r.uint32());
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
  export type IBodyOneOf =
  { brokerOffer: BootstrapServiceMessage.IBrokerOffer }
  |{ publishRequest: BootstrapServiceMessage.IPublishRequest }
  |{ publishResponse: BootstrapServiceMessage.IPublishResponse }
  ;

  export class BodyOneOf {
    private _brokerOffer: BootstrapServiceMessage.BrokerOffer | undefined;
    private _publishRequest: BootstrapServiceMessage.PublishRequest | undefined;
    private _publishResponse: BootstrapServiceMessage.PublishResponse | undefined;
    private _case: BodyCase = 0;

    constructor(v?: IBodyOneOf) {
      if (v && "brokerOffer" in v) this.brokerOffer = new BootstrapServiceMessage.BrokerOffer(v.brokerOffer);
      if (v && "publishRequest" in v) this.publishRequest = new BootstrapServiceMessage.PublishRequest(v.publishRequest);
      if (v && "publishResponse" in v) this.publishResponse = new BootstrapServiceMessage.PublishResponse(v.publishResponse);
    }

    public clear() {
      this._brokerOffer = undefined;
      this._publishRequest = undefined;
      this._publishResponse = undefined;
      this._case = BodyCase.NOT_SET;
    }

    get case(): BodyCase {
      return this._case;
    }

    set brokerOffer(v: BootstrapServiceMessage.BrokerOffer) {
      this.clear();
      this._brokerOffer = v;
      this._case = BodyCase.BROKER_OFFER;
    }

    get brokerOffer(): BootstrapServiceMessage.BrokerOffer {
      return this._brokerOffer;
    }

    set publishRequest(v: BootstrapServiceMessage.PublishRequest) {
      this.clear();
      this._publishRequest = v;
      this._case = BodyCase.PUBLISH_REQUEST;
    }

    get publishRequest(): BootstrapServiceMessage.PublishRequest {
      return this._publishRequest;
    }

    set publishResponse(v: BootstrapServiceMessage.PublishResponse) {
      this.clear();
      this._publishResponse = v;
      this._case = BodyCase.PUBLISH_RESPONSE;
    }

    get publishResponse(): BootstrapServiceMessage.PublishResponse {
      return this._publishResponse;
    }
  }

  export enum BodyCase {
    NOT_SET = 0,
    BROKER_OFFER = 1,
    PUBLISH_REQUEST = 2,
    PUBLISH_RESPONSE = 3,
  }

  export interface IBrokerOffer {
  }

  export class BrokerOffer {

    constructor(v?: IBrokerOffer) {
      // noop
    }

    static encode(m: BrokerOffer, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): BrokerOffer {
      if (r instanceof Reader && length) r.skip(length);
      return new BrokerOffer();
    }
  }

  export interface IPublishRequest {
    name?: string;
    certificate?: strims_type_ICertificate;
  }

  export class PublishRequest {
    name: string = "";
    certificate: strims_type_Certificate | undefined;

    constructor(v?: IPublishRequest) {
      this.name = v?.name || "";
      this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    }

    static encode(m: PublishRequest, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
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

  export interface IPublishResponse {
    body?: PublishResponse.IBodyOneOf
  }

  export class PublishResponse {
    body: PublishResponse.BodyOneOf;

    constructor(v?: IPublishResponse) {
      this.body = new PublishResponse.BodyOneOf(v?.body);
    }

    static encode(m: PublishResponse, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      switch (m.body.case) {
        case 1:
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
          m.body.error = r.string();
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
    export type IBodyOneOf =
    { error: string }
    ;

    export class BodyOneOf {
      private _error: string = "";
      private _case: BodyCase = 0;

      constructor(v?: IBodyOneOf) {
        if (v && "error" in v) this.error = v.error;
      }

      public clear() {
        this._error = "";
        this._case = BodyCase.NOT_SET;
      }

      get case(): BodyCase {
        return this._case;
      }

      set error(v: string) {
        this.clear();
        this._error = v;
        this._case = BodyCase.ERROR;
      }

      get error(): string {
        return this._error;
      }
    }

    export enum BodyCase {
      NOT_SET = 0,
      ERROR = 1,
    }

  }

}

export interface IPublishNetworkToBootstrapPeerRequest {
  peerId?: bigint;
  network?: strims_network_v1_INetwork;
}

export class PublishNetworkToBootstrapPeerRequest {
  peerId: bigint = BigInt(0);
  network: strims_network_v1_Network | undefined;

  constructor(v?: IPublishNetworkToBootstrapPeerRequest) {
    this.peerId = v?.peerId || BigInt(0);
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: PublishNetworkToBootstrapPeerRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
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

export interface IPublishNetworkToBootstrapPeerResponse {
}

export class PublishNetworkToBootstrapPeerResponse {

  constructor(v?: IPublishNetworkToBootstrapPeerResponse) {
    // noop
  }

  static encode(m: PublishNetworkToBootstrapPeerResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PublishNetworkToBootstrapPeerResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new PublishNetworkToBootstrapPeerResponse();
  }
}

