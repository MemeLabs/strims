// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";
import { readFileSync, readdirSync } from "fs";
import path from "path";

import { expandVocab } from "./domains";
import { CURSOR, CURSOR_MARKS, hovered } from "./hover";
import { config } from "./theme";

type Spec = Record<string, unknown>;

const VIEWS = path.resolve(__dirname, "../../../../hack/console/views");

const x = { field: "t", type: "quantitative" };
const line = { mark: "line", encoding: { x, y: { field: "v", type: "quantitative" } } };
const cells = { mark: { type: "rect", tooltip: true }, encoding: { x, y: { field: "who" } } };

const layers = (s: unknown): Spec[] => (s as { layer: Spec[] }).layer;
const names = (s: unknown): unknown[] => layers(s).map((l) => l.name);
const params = (s: Spec): Spec[] => s.params as Spec[];

describe("hovered", () => {
  it("leaves a spec without a time field as it was, but its hover keys", () => {
    const spec = { vconcat: [{ ...line, hover: { tooltip: [{ field: "v" }] } }] };
    assert.deepStrictEqual(hovered(spec), { vconcat: [line] });
    assert.deepStrictEqual(hovered(line), line);
  });

  it("covers a time series with a rule whose tooltip is its rows' at the nearest x", () => {
    const tooltip = [{ field: "t" }, { field: "a" }];
    const pivot = { pivot: "series", value: "v", groupby: ["t"] };
    const spec = {
      hover: { x: "t" },
      vconcat: [
        {
          title: "lag",
          height: 100,
          transform: [{ filter: "datum.track === 'lag'" }],
          hover: { tooltip, transform: [pivot] },
          ...line,
        },
      ],
    };
    const before = JSON.stringify(spec);
    const out = hovered(spec);
    assert.strictEqual(JSON.stringify(spec), before, "the spec given is unchanged");
    assert.strictEqual(out.hover, undefined);
    const panel = (out.vconcat as Spec[])[0];
    assert.deepStrictEqual(
      Object.keys(panel).sort(),
      ["height", "layer", "title"],
      "a unit's own keys go to its layer"
    );
    assert.deepStrictEqual(names(panel), [undefined, "cursor_0", "hover_0"]);
    const [main, cursor, hover] = layers(panel);
    assert.deepStrictEqual(main, { ...line, transform: [{ filter: "datum.track === 'lag'" }] });
    assert.deepStrictEqual(hover.transform, [{ filter: "datum.track === 'lag'" }, pivot]);
    assert.deepStrictEqual(hover.encoding, { x, tooltip });
    assert.deepStrictEqual(hover.mark, { type: "rule", opacity: 0 });
    // The cursor: every row's x once, shown where selected, and the grid's ends unseen.
    assert.deepStrictEqual((cursor.encoding as Spec).opacity, {
      condition: { test: { param: CURSOR, empty: false }, value: 1 },
      value: 0,
    });
    assert.deepStrictEqual(params(out), [
      {
        name: CURSOR,
        select: {
          type: "point",
          encodings: ["x"],
          nearest: true,
          on: "pointerover",
          clear: "pointerout",
          toggle: false,
        },
        views: ["hover_0"],
      },
    ]);
  });

  it("lets a track's marks move the cursor and keep their tooltips", () => {
    const spec = {
      hover: { x: "t" },
      vconcat: [
        { hover: { tooltip: [{ field: "v" }] }, ...line },
        { hover: {}, transform: [{ filter: "datum.track === 'heat'" }], ...cells },
      ],
    };
    const out = hovered(spec);
    const track = (out.vconcat as Spec[])[1];
    assert.deepStrictEqual(names(track), ["hover_1", "marks_1", "cursor_1"]);
    const [gaps, marks] = layers(track);
    assert.deepStrictEqual(marks, {
      ...cells,
      transform: [{ filter: "datum.track === 'heat'" }],
      name: "marks_1",
    });
    assert.deepStrictEqual(gaps.encoding, { x });
    assert.deepStrictEqual(
      params(out).map((p) => [p.name, p.views]),
      [
        [CURSOR, ["hover_0", "hover_1"]],
        [CURSOR_MARKS, ["marks_1"]],
      ]
    );
    assert.deepStrictEqual((params(out)[1].select as Spec).fields, ["t"]);
    const shown = {
      or: [
        { param: CURSOR, empty: false },
        { param: CURSOR_MARKS, empty: false },
      ],
    };
    for (const p of out.vconcat as Spec[]) {
      const cursor = layers(p).find(
        (l) => typeof l.name === "string" && l.name.startsWith("cursor_")
      );
      assert.ok(cursor);
      assert.deepStrictEqual(((cursor.encoding as Spec).opacity as Spec).condition, {
        test: shown,
        value: 1,
      });
    }
  });

  it("moves a layer panel's transforms to its layers of its data", () => {
    const rule = { data: { values: [{ d: 1 }] }, mark: "rule", encoding: { y: { field: "d" } } };
    const spec = {
      hover: { x: "t" },
      transform: [{ filter: "datum.track === 'lag'" }],
      layer: [line, rule],
    };
    const out = hovered({ ...spec, hover: { x: "t", tooltip: [{ field: "v" }] } });
    assert.deepStrictEqual(names(out), [undefined, undefined, "cursor_0", "hover_0"]);
    const [main, other, , hover] = layers(out);
    assert.deepStrictEqual(main.transform, [{ filter: "datum.track === 'lag'" }]);
    assert.deepStrictEqual(other, rule);
    assert.deepStrictEqual(hover.transform, [{ filter: "datum.track === 'lag'" }]);
    assert.strictEqual(out.transform, undefined);
    assert.deepStrictEqual(
      params(out).map((p) => p.name),
      [CURSOR]
    );
  });

  it("expands every saved view, its vocabulary then its hover keys, into a spec Vega-Lite compiles without a warning", async () => {
    const { compile } = await import("vega-lite");
    const files = readdirSync(VIEWS).filter((f) => f.endsWith(".vl.json"));
    assert.ok(files.length > 0);
    let withHover = 0;
    for (const f of files) {
      const spec = JSON.parse(readFileSync(path.join(VIEWS, f), "utf8")) as Spec;
      if (spec.hover !== undefined) withHover++;
      const warnings: string[] = [];
      const logger = {
        level: () => logger,
        info: () => logger,
        debug: () => logger,
        warn: (...a: unknown[]) => (warnings.push(a.join(" ")), logger),
        error: (...a: unknown[]) => (warnings.push(a.join(" ")), logger),
      };
      const out = hovered(expandVocab(spec));
      assert.ok(!JSON.stringify(out).includes('"hover":{'), `${f}: a hover key is left`);
      assert.ok(!JSON.stringify(out).includes('"vocab":'), `${f}: a vocab reference is left`);
      compile({ ...out, data: { values: [] }, config } as never, { logger });
      assert.deepStrictEqual(warnings, [], f);
    }
    assert.ok(withHover >= 4, "the time series views have hover cursors");
  });
});
