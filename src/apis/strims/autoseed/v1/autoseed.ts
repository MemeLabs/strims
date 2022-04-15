import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IConfig = {
  enable?: boolean;
}

export class Config {
  enable: boolean;

  constructor(v?: IConfig) {
    this.enable = v?.enable || false;
  }

  static encode(m: Config, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enable) w.uint32(8).bool(m.enable);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Config {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Config();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enable = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IRule = {
  id?: bigint;
  networkKey?: Uint8Array;
  swarmId?: Uint8Array;
  salt?: Uint8Array;
  label?: string;
}

export class Rule {
  id: bigint;
  networkKey: Uint8Array;
  swarmId: Uint8Array;
  salt: Uint8Array;
  label: string;

  constructor(v?: IRule) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.swarmId = v?.swarmId || new Uint8Array();
    this.salt = v?.salt || new Uint8Array();
    this.label = v?.label || "";
  }

  static encode(m: Rule, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.swarmId.length) w.uint32(26).bytes(m.swarmId);
    if (m.salt.length) w.uint32(34).bytes(m.salt);
    if (m.label.length) w.uint32(42).string(m.label);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Rule {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Rule();
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
        m.swarmId = r.bytes();
        break;
        case 4:
        m.salt = r.bytes();
        break;
        case 5:
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

export type IGetConfigRequest = {
}

export class GetConfigRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IGetConfigRequest) {
  }

  static encode(m: GetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetConfigRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new GetConfigRequest();
  }
}

export type IGetConfigResponse = {
  config?: IConfig;
}

export class GetConfigResponse {
  config: Config | undefined;

  constructor(v?: IGetConfigResponse) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: GetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISetConfigRequest = {
  config?: IConfig;
}

export class SetConfigRequest {
  config: Config | undefined;

  constructor(v?: ISetConfigRequest) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: SetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SetConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISetConfigResponse = {
  config?: IConfig;
}

export class SetConfigResponse {
  config: Config | undefined;

  constructor(v?: ISetConfigResponse) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: SetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateRuleRequest = {
  rule?: IRule;
}

export class CreateRuleRequest {
  rule: Rule | undefined;

  constructor(v?: ICreateRuleRequest) {
    this.rule = v?.rule && new Rule(v.rule);
  }

  static encode(m: CreateRuleRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.rule) Rule.encode(m.rule, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateRuleRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateRuleRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.rule = Rule.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateRuleResponse = {
  rule?: IRule;
}

export class CreateRuleResponse {
  rule: Rule | undefined;

  constructor(v?: ICreateRuleResponse) {
    this.rule = v?.rule && new Rule(v.rule);
  }

  static encode(m: CreateRuleResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.rule) Rule.encode(m.rule, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateRuleResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateRuleResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.rule = Rule.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateRuleRequest = {
  id?: bigint;
  rule?: IRule;
}

export class UpdateRuleRequest {
  id: bigint;
  rule: Rule | undefined;

  constructor(v?: IUpdateRuleRequest) {
    this.id = v?.id || BigInt(0);
    this.rule = v?.rule && new Rule(v.rule);
  }

  static encode(m: UpdateRuleRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.rule) Rule.encode(m.rule, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateRuleRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateRuleRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.rule = Rule.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateRuleResponse = {
  rule?: IRule;
}

export class UpdateRuleResponse {
  rule: Rule | undefined;

  constructor(v?: IUpdateRuleResponse) {
    this.rule = v?.rule && new Rule(v.rule);
  }

  static encode(m: UpdateRuleResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.rule) Rule.encode(m.rule, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateRuleResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateRuleResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.rule = Rule.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteRuleRequest = {
  id?: bigint;
}

export class DeleteRuleRequest {
  id: bigint;

  constructor(v?: IDeleteRuleRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteRuleRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteRuleRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteRuleRequest();
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

export type IDeleteRuleResponse = {
}

export class DeleteRuleResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteRuleResponse) {
  }

  static encode(m: DeleteRuleResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteRuleResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteRuleResponse();
  }
}

export type IGetRuleRequest = {
  id?: bigint;
}

export class GetRuleRequest {
  id: bigint;

  constructor(v?: IGetRuleRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetRuleRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetRuleRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetRuleRequest();
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

export type IGetRuleResponse = {
  rule?: IRule;
}

export class GetRuleResponse {
  rule: Rule | undefined;

  constructor(v?: IGetRuleResponse) {
    this.rule = v?.rule && new Rule(v.rule);
  }

  static encode(m: GetRuleResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.rule) Rule.encode(m.rule, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetRuleResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetRuleResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.rule = Rule.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListRulesRequest = {
}

export class ListRulesRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IListRulesRequest) {
  }

  static encode(m: ListRulesRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListRulesRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListRulesRequest();
  }
}

export type IListRulesResponse = {
  rules?: IRule[];
}

export class ListRulesResponse {
  rules: Rule[];

  constructor(v?: IListRulesResponse) {
    this.rules = v?.rules ? v.rules.map(v => new Rule(v)) : [];
  }

  static encode(m: ListRulesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.rules) Rule.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListRulesResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListRulesResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.rules.push(Rule.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

