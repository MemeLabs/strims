// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * A saved view's references to the event vocabulary (crates/strims-wan/vocab.toml, as vocab.ts
 * lists it), expanded into Vega-Lite: a spec names a vocabulary rather than spelling its names and
 * colours, so a phase, an episode or a reason has the document's one colour in every view.
 *
 * A reference is `{"vocab": V}` or `{"vocab": V, "only": [...]}`, V a vocabulary (`phases`,
 * `episodes` or `reasons`) or a list of them. Its names are V's in the document's order (a list's,
 * each vocabulary's in turn), those `only` lists alone where it's given.
 *
 * - A scale, `"scale": {"vocab": ...}`: its domain is the names and its range their colours; its
 *   other keys stay.
 * - A sort, `"sort": {"vocab": ...}`: the names.
 *
 * A reference anywhere else, to a vocabulary the document lacks, or whose `only` lists a name its
 * vocabularies lack, throws.
 */

import { EPISODES, PHASES, REASONS } from "./vocab";

type Spec = Record<string, unknown>;

const isSpec = (v: unknown): v is Spec => typeof v === "object" && v !== null && !Array.isArray(v);

/** A name of a vocabulary, and its colour. */
export interface Named {
  readonly name: string;
  readonly colour: string;
}

/** The vocabularies a reference names, each its names in the document's order. */
export const VOCABS: Readonly<Record<string, readonly Named[]>> = {
  phases: PHASES,
  episodes: EPISODES,
  reasons: REASONS,
};

const list = (v: unknown): v is string[] =>
  Array.isArray(v) && v.length > 0 && v.every((x) => typeof x === "string");

/**
 * The names a reference stands for, with their colours.
 *
 * @param ref - a reference: `vocab`, and `only` if given
 * @param at - where it is in the spec, for the errors
 * @returns its names in the document's order
 */
export const named = (ref: { vocab?: unknown; only?: unknown }, at: string): Named[] => {
  const vocabs = typeof ref.vocab === "string" ? [ref.vocab] : ref.vocab;
  if (!list(vocabs)) throw new Error(`${at}: vocab is a vocabulary or a list of them`);
  const all: Named[] = [];
  const seen = new Map<string, string>();
  for (const v of vocabs) {
    const names = Object.prototype.hasOwnProperty.call(VOCABS, v) ? VOCABS[v] : undefined;
    if (!names)
      throw new Error(`${at}: no vocabulary ${JSON.stringify(v)} (${Object.keys(VOCABS).join(", ")})`);
    for (const n of names) {
      const other = seen.get(n.name);
      if (other !== undefined)
        throw new Error(`${at}: ${JSON.stringify(n.name)} is a name of both ${other} and ${v}`);
      seen.set(n.name, v);
      all.push(n);
    }
  }
  if (ref.only === undefined) return all;
  if (!list(ref.only)) throw new Error(`${at}: only is a list of names`);
  for (const n of ref.only)
    if (!seen.has(n))
      throw new Error(`${at}: ${JSON.stringify(n)} is no name of ${vocabs.join(" or ")}`);
  if (new Set(ref.only).size !== ref.only.length)
    throw new Error(`${at}: only lists a name twice`);
  const only = new Set(ref.only);
  return all.filter((n) => only.has(n.name));
};

const scale = (s: Spec, at: string): Spec => {
  const { vocab, only, ...rest } = s;
  for (const k of ["domain", "range", "scheme"])
    if (k in rest) throw new Error(`${at}: a vocab scale takes no ${k}: the document gives it`);
  const names = named({ vocab, only }, at);
  if (new Set(names.map((n) => n.colour)).size !== names.length)
    throw new Error(`${at}: two of its names have one colour`);
  return { ...rest, domain: names.map((n) => n.name), range: names.map((n) => n.colour) };
};

const sort = (s: Spec, at: string): string[] => {
  const other = Object.keys(s).filter((k) => k !== "vocab" && k !== "only");
  if (other.length > 0) throw new Error(`${at}: a vocab sort takes no ${other.join(", ")}`);
  return named(s, at).map((n) => n.name);
};

const walk = (v: unknown, at: string): unknown => {
  if (Array.isArray(v)) return v.map((x, i) => walk(x, `${at}[${i}]`));
  if (!isSpec(v)) return v;
  if ("vocab" in v) throw new Error(`${at || "spec"}: a vocab reference stands for a scale or a sort`);
  const out: Spec = {};
  for (const [k, x] of Object.entries(v)) {
    const here = at ? `${at}.${k}` : k;
    if (k === "data" || k === "datum") out[k] = x;
    else if (k === "scale" && isSpec(x) && "vocab" in x) out[k] = scale(x, here);
    else if (k === "sort" && isSpec(x) && "vocab" in x) out[k] = sort(x, here);
    else out[k] = walk(x, here);
  }
  return out;
};

/**
 * `spec` with its vocabulary references expanded (the module's docs).
 *
 * @param spec - a saved view's spec
 * @returns a copy with each reference's names and colours in its place
 * @throws on a reference out of place, or to a vocabulary or name the document lacks
 */
export const expandVocab = (spec: Spec): Spec => walk(spec, "") as Spec;
