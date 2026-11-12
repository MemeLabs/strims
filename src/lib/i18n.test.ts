// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";
import { readFileSync, readdirSync } from "fs";
import path from "path";

import JSON5 from "json5";

const ROOT = path.resolve(__dirname, "../..");
const LOCALES = path.join(ROOT, "assets/locales");
const SRC = path.join(ROOT, "src");
const FALLBACK = "en";

type Resources = { [key: string]: string | Resources };

// A locale's keys as t() names them: nested objects joined with dots.
const keys = (lng: string): string[] => {
  const walk = (o: Resources, prefix: string): string[] =>
    Object.entries(o).flatMap(([k, v]) => {
      const key = prefix ? `${prefix}.${k}` : k;
      return typeof v === "string" ? [key] : walk(v, key);
    });
  return walk(JSON5.parse(readFileSync(path.join(LOCALES, lng, "translation.json"), "utf8")), "");
};

const sources = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) return sources(p);
    return /\.tsx?$/.test(e.name) && !e.name.endsWith(".d.ts") && p !== __filename ? [p] : [];
  });

// Keys the source builds at run time, each family with every key it can
// build. A t() call, i18nKey or <Trans> whose key isn't a literal must be
// one of these, matched by its file and its text.
const DYNAMIC: { file: string; use: RegExp; keys: () => string[] }[] = [];

interface Use {
  where: string;
  key: string;
}

interface Uses {
  keys: Use[];
  /** Uses whose key isn't a literal and no DYNAMIC family covers. */
  dynamic: Use[];
  /** useTranslation calls with a namespace or keyPrefix, which the keys don't take into account. */
  scoped: string[];
}

// A string literal's value, or null for any other expression: a template
// with a substitution, a name, a call.
const literal = (expr: string): string | null => {
  const m = /^\s*(?:"((?:\\.|[^"\\])*)"|'((?:\\.|[^'\\])*)'|`((?:\\.|[^`\\$])*)`)\s*$/.exec(expr);
  if (!m) return null;
  // As a JSON string: escapes kept but \' and \`, a bare " escaped.
  const raw = (m[1] ?? m[2] ?? m[3]).replace(/\\([\s\S])|"/g, (e, c?: string) =>
    c === undefined ? '\\"' : c === "'" || c === "`" ? c : e
  );
  return JSON.parse(`"${raw}"`) as string;
};

// t("key", ...) and i18n.t("key", ...): the first argument, up to its comma
// or the call's closing parenthesis.
const T_CALL = /\bt\(\s*((?:"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|[^,()])+)/g;

const usesOf = (files: string[]): Uses => {
  const uses: Uses = { keys: [], dynamic: [], scoped: [] };
  for (const file of files) {
    const text = readFileSync(file, "utf8");
    if (!/from "(react-)?i18next"/.test(text)) continue;
    const rel = path.relative(ROOT, file);
    const lineOf = (i: number) => `${rel}:${text.substring(0, i).split("\n").length}`;
    const add = (i: number, expr: string, use: string) => {
      const where = lineOf(i);
      const key = literal(expr);
      if (key !== null) {
        uses.keys.push({ where, key });
        return;
      }
      const family = DYNAMIC.find((d) => rel === d.file && d.use.test(use));
      if (family) {
        uses.keys.push(...family.keys().map((key) => ({ where, key })));
      } else {
        uses.dynamic.push({ where, key: use });
      }
    };
    for (const m of text.matchAll(T_CALL)) {
      add(m.index, m[1], m[0]);
    }
    for (const m of text.matchAll(/\bi18nKey=(?:\{([^}]*)\}|("(?:\\.|[^"\\])*"|'[^']*'))/g)) {
      add(m.index, m[1] ?? m[2], m[0]);
    }
    // <Trans>key</Trans>, with no i18nKey: its children are the key.
    for (const m of text.matchAll(/<Trans((?:\s[^>]*?)?)(?<!\/)>([\s\S]*?)<\/Trans>/g)) {
      if (/\bi18nKey=/.test(m[1])) continue;
      const children = m[2].trim();
      add(m.index, /^[^<>{}]+$/.test(children) ? JSON.stringify(children) : children, m[0]);
    }
    for (const m of text.matchAll(/\buseTranslation\(([^)]*)\)/g)) {
      if (m[1].trim()) uses.scoped.push(`${lineOf(m.index)}: ${m[0]}`);
    }
  }
  return uses;
};

const en = new Set(keys(FALLBACK));
const others = readdirSync(LOCALES).filter((lng) => lng !== FALLBACK);
const uses = usesOf(sources(SRC));

describe("i18n", () => {
  it("finds the source's keys", () => {
    assert.ok(uses.keys.length > 100, `only ${uses.keys.length} key uses found`);
  });

  it(`${FALLBACK} has every key the source uses`, () => {
    const missing = uses.keys.filter(({ key }) => !en.has(key));
    assert.deepStrictEqual(
      missing.map(({ where, key }) => `${where}: ${key}`),
      [],
      `keys ${FALLBACK} lacks`
    );
  });

  it("knows every key the source builds at run time", () => {
    assert.deepStrictEqual(
      uses.dynamic.map(({ where, key }) => `${where}: ${key}`),
      [],
      "keys that aren't literals: list each family's keys in DYNAMIC"
    );
    assert.deepStrictEqual(uses.scoped, [], "useTranslation with a namespace or keyPrefix");
  });

  for (const lng of others) {
    describe(lng, () => {
      const own = keys(lng);
      const translated = own.filter((k) => en.has(k)).length;

      it(`has no keys ${FALLBACK} lacks`, () => {
        assert.deepStrictEqual(
          own.filter((k) => !en.has(k)),
          [],
          `stale keys in ${lng}`
        );
      });

      // Coverage, in the title: a key a translation lacks falls back to en,
      // so a gap is translation work, not a failure.
      it(`translates ${translated} of ${en.size} keys (${en.size - translated} missing)`, () => {
        assert.ok(translated > 0, `${lng} translates nothing`);
      });
    });
  }
});
