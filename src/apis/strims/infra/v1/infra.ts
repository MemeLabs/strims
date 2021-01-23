import Reader from "../../../../lib/pb/reader";
import Writer from "../../../../lib/pb/writer";

import {
  LatLng as strims_type_LatLng,
  ILatLng as strims_type_ILatLng,
} from "../../type/latlng";

export interface INode {
  user?: string;
  driver?: string;
  providerName?: string;
  providerId?: string;
  name?: string;
  wireguardPrivKey?: string;
  wireguardIpv4?: string;
  status?: string;
  startedAt?: bigint;
  stoppedAt?: bigint;
  networks?: Node.INetworks | undefined;
  region?: Node.IRegion | undefined;
  sku?: Node.ISKU | undefined;
  usage?: Node.IUsage | undefined;
}

export class Node {
  user: string = "";
  driver: string = "";
  providerName: string = "";
  providerId: string = "";
  name: string = "";
  wireguardPrivKey: string = "";
  wireguardIpv4: string = "";
  status: string = "";
  startedAt: bigint = BigInt(0);
  stoppedAt: bigint = BigInt(0);
  networks: Node.Networks | undefined;
  region: Node.Region | undefined;
  sku: Node.SKU | undefined;
  usage: Node.Usage | undefined;

  constructor(v?: INode) {
    this.user = v?.user || "";
    this.driver = v?.driver || "";
    this.providerName = v?.providerName || "";
    this.providerId = v?.providerId || "";
    this.name = v?.name || "";
    this.wireguardPrivKey = v?.wireguardPrivKey || "";
    this.wireguardIpv4 = v?.wireguardIpv4 || "";
    this.status = v?.status || "";
    this.startedAt = v?.startedAt || BigInt(0);
    this.stoppedAt = v?.stoppedAt || BigInt(0);
    this.networks = v?.networks && new Node.Networks(v.networks);
    this.region = v?.region && new Node.Region(v.region);
    this.sku = v?.sku && new Node.SKU(v.sku);
    this.usage = v?.usage && new Node.Usage(v.usage);
  }

