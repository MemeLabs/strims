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

const DEFAULTS: Selection = {
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

/**
 * The selection in the URL's search params, and a setter that merges into it.
 *
 * @returns the selection and its setter
 */
export const useSelection = (): [Selection, (s: Partial<Selection>) => void] => {
  const [params, setParams] = useSearchParams();
  const sel = Object.fromEntries(
    KEYS.map((k) => [k, params.get(k) ?? DEFAULTS[k]])
  ) as unknown as Selection;
  const set = useCallback(
    (s: Partial<Selection>) =>
      setParams(
        (old) => {
          const next = new URLSearchParams(old);
          for (const [k, v] of Object.entries(s)) {
            if (v === undefined || v === "" || v === DEFAULTS[k as keyof Selection]) next.delete(k);
            else next.set(k, v);
          }
          return next;
        },
        { replace: true }
      ),
    [setParams]
  );
  return [sel, set];
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
