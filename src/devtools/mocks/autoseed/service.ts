// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as autoseedv1 from "../../../apis/strims/autoseed/v1/autoseed";
import {
  AutoseedFrontendService,
  UnimplementedAutoseedFrontendService,
} from "../../../apis/strims/autoseed/v1/autoseed_rpc";
import { bytes } from "../../storybook/fixtures/ids";
import { networkKey } from "../../storybook/fixtures/networks";

const labels = [
  "Main stage",
  "Speedrun relay",
  "Film night",
  "Late night lofi",
  "Retro marathon",
  "Music lounge",
];

// Rule `i`, from 1: a swarm (20 byte id) of fixture network ((i - 1) % 5) + 1.
export const rule = (i: number, opts: Partial<autoseedv1.IRule> = {}): autoseedv1.Rule =>
  new autoseedv1.Rule({
    id: BigInt(i),
    label: labels[(i - 1) % labels.length] + (i <= labels.length ? "" : ` ${i}`),
    networkKey: networkKey(((i - 1) % 5) + 1),
    swarmId: bytes(`swarm:${i}`, 20),
    salt: new TextEncoder().encode(i % 2 ? "" : `salt-${i}`),
    ...opts,
  });

// Rules 1..count.
export const rules = (count: number): autoseedv1.Rule[] =>
  Array.from({ length: count }, (_, i) => rule(i + 1));

export interface AutoseedServiceOptions {
  /** The stored config; default enabled. */
  config?: autoseedv1.IConfig;
  /** The rules it starts with; default rules(3). */
  rules?: autoseedv1.Rule[];
}

const errNotFound = (id: bigint) => new Error(`rule ${id} not found`);

// One node's autoseed config and rules.
export default class AutoseedService
  extends UnimplementedAutoseedFrontendService
  implements AutoseedFrontendService
{
  config: autoseedv1.Config;
  rules: autoseedv1.Rule[];
  nextId: number;

  constructor({ config, rules: initial }: AutoseedServiceOptions = {}) {
    super();
    this.config = new autoseedv1.Config(config ?? { enable: true });
    this.rules = initial ?? rules(3);
    this.nextId = Math.max(0, ...this.rules.map((r) => Number(r.id))) + 1;
  }

  private find(id: bigint): autoseedv1.Rule {
    const rule = this.rules.find((r) => r.id === id);
    if (!rule) {
      throw errNotFound(id);
    }
    return rule;
  }

  getConfig(): autoseedv1.GetConfigResponse {
    return new autoseedv1.GetConfigResponse({ config: this.config });
  }

  setConfig(req: autoseedv1.SetConfigRequest): autoseedv1.SetConfigResponse {
    this.config = new autoseedv1.Config(req.config);
    return new autoseedv1.SetConfigResponse({ config: this.config });
  }

  listRules(): autoseedv1.ListRulesResponse {
    return new autoseedv1.ListRulesResponse({ rules: this.rules });
  }

  getRule(req: autoseedv1.GetRuleRequest): autoseedv1.GetRuleResponse {
    return new autoseedv1.GetRuleResponse({ rule: this.find(req.id) });
  }

  createRule(req: autoseedv1.CreateRuleRequest): autoseedv1.CreateRuleResponse {
    const rule = new autoseedv1.Rule({ ...req.rule, id: BigInt(this.nextId++) });
    this.rules = [...this.rules, rule];
    return new autoseedv1.CreateRuleResponse({ rule });
  }

  updateRule(req: autoseedv1.UpdateRuleRequest): autoseedv1.UpdateRuleResponse {
    this.find(req.id);
    const rule = new autoseedv1.Rule({ ...req.rule, id: req.id });
    this.rules = this.rules.map((r) => (r.id === req.id ? rule : r));
    return new autoseedv1.UpdateRuleResponse({ rule });
  }

  deleteRule(req: autoseedv1.DeleteRuleRequest): autoseedv1.DeleteRuleResponse {
    this.find(req.id);
    this.rules = this.rules.filter((r) => r.id !== req.id);
    return new autoseedv1.DeleteRuleResponse();
  }
}