  static encode(m: Node, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.user) w.uint32(10).string(m.user);
    if (m.driver) w.uint32(18).string(m.driver);
    if (m.providerName) w.uint32(26).string(m.providerName);
    if (m.providerId) w.uint32(34).string(m.providerId);
    if (m.name) w.uint32(42).string(m.name);
    if (m.wireguardPrivKey) w.uint32(50).string(m.wireguardPrivKey);
    if (m.wireguardIpv4) w.uint32(58).string(m.wireguardIpv4);
    if (m.status) w.uint32(66).string(m.status);
    if (m.startedAt) w.uint32(96).int64(m.startedAt);
    if (m.stoppedAt) w.uint32(104).int64(m.stoppedAt);
    if (m.networks) Node.Networks.encode(m.networks, w.uint32(114).fork()).ldelim();
    if (m.region) Node.Region.encode(m.region, w.uint32(122).fork()).ldelim();
    if (m.sku) Node.SKU.encode(m.sku, w.uint32(130).fork()).ldelim();
    if (m.usage) Node.Usage.encode(m.usage, w.uint32(138).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Node {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Node();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.user = r.string();
        break;
        case 2:
        m.driver = r.string();
        break;
        case 3:
        m.providerName = r.string();
        break;
        case 4:
        m.providerId = r.string();
        break;
        case 5:
        m.name = r.string();
        break;
        case 6:
        m.wireguardPrivKey = r.string();
        break;
        case 7:
        m.wireguardIpv4 = r.string();
        break;
        case 8:
        m.status = r.string();
        break;
        case 12:
        m.startedAt = r.int64();
        break;
        case 13:
        m.stoppedAt = r.int64();
        break;
        case 14:
        m.networks = Node.Networks.decode(r, r.uint32());
        break;
        case 15:
        m.region = Node.Region.decode(r, r.uint32());
        break;
        case 16:
        m.sku = Node.SKU.decode(r, r.uint32());
        break;
        case 17:
        m.usage = Node.Usage.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Node {
  export interface INetworks {
    v4?: string[];
    v6?: string[];
  }

  export class Networks {
    v4: string[] = [];
    v6: string[] = [];

    constructor(v?: INetworks) {
      if (v?.v4) this.v4 = v.v4;
      if (v?.v6) this.v6 = v.v6;
    }

    static encode(m: Networks, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.v4) w.uint32(10).string(v);
      for (const v of m.v6) w.uint32(18).string(v);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Networks {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Networks();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.v4.push(r.string())
          break;
          case 2:
          m.v6.push(r.string())
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IRegion {
    name?: string;
    city?: string;
    latLng?: strims_type_ILatLng | undefined;
  }

  export class Region {
    name: string = "";
    city: string = "";
    latLng: strims_type_LatLng | undefined;

    constructor(v?: IRegion) {
      this.name = v?.name || "";
      this.city = v?.city || "";
      this.latLng = v?.latLng && new strims_type_LatLng(v.latLng);
    }

    static encode(m: Region, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.name) w.uint32(10).string(m.name);
      if (m.city) w.uint32(18).string(m.city);
      if (m.latLng) strims_type_LatLng.encode(m.latLng, w.uint32(26).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Region {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Region();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.name = r.string();
          break;
          case 2:
          m.city = r.string();
          break;
          case 3:
          m.latLng = strims_type_LatLng.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IPrice {
    value?: number;
    currency?: string;
  }

  export class Price {
    value: number = 0;
    currency: string = "";

    constructor(v?: IPrice) {
      this.value = v?.value || 0;
      this.currency = v?.currency || "";
    }

    static encode(m: Price, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.value) w.uint32(9).double(m.value);
      if (m.currency) w.uint32(18).string(m.currency);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Price {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Price();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.value = r.double();
          break;
          case 2:
          m.currency = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ISKU {
    name?: string;
    memory?: number;
    cpus?: number;
    disk?: number;
    networkCap?: number;
    networkSpeed?: number;
    priceMonthly?: Node.IPrice | undefined;
    priceHourly?: Node.IPrice | undefined;
  }

  export class SKU {
    name: string = "";
    memory: number = 0;
    cpus: number = 0;
    disk: number = 0;
    networkCap: number = 0;
    networkSpeed: number = 0;
    priceMonthly: Node.Price | undefined;
    priceHourly: Node.Price | undefined;

    constructor(v?: ISKU) {
      this.name = v?.name || "";
      this.memory = v?.memory || 0;
      this.cpus = v?.cpus || 0;
      this.disk = v?.disk || 0;
      this.networkCap = v?.networkCap || 0;
      this.networkSpeed = v?.networkSpeed || 0;
      this.priceMonthly = v?.priceMonthly && new Node.Price(v.priceMonthly);
      this.priceHourly = v?.priceHourly && new Node.Price(v.priceHourly);
    }

    static encode(m: SKU, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.name) w.uint32(10).string(m.name);
      if (m.memory) w.uint32(16).int32(m.memory);
      if (m.cpus) w.uint32(24).int32(m.cpus);
      if (m.disk) w.uint32(32).int32(m.disk);
      if (m.networkCap) w.uint32(40).int32(m.networkCap);
      if (m.networkSpeed) w.uint32(48).int32(m.networkSpeed);
      if (m.priceMonthly) Node.Price.encode(m.priceMonthly, w.uint32(58).fork()).ldelim();
      if (m.priceHourly) Node.Price.encode(m.priceHourly, w.uint32(66).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): SKU {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new SKU();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.name = r.string();
          break;
          case 2:
          m.memory = r.int32();
          break;
          case 3:
          m.cpus = r.int32();
          break;
          case 4:
          m.disk = r.int32();
          break;
          case 5:
          m.networkCap = r.int32();
          break;
          case 6:
          m.networkSpeed = r.int32();
          break;
          case 7:
          m.priceMonthly = Node.Price.decode(r, r.uint32());
          break;
          case 8:
          m.priceHourly = Node.Price.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IUsage {
    networkIn?: number;
    networkOut?: number;
    cpuUsage?: number;
    memUsage?: number;
    uptime?: bigint;
    cost?: Node.IPrice | undefined;
  }

  export class Usage {
    networkIn: number = 0;
    networkOut: number = 0;
    cpuUsage: number = 0;
    memUsage: number = 0;
    uptime: bigint = BigInt(0);
    cost: Node.Price | undefined;

    constructor(v?: IUsage) {
      this.networkIn = v?.networkIn || 0;
      this.networkOut = v?.networkOut || 0;
      this.cpuUsage = v?.cpuUsage || 0;
      this.memUsage = v?.memUsage || 0;
      this.uptime = v?.uptime || BigInt(0);
      this.cost = v?.cost && new Node.Price(v.cost);
    }

    static encode(m: Usage, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.networkIn) w.uint32(9).double(m.networkIn);
      if (m.networkOut) w.uint32(17).double(m.networkOut);
      if (m.cpuUsage) w.uint32(25).double(m.cpuUsage);
      if (m.memUsage) w.uint32(33).double(m.memUsage);
      if (m.uptime) w.uint32(40).int64(m.uptime);
      if (m.cost) Node.Price.encode(m.cost, w.uint32(50).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Usage {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Usage();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.networkIn = r.double();
          break;
          case 2:
          m.networkOut = r.double();
          break;
          case 3:
          m.cpuUsage = r.double();
          break;
          case 4:
          m.memUsage = r.double();
          break;
          case 5:
          m.uptime = r.int64();
          break;
          case 6:
          m.cost = Node.Price.decode(r, r.uint32());
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

export interface IHistory {
  nodes?: INode[];
}

export class History {
  nodes: Node[] = [];

  constructor(v?: IHistory) {
    if (v?.nodes) this.nodes = v.nodes.map(v => new Node(v));
  }

  static encode(m: History, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.nodes) Node.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): History {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new History();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.nodes.push(Node.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IGetHistoryRequest {
}

export class GetHistoryRequest {

  constructor(v?: IGetHistoryRequest) {
    // noop
  }

  static encode(m: GetHistoryRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetHistoryRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new GetHistoryRequest();
  }
}

export interface IGetHistoryResponse {
  history?: IHistory | undefined;
}

export class GetHistoryResponse {
  history: History | undefined;

  constructor(v?: IGetHistoryResponse) {
    this.history = v?.history && new History(v.history);
  }

  static encode(m: GetHistoryResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.history) History.encode(m.history, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetHistoryResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetHistoryResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.history = History.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

