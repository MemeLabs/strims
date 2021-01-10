import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";


export interface IPeerExchangeMessage {
  body?: PeerExchangeMessage.IBodyOneOf
}

export class PeerExchangeMessage {
  body: PeerExchangeMessage.BodyOneOf;

  constructor(v?: IPeerExchangeMessage) {
    this.body = new PeerExchangeMessage.BodyOneOf(v?.body);
  }

  static encode(m: PeerExchangeMessage, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    switch (m.body.case) {
      case 1:
      PeerExchangeMessage.Request.encode(m.body.request, w.uint32(10).fork()).ldelim();
      break;
      case 2:
      PeerExchangeMessage.Response.encode(m.body.response, w.uint32(18).fork()).ldelim();
      break;
      case 3:
      PeerExchangeMessage.Offer.encode(m.body.offer, w.uint32(26).fork()).ldelim();
      break;
      case 4:
      PeerExchangeMessage.Answer.encode(m.body.answer, w.uint32(34).fork()).ldelim();
      break;
      case 5:
      PeerExchangeMessage.IceCandidate.encode(m.body.iceCandidate, w.uint32(42).fork()).ldelim();
      break;
      case 6:
      PeerExchangeMessage.CallbackRequest.encode(m.body.callbackRequest, w.uint32(50).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerExchangeMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerExchangeMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body.request = PeerExchangeMessage.Request.decode(r, r.uint32());
        break;
        case 2:
        m.body.response = PeerExchangeMessage.Response.decode(r, r.uint32());
        break;
        case 3:
        m.body.offer = PeerExchangeMessage.Offer.decode(r, r.uint32());
        break;
        case 4:
        m.body.answer = PeerExchangeMessage.Answer.decode(r, r.uint32());
        break;
        case 5:
        m.body.iceCandidate = PeerExchangeMessage.IceCandidate.decode(r, r.uint32());
        break;
        case 6:
        m.body.callbackRequest = PeerExchangeMessage.CallbackRequest.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace PeerExchangeMessage {
  export type IBodyOneOf =
  { request: PeerExchangeMessage.IRequest }
  |{ response: PeerExchangeMessage.IResponse }
  |{ offer: PeerExchangeMessage.IOffer }
  |{ answer: PeerExchangeMessage.IAnswer }
  |{ iceCandidate: PeerExchangeMessage.IIceCandidate }
  |{ callbackRequest: PeerExchangeMessage.ICallbackRequest }
  ;

  export class BodyOneOf {
    private _request: PeerExchangeMessage.Request | undefined;
    private _response: PeerExchangeMessage.Response | undefined;
    private _offer: PeerExchangeMessage.Offer | undefined;
    private _answer: PeerExchangeMessage.Answer | undefined;
    private _iceCandidate: PeerExchangeMessage.IceCandidate | undefined;
    private _callbackRequest: PeerExchangeMessage.CallbackRequest | undefined;
    private _case: BodyCase = 0;

    constructor(v?: IBodyOneOf) {
      if (v && "request" in v) this.request = new PeerExchangeMessage.Request(v.request);
      if (v && "response" in v) this.response = new PeerExchangeMessage.Response(v.response);
      if (v && "offer" in v) this.offer = new PeerExchangeMessage.Offer(v.offer);
      if (v && "answer" in v) this.answer = new PeerExchangeMessage.Answer(v.answer);
      if (v && "iceCandidate" in v) this.iceCandidate = new PeerExchangeMessage.IceCandidate(v.iceCandidate);
      if (v && "callbackRequest" in v) this.callbackRequest = new PeerExchangeMessage.CallbackRequest(v.callbackRequest);
    }

    public clear() {
      this._request = undefined;
      this._response = undefined;
      this._offer = undefined;
      this._answer = undefined;
      this._iceCandidate = undefined;
      this._callbackRequest = undefined;
      this._case = BodyCase.NOT_SET;
    }

    get case(): BodyCase {
      return this._case;
    }

    set request(v: PeerExchangeMessage.Request) {
      this.clear();
      this._request = v;
      this._case = BodyCase.REQUEST;
    }

    get request(): PeerExchangeMessage.Request {
      return this._request;
    }

    set response(v: PeerExchangeMessage.Response) {
      this.clear();
      this._response = v;
      this._case = BodyCase.RESPONSE;
    }

    get response(): PeerExchangeMessage.Response {
      return this._response;
    }

    set offer(v: PeerExchangeMessage.Offer) {
      this.clear();
      this._offer = v;
      this._case = BodyCase.OFFER;
    }

    get offer(): PeerExchangeMessage.Offer {
      return this._offer;
    }

    set answer(v: PeerExchangeMessage.Answer) {
      this.clear();
      this._answer = v;
      this._case = BodyCase.ANSWER;
    }

    get answer(): PeerExchangeMessage.Answer {
      return this._answer;
    }

    set iceCandidate(v: PeerExchangeMessage.IceCandidate) {
      this.clear();
      this._iceCandidate = v;
      this._case = BodyCase.ICE_CANDIDATE;
    }

    get iceCandidate(): PeerExchangeMessage.IceCandidate {
      return this._iceCandidate;
    }

    set callbackRequest(v: PeerExchangeMessage.CallbackRequest) {
      this.clear();
      this._callbackRequest = v;
      this._case = BodyCase.CALLBACK_REQUEST;
    }

    get callbackRequest(): PeerExchangeMessage.CallbackRequest {
      return this._callbackRequest;
    }
  }

  export enum BodyCase {
    NOT_SET = 0,
    REQUEST = 1,
    RESPONSE = 2,
    OFFER = 3,
    ANSWER = 4,
    ICE_CANDIDATE = 5,
    CALLBACK_REQUEST = 6,
  }

  export interface IRequest {
    count?: number;
  }

  export class Request {
    count: number = 0;

    constructor(v?: IRequest) {
      this.count = v?.count || 0;
    }

    static encode(m: Request, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.count) w.uint32(8).uint32(m.count);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Request {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Request();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.count = r.uint32();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IResponse {
    ids?: Uint8Array[];
  }

  export class Response {
    ids: Uint8Array[] = [];

    constructor(v?: IResponse) {
      if (v?.ids) this.ids = v.ids;
    }

    static encode(m: Response, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      for (const v of m.ids) w.uint32(10).bytes(v);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Response {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Response();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.ids.push(r.bytes())
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IOffer {
    mediationId?: bigint;
    data?: Uint8Array;
  }

  export class Offer {
    mediationId: bigint = BigInt(0);
    data: Uint8Array = new Uint8Array();

    constructor(v?: IOffer) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: Offer, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.data) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Offer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Offer();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.mediationId = r.uint64();
          break;
          case 2:
          m.data = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IAnswer {
    mediationId?: bigint;
    data?: Uint8Array;
  }

  export class Answer {
    mediationId: bigint = BigInt(0);
    data: Uint8Array = new Uint8Array();

    constructor(v?: IAnswer) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: Answer, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.data) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Answer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Answer();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.mediationId = r.uint64();
          break;
          case 2:
          m.data = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IIceCandidate {
    mediationId?: bigint;
    index?: bigint;
    data?: Uint8Array;
  }

  export class IceCandidate {
    mediationId: bigint = BigInt(0);
    index: bigint = BigInt(0);
    data: Uint8Array = new Uint8Array();

    constructor(v?: IIceCandidate) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.index = v?.index || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: IceCandidate, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.index) w.uint32(16).uint64(m.index);
      if (m.data) w.uint32(26).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): IceCandidate {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new IceCandidate();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.mediationId = r.uint64();
          break;
          case 2:
          m.index = r.uint64();
          break;
          case 3:
          m.data = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ICallbackRequest {
  }

  export class CallbackRequest {

    constructor(v?: ICallbackRequest) {
      // noop
    }

    static encode(m: CallbackRequest, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): CallbackRequest {
      if (r instanceof Reader && length) r.skip(length);
      return new CallbackRequest();
    }
  }

}

