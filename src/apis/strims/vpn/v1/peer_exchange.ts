import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_vnic_v1_LinkDescription,
  strims_vnic_v1_ILinkDescription,
} from "../../vnic/v1/vnic";

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
      case PeerExchangeMessage.BodyCase.LINK_OFFER:
      strims_vpn_v1_PeerExchangeMessage_LinkOffer.encode(m.body.linkOffer, w.uint32(8010).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.LINK_ANSWER:
      strims_vpn_v1_PeerExchangeMessage_LinkAnswer.encode(m.body.linkAnswer, w.uint32(8018).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.REJECTION:
      strims_vpn_v1_PeerExchangeMessage_Rejection.encode(m.body.rejection, w.uint32(8026).fork()).ldelim();
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
        m.body = new PeerExchangeMessage.Body({ linkOffer: strims_vpn_v1_PeerExchangeMessage_LinkOffer.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new PeerExchangeMessage.Body({ linkAnswer: strims_vpn_v1_PeerExchangeMessage_LinkAnswer.decode(r, r.uint32()) });
        break;
        case 1003:
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
    LINK_OFFER = 1001,
    LINK_ANSWER = 1002,
    REJECTION = 1003,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.LINK_OFFER, linkOffer: strims_vpn_v1_PeerExchangeMessage_ILinkOffer }
  |{ case?: BodyCase.LINK_ANSWER, linkAnswer: strims_vpn_v1_PeerExchangeMessage_ILinkAnswer }
  |{ case?: BodyCase.REJECTION, rejection: strims_vpn_v1_PeerExchangeMessage_IRejection }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.LINK_OFFER, linkOffer: strims_vpn_v1_PeerExchangeMessage_LinkOffer }
  |{ case: BodyCase.LINK_ANSWER, linkAnswer: strims_vpn_v1_PeerExchangeMessage_LinkAnswer }
  |{ case: BodyCase.REJECTION, rejection: strims_vpn_v1_PeerExchangeMessage_Rejection }
  >;

  class BodyImpl {
    linkOffer: strims_vpn_v1_PeerExchangeMessage_LinkOffer;
    linkAnswer: strims_vpn_v1_PeerExchangeMessage_LinkAnswer;
    rejection: strims_vpn_v1_PeerExchangeMessage_Rejection;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "linkOffer" in v) {
        this.case = BodyCase.LINK_OFFER;
        this.linkOffer = new strims_vpn_v1_PeerExchangeMessage_LinkOffer(v.linkOffer);
      } else
      if (v && "linkAnswer" in v) {
        this.case = BodyCase.LINK_ANSWER;
        this.linkAnswer = new strims_vpn_v1_PeerExchangeMessage_LinkAnswer(v.linkAnswer);
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
    T extends { linkOffer: strims_vpn_v1_PeerExchangeMessage_ILinkOffer } ? { case: BodyCase.LINK_OFFER, linkOffer: strims_vpn_v1_PeerExchangeMessage_LinkOffer } :
    T extends { linkAnswer: strims_vpn_v1_PeerExchangeMessage_ILinkAnswer } ? { case: BodyCase.LINK_ANSWER, linkAnswer: strims_vpn_v1_PeerExchangeMessage_LinkAnswer } :
    T extends { rejection: strims_vpn_v1_PeerExchangeMessage_IRejection } ? { case: BodyCase.REJECTION, rejection: strims_vpn_v1_PeerExchangeMessage_Rejection } :
    never
    >;
  };

  export type ILinkOffer = {
    exchangeId?: bigint;
    descriptions?: strims_vnic_v1_ILinkDescription[];
  }

  export class LinkOffer {
    exchangeId: bigint;
    descriptions: strims_vnic_v1_LinkDescription[];

    constructor(v?: ILinkOffer) {
      this.exchangeId = v?.exchangeId || BigInt(0);
      this.descriptions = v?.descriptions ? v.descriptions.map(v => new strims_vnic_v1_LinkDescription(v)) : [];
    }

    static encode(m: LinkOffer, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.exchangeId) w.uint32(8).uint64(m.exchangeId);
      for (const v of m.descriptions) strims_vnic_v1_LinkDescription.encode(v, w.uint32(18).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): LinkOffer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new LinkOffer();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.exchangeId = r.uint64();
          break;
          case 2:
          m.descriptions.push(strims_vnic_v1_LinkDescription.decode(r, r.uint32()));
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type ILinkAnswer = {
    exchangeId?: bigint;
    descriptions?: strims_vnic_v1_ILinkDescription[];
    errorMessage?: string;
  }

  export class LinkAnswer {
    exchangeId: bigint;
    descriptions: strims_vnic_v1_LinkDescription[];
    errorMessage: string;

    constructor(v?: ILinkAnswer) {
      this.exchangeId = v?.exchangeId || BigInt(0);
      this.descriptions = v?.descriptions ? v.descriptions.map(v => new strims_vnic_v1_LinkDescription(v)) : [];
      this.errorMessage = v?.errorMessage || "";
    }

    static encode(m: LinkAnswer, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.exchangeId) w.uint32(8).uint64(m.exchangeId);
      for (const v of m.descriptions) strims_vnic_v1_LinkDescription.encode(v, w.uint32(18).fork()).ldelim();
      if (m.errorMessage.length) w.uint32(26).string(m.errorMessage);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): LinkAnswer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new LinkAnswer();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.exchangeId = r.uint64();
          break;
          case 2:
          m.descriptions.push(strims_vnic_v1_LinkDescription.decode(r, r.uint32()));
          break;
          case 3:
          m.errorMessage = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
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
export const strims_vpn_v1_PeerExchangeMessage_LinkOffer = PeerExchangeMessage.LinkOffer;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_LinkOffer = PeerExchangeMessage.LinkOffer;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_ILinkOffer = PeerExchangeMessage.ILinkOffer;
/* @internal */
export const strims_vpn_v1_PeerExchangeMessage_LinkAnswer = PeerExchangeMessage.LinkAnswer;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_LinkAnswer = PeerExchangeMessage.LinkAnswer;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_ILinkAnswer = PeerExchangeMessage.ILinkAnswer;
/* @internal */
export const strims_vpn_v1_PeerExchangeMessage_Rejection = PeerExchangeMessage.Rejection;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_Rejection = PeerExchangeMessage.Rejection;
/* @internal */
export type strims_vpn_v1_PeerExchangeMessage_IRejection = PeerExchangeMessage.IRejection;
