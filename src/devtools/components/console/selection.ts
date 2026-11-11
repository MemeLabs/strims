// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";

import { runSql } from "./api";
import type { Json, Param, SavedView } from "./api";

/** What the console's views are about: an experiment, and a job of it. Kept in the URL. */
export interface Selection {
  exp: string;
  cell: string;
  arm: string;
  arm2: string;
  seed: string;
  delay: string;
  min_stall_ms: string;
  align: string;
  t: string;
  search: string;
}

export const DEFAULTS: Selection = {
  exp: "",
  cell: "",
  arm: "",
  arm2: "",
  seed: "",
  delay: "1250",
  min_stall_ms: "50",
  align: "start",
  t: "30",
  search: "",
};

export const KEYS = Object.keys(DEFAULTS) as (keyof Selection)[];

/** The address's selection on the graph (sel.ts), last in its query. */
export const SEL = "sel";
/** The workbench's statement, first in its query: a saved view's or its own. */
export const STATEMENT = ["view", "sql"];
/** The workbench's auto-chart choices, after its `$name`s. */
export const CHART = ["mark", "x", "y", "color", "logy"];

export const isKey = (k: string): k is keyof Selection => (KEYS as string[]).includes(k);

/**
 * `query` in the order the console's addresses take (hack/simlab/views.py's docs): the workbench's
 * statement, the selection's keys in KEYS' order, any other key in its order (the workbench's
 * `$name`s), the auto-chart's choices, then `sel`; each key once, and none whose value is empty or
 * its key's default.
 *
 * @param query - an address's query
 * @returns it in that order
 */
export const ordered = (query: URLSearchParams): URLSearchParams => {
  const first = new Map<string, string>();
  for (const [k, v] of query) if (!first.has(k)) first.set(k, v);
  const out = new URLSearchParams();
  const put = (k: string) => {
    const v = first.get(k);
    first.delete(k);
    if (v !== undefined && v !== "" && !(isKey(k) && v === DEFAULTS[k])) out.append(k, v);
  };
  const last = [...CHART, SEL];
  [...STATEMENT, ...KEYS].forEach(put);
  [...first.keys()].filter((k) => !last.includes(k)).forEach(put);
  last.forEach(put);
  return out;
};

/**
 * What of an address's query another view takes: the selection's keys alone.
 *
 * @param search - an address's query
 * @returns its selection's keys, in order
 */
export const carried = (search: string | URLSearchParams): URLSearchParams =>
  ordered(new URLSearchParams([...new URLSearchParams(search)].filter(([k]) => isKey(k))));

/**
 * The selection a query holds: each key's value, else its default.
 *
 * @param query - an address's query
 * @returns its selection
 */
export const selectionOf = (query: URLSearchParams): Selection =>
  Object.fromEntries(KEYS.map((k) => [k, query.get(k) ?? DEFAULTS[k]])) as unknown as Selection;

/**
 * The selection in the URL's search params, and a setter that merges into it.
 *
 * @returns the selection and its setter
 */
export const useSelection = (): [Selection, (s: Partial<Selection>) => void] => {
  const [params, setParams] = useSearchParams();
  const sel = selectionOf(params);
  const set = useCallback(
    (s: Partial<Selection>) =>
      setParams(
        (old) => {
          const next = new URLSearchParams(old);
          for (const [k, v] of Object.entries(s)) {
            if (v === undefined || v === "" || v === DEFAULTS[k as keyof Selection]) next.delete(k);
            else next.set(k, v);
          }
          return ordered(next);
        },
        { replace: true }
      ),
    [setParams]
  );
  return [sel, set];
};

/**
 * The address's selection on the graph as its text (sel.ts reads it), and a setter that writes it,
 * or with null or "" drops it.
 *
 * @returns its text ("" for none) and its setter
 */
export const useSel = (): [string, (text: string | null) => void] => {
  const [params, setParams] = useSearchParams();
  const set = useCallback(
    (text: string | null) =>
      setParams(
        (old) => {
          const next = new URLSearchParams(old);
          if (text) next.set(SEL, text);
          else next.delete(SEL);
          return ordered(next);
        },
        { replace: true }
      ),
    [setParams]
  );
  return [params.get(SEL) ?? "", set];
};

const NUMBER_KINDS = new Set(["seed", "number"]);

/**
 * A saved view's parameters from the selection: each by its name, else its kind (`exp`, `cell`,
 * ...), else its default.
 *
 * @param view - the saved view
 * @param sel - the selection
 * @returns its params, and the names of the ones missing
 */
export const viewParams = (
  view: Pick<SavedView, "params">,
  sel: Selection
): [Record<string, Param>, string[]] => {
  const params: Record<string, Param> = {};
  const missing: string[] = [];
  for (const p of view.params) {
    const raw =
      (sel as unknown as Record<string, string>)[p.name] ??
      (sel as unknown as Record<string, string>)[p.kind];
    let v: Param = raw !== undefined && raw !== "" ? raw : p.default ?? null;
    if (typeof v === "string" && NUMBER_KINDS.has(p.kind)) v = Number(v);
    if (v === null || (typeof v === "number" && Number.isNaN(v))) missing.push(p.label ?? p.name);
    else params[p.name] = v;
  }
  return [params, missing];
};

export interface Options {
  cells: string[];
  arms: string[];
  /** The seeds of the selected cell and arm, with whether each collapsed. */
  seeds: { seed: number; collapsed: boolean | null }[];
}

/**
 * The pickers' options for the selection's experiment, cell and arm.
 *
 * @param sel - the selection
 * @returns the cells, arms and seeds to pick from
 */
export const useOptions = (sel: Selection): Options => {
  const [cells, setCells] = useState<string[]>([]);
  const [arms, setArms] = useState<string[]>([]);
  const [seeds, setSeeds] = useState<Options["seeds"]>([]);
  useEffect(() => {
    if (!sel.exp) return;
    const params = { exp: sel.exp };
    runSql(
      "SELECT list(DISTINCT cell ORDER BY cell) AS cells, list(DISTINCT arm_label ORDER BY arm_label) AS arms " +
        "FROM experiment_jobs WHERE exp_id = $exp",
      params
    ).then(
      (r) => {
        const [c, a] = (r.rows[0] ?? [[], []]) as [Json, Json];
        setCells((c as string[] | null) ?? []);
        setArms((a as string[] | null) ?? []);
      },
      () => {
        setCells([]);
        setArms([]);
      }
    );
  }, [sel.exp]);
  useEffect(() => {
    if (!sel.exp || !sel.cell || !sel.arm) {
      setSeeds([]);
      return;
    }
    runSql(
      "SELECT e.seed, v.collapsed FROM experiment_jobs e LEFT JOIN verdicts v USING (k, src) " +
        "WHERE e.exp_id = $exp AND e.cell = $cell AND e.arm_label = $arm ORDER BY e.seed",
      { exp: sel.exp, cell: sel.cell, arm: sel.arm }
    ).then(
      (r) =>
        setSeeds(r.rows.map(([s, c]) => ({ seed: s as number, collapsed: c as boolean | null }))),
      () => setSeeds([])
    );
  }, [sel.exp, sel.cell, sel.arm]);
  return { cells, arms, seeds };
};
