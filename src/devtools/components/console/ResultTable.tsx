// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import type { Json, QueryResult } from "./api";

const NUMERIC = /^(TINYINT|SMALLINT|INTEGER|BIGINT|HUGEINT|U\w*INT|FLOAT|DOUBLE|DECIMAL)/;

/** A value as a cell shows it. */
export const cellText = (v: Json): string => {
  if (v === null) return "";
  if (typeof v === "number") {
    return Number.isInteger(v) ? String(v) : v.toPrecision(6).replace(/\.?0+$/, "");
  }
  if (typeof v === "string") return v;
  // A list of strings, a line each.
  if (Array.isArray(v) && v.every((x) => typeof x === "string")) return v.join("\n");
  return JSON.stringify(v);
};

export interface ResultTableProps {
  result: QueryResult;
  /** The most rows it shows. */
  limit?: number;
  /** A cell before each row's, given the row. */
  actions?: (row: Json[]) => React.ReactNode;
}

/** A query's rows as a table, with its columns' types in the header. */
const ResultTable: React.FC<ResultTableProps> = ({ result, limit = 1000, actions }) => {
  const numeric = result.columns.map((c) => NUMERIC.test(c.type));
  const rows = result.rows.slice(0, limit);
  return (
    <div className="console__table-box">
      <table className="console__table">
        <thead>
          <tr>
            {actions && <th />}
            {result.columns.map((c) => (
              <th key={c.name} title={c.type}>
                {c.name}
                <span className="console__type">{c.type}</span>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i}>
              {actions && <td className="console__actions">{actions(row)}</td>}
              {row.map((v, j) => {
                const text = cellText(v);
                return (
                  <td
                    key={j}
                    className={numeric[j] ? "console__num" : undefined}
                    title={text.length > 80 ? text : undefined}
                  >
                    {text}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
      <div className="console__meta">
        {result.rows.length} rows{result.truncated ? " (truncated at the row cap)" : ""}
        {result.rows.length > limit ? `, the first ${limit} shown` : ""} · {result.elapsed_ms} ms
      </div>
    </div>
  );
};

export default ResultTable;
