import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IPeerExchangeMessage = {
  body?: PeerExchangeMessage.IBody
}

export class PeerExchangeMessage {
  body: PeerExchangeMessage.TBody;

  constructor(v?: IPeerExchangeMessage) {
    this.body = new PeerExchangeMessage.Body(v?.body);
  }

  static encode(m: PeerExchangeMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case PeerExchangeMessage.BodyCase.MEDIATION_OFFER:
      strims_vpn_v1_PeerExchangeMessage_MediationOffer.encode(m.body.mediationOffer, w.uint32(8010).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.MEDIATION_ANSWER:
      strims_vpn_v1_PeerExchangeMessage_MediationAnswer.encode(m.body.mediationAnswer, w.uint32(8018).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.CALLBACK_REQUEST:
      strims_vpn_v1_PeerExchangeMessage_CallbackRequest.encode(m.body.callbackRequest, w.uint32(8026).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.REJECTION:
      strims_vpn_v1_PeerExchangeMessage_Rejection.encode(m.body.rejection, w.uint32(8034).fork()).ldelim();
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
        case 1001:
        m.body = new PeerExchangeMessage.Body({ mediationOffer: strims_vpn_v1_PeerExchangeMessage_MediationOffer.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new PeerExchangeMessage.Body({ mediationAnswer: strims_vpn_v1_PeerExchangeMessage_MediationAnswer.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new PeerExchangeMessage.Body({ callbackRequest: strims_vpn_v1_PeerExchangeMessage_CallbackRequest.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new PeerExchangeMessage.Body({ rejection: strims_vpn_v1_PeerExchangeMessage_Rejection.decode(r, r.uint32()) });
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
  export enum BodyCase {
    NOT_SET = 0,
    MEDIATION_OFFER = 1001,
    MEDIATION_ANSWER = 1002,
    CALLBACK_REQUEST = 1003,
    REJECTION = 1004,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.MEDIATION_OFFER, mediationOffer: strims_vpn_v1_PeerExchangeMessage_IMediationOffer }
  |{ case?: BodyCase.MEDIATION_ANSWER, mediationAnswer: strims_vpn_v1_PeerExchangeMessage_IMediationAnswer }
  |{ case?: BodyCase.CALLBACK_REQUEST, callbackRequest: strims_vpn_v1_PeerExchangeMessage_ICallbackRequest }
  |{ case?: BodyCase.REJECTION, rejection: strims_vpn_v1_PeerExchangeMessage_IRejection }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.MEDIATION_OFFER, mediationOffer: strims_vpn_v1_PeerExchangeMessage_MediationOffer }
  |{ case: BodyCase.MEDIATION_ANSWER, mediationAnswer: strims_vpn_v1_PeerExchangeMessage_MediationAnswer }
  |{ case: BodyCase.CALLBACK_REQUEST, callbackRequest: strims_vpn_v1_PeerExchangeMessage_CallbackRequest }
  |{ case: BodyCase.REJECTION, rejection: strims_vpn_v1_PeerExchangeMessage_Rejection }
  >;

  class BodyImpl {
    mediationOffer: strims_vpn_v1_PeerExchangeMessage_MediationOffer;
    mediationAnswer: strims_vpn_v1_PeerExchangeMessage_MediationAnswer;
    callbackRequest: strims_vpn_v1_PeerExchangeMessage_CallbackRequest;
    rejection: strims_vpn_v1_PeerExchangeMessage_Rejection;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "mediationOffer" in v) {
        this.case = BodyCase.MEDIATION_OFFER;
        this.mediationOffer = new strims_vpn_v1_PeerExchangeMessage_MediationOffer(v.mediationOffer);
      } else
      if (v && "mediationAnswer" in v) {
        this.case = BodyCase.MEDIATION_ANSWER;
        this.mediationAnswer = new strims_vpn_v1_PeerExchangeMessage_MediationAnswer(v.mediationAnswer);
      } else
      if (v && "callbackRequest" in v) {
        this.case = BodyCase.CALLBACK_REQUEST;
        this.callbackRequest = new strims_vpn_v1_PeerExchangeMessage_CallbackRequest(v.callbackRequest);
      } else
      if (v && "rejection" in v) {
        this.case = BodyCase.REJECTION;
        this.rejection = new strims_vpn_v1_PeerExchangeMessage_Rejection(v.rejection);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { mediationOffer: strims_vpn_v1_PeerExchangeMessage_IMediationOffer } ? { case: BodyCase.MEDIATION_OFFER, mediationOffer: strims_vpn_v1_PeerExchangeMessage_MediationOffer } :
    T extends { mediationAnswer: strims_vpn_v1_PeerExchangeMessage_IMediationAnswer } ? { case: BodyCase.MEDIATION_ANSWER, mediationAnswer: strims_vpn_v1_PeerExchangeMessage_MediationAnswer } :
    T extends { callbackRequest: strims_vpn_v1_PeerExchangeMessage_ICallbackRequest } ? { case: BodyCase.CALLBACK_REQUEST, callbackRequest: strims_vpn_v1_PeerExchangeMessage_CallbackRequest } :
    T extends { rejection: strims_vpn_v1_PeerExchangeMessage_IRejection } ? { case: BodyCase.REJECTION, rejection: strims_vpn_v1_PeerExchangeMessage_Rejection } :
    never
    >;
  };

  export type IMediationOffer = {
    mediationId?: bigint;
    data?: Uint8Array;
  }

  export class MediationOffer {
    mediationId: bigint;
    data: Uint8Array;

    constructor(v?: IMediationOffer) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: MediationOffer, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.data.length) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): MediationOffer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new MediationOffer();
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

  export type IMediationAnswer = {
    mediationId?: bigint;
    data?: Uint8Array;
  }

  export class MediationAnswer {
    mediationId: bigint;
    data: Uint8Array;

    constructor(v?: IMediationAnswer) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: MediationAnswer, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.data.length) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): MediationAnswer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new MediationAnswer();
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

  export type ICallbackRequest = Record<string, any>;

  export class CallbackRequest {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: ICallbackRequest) {
    }

    static encode(m: CallbackRequest, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): CallbackRequest {
      if (r instanceof Reader && length) r.skip(length);
      return new CallbackRequest();
    }
  }

  export type IRejection = {
    mediationId?: bigint;
  }

  export class Rejection {
    mediationId: bigint;

    constructor(v?: IRejection) {
      this.mediationId = v?.mediationId || BigInt(0);
    }

    static encode(m: Rejection, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Rejection {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Rejection();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.mediationId = r.uint64();
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

/* @internal */
export const strims_vpn_v1_PeerExchangeMessage = PeerExchangeMessage;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage = PeerExchangeMessage;
/* @internal */
export type strims_vpn_v1_IPeerExchangeMessage = IPeerExchangeMessage;
/* @internal */
export const strims_vpn_v1_PeerExchangeMessage_MediationOffer = PeerExchangeMessage.MediationOffer;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_MediationOffer = PeerExchangeMessage.MediationOffer;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_IMediationOffer = PeerExchangeMessage.IMediationOffer;
/* @internal */
export const strims_vpn_v1_PeerExchangeMessage_MediationAnswer = PeerExchangeMessage.MediationAnswer;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_MediationAnswer = PeerExchangeMessage.MediationAnswer;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_IMediationAnswer = PeerExchangeMessage.IMediationAnswer;
/* @internal */
export const strims_vpn_v1_PeerExchangeMessage_CallbackRequest = PeerExchangeMessage.CallbackRequest;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_CallbackRequest = PeerExchangeMessage.CallbackRequest;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_ICallbackRequest = PeerExchangeMessage.ICallbackRequest;
/* @internal */
export const strims_vpn_v1_PeerExchangeMessage_Rejection = PeerExchangeMessage.Rejection;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_Rejection = PeerExchangeMessage.Rejection;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_IRejection = PeerExchangeMessage.IRejection;
