// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { useMemo, useState } from "react";
import { Link, useLocation } from "react-router-dom";

import { FileStatus, IndexData, IndexFile } from "./catalog";
import { jsonScript } from "./ErrorList";

type Filter = FileStatus | "all";

const FILTERS: Filter[] = ["all", "missing", "covered", "skipped"];

const dirname = (path: string) => path.substring(0, path.lastIndexOf("/"));
const basename = (path: string) => path.substring(path.lastIndexOf("/") + 1);

const FileRow: React.FC<{ file: IndexFile; search: string }> = ({ file, search }) => (
  <li className={clsx("storybook_catalog__file", `storybook_catalog__file--${file.status}`)}>
    <span className="storybook_catalog__status">{file.status}</span>
    <span className="storybook_catalog__name">{basename(file.path)}</span>
    {file.stories?.map((route) => (
      <Link key={route} className="storybook_catalog__story" to={{ pathname: route, search }}>
        {decodeURIComponent(route.substring(route.lastIndexOf("/") + 1))}
      </Link>
    ))}
    {file.reason && (
      <span className="storybook_catalog__reason">
        {file.reason} <em>({file.skippedBy})</em>
      </span>
    )}
  </li>
);

interface CatalogPageProps {
  /** The index its frame built (null while it builds), or why it failed. */
  data: IndexData | null;
  error: string | null;
  notFound?: string;
}

// The catalog of UI files (src/components, pages and watch), each covered by
// stories, skipped with a reason, or missing; and, in #storybook-index, the
// same with every story's route as JSON (hack/storybook/check.py reads the
// index frame's).
const CatalogPage: React.FC<CatalogPageProps> = ({ data, error, notFound }) => {
  const { search } = useLocation();
  const [filter, setFilter] = useState<Filter>("all");

  const groups = useMemo(() => {
    const groups = new Map<string, IndexFile[]>();
    for (const file of data?.files ?? []) {
      if (filter === "all" || file.status === filter) {
        const dir = dirname(file.path);
        groups.set(dir, [...(groups.get(dir) ?? []), file]);
      }
    }
    return Array.from(groups);
  }, [data, filter]);

  // Story modules whose path mirrors no catalog file.
  const others = useMemo(() => {
    const files = new Set(data?.files.map((f) => f.path.replace(/\.tsx$/, "")));
    return data?.stories.filter((s) => !files.has(s.module)) ?? [];
  }, [data]);

  if (error) {
    return <pre className="storybook_catalog__error">{error}</pre>;
  }
  if (!data) {
    return <p className="loading_message">loading the stories</p>;
  }

  const { counts } = data;
  const pct = counts.total
    ? Math.round(((counts.covered + counts.skipped) / counts.total) * 100)
    : 100;

  return (
    <div className="storybook_catalog">
      <script
        type="application/json"
        id="storybook-index"
        dangerouslySetInnerHTML={jsonScript(data)}
      />
      {notFound && (
        <p className="storybook_catalog__not_found">
          No story at <code>{notFound}</code>.
        </p>
      )}
      <h2>Catalog</h2>
      <p className="storybook_catalog__counts">
        {counts.covered} covered, {counts.skipped} skipped, {counts.missing} missing of{" "}
        {counts.total} files ({pct}% accounted for); {data.stories.length} stories in{" "}
        {data.modules.length} modules
      </p>
      {data.modules
        .filter((m) => m.error)
        .map((m) => (
          <pre key={m.module} className="storybook_catalog__error">
            {m.module}: {m.error}
          </pre>
        ))}
      {data.stale.length > 0 && (
        <ul className="storybook_catalog__stale">
          {data.stale.map((s, i) => (
            <li key={i}>
              {s.module}:{" "}
              {s.kind === "skips-covered"
                ? "skips a covered file"
                : `${s.kind} a file outside the catalog`}{" "}
              <code>{s.path}</code>
            </li>
          ))}
        </ul>
      )}
      <div className="storybook_catalog__filters">
        {FILTERS.map((f) => (
          <button
            key={f}
            className={clsx("storybook_catalog__filter", {
              "storybook_catalog__filter--active": f === filter,
            })}
            onClick={() => setFilter(f)}
          >
            {f} {f === "all" ? counts.total : counts[f]}
          </button>
        ))}
      </div>
      {groups.map(([dir, files]) => (
        <section key={dir} className="storybook_catalog__group">
          <h3>
            {dir}{" "}
            <span className="storybook_catalog__group_counts">
              {files.filter((f) => f.status !== "missing").length}/{files.length}
            </span>
          </h3>
          <ul>
            {files.map((file) => (
              <FileRow key={file.path} file={file} search={search} />
            ))}
          </ul>
        </section>
      ))}
      {others.length > 0 && (
        <section className="storybook_catalog__group">
          <h3>Other stories</h3>
          <ul>
            {others.map((s) => (
              <li key={s.route} className="storybook_catalog__file">
                <Link to={{ pathname: s.route, search }}>
                  {s.module}/{s.name}
                </Link>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
};

export default CatalogPage;
