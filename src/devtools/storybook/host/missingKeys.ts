// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Reports each translation key a story renders that en, the fallback
// language every other falls back to, lacks: one console.error per key,
// which the recorder fails the story on. A key looked up before en's
// resources arrive (a Trans renders without suspending) is checked once
// they do, so only a key en really lacks is reported.

import i18n from "i18next";

const FALLBACK = "en";

const reported = new Set<string>();
const failed = new Set<string>();
let pending: { ns: string; key: string }[] = [];

const report = (ns: string, key: string, why = "") => {
  const id = `${ns}:${key}`;
  if (!reported.has(id)) {
    reported.add(id);
    const name = [i18n.options.defaultNS].flat().includes(ns) ? key : `${ns}:${key}`;
    // eslint-disable-next-line no-console
    console.error(`i18n: missing key "${name}" in ${FALLBACK}${why}`);
  }
};

// Whether a lookup can be settled: en's namespace is in, or failed to load.
const settle = (ns: string, key: string): boolean => {
  if (i18n.hasResourceBundle(FALLBACK, ns)) {
    if (!i18n.exists(key, { ns, lng: FALLBACK })) {
      report(ns, key);
    }
    return true;
  }
  if (failed.has(ns)) {
    report(ns, key, ` (its resources failed to load)`);
    return true;
  }
  return false;
};

const flush = () => {
  pending = pending.filter(({ ns, key }) => !settle(ns, key));
};

let installed = false;

/** Reports the keys this document's stories render that en lacks. */
export const reportMissingKeys = (): void => {
  if (installed) {
    return;
  }
  installed = true;
  // A missed lookup reaches missingKeyHandler only with saveMissing on; the
  // handler stands in for the backend's saving, so nothing is sent anywhere.
  // A pluralized lookup reports its own key, not its plural suffixes.
  i18n.options.saveMissing = true;
  i18n.options.saveMissingPlurals = false;
  i18n.options.missingKeyHandler = (_lngs, ns, key) => {
    if (!settle(ns, key)) {
      pending.push({ ns, key });
    }
  };
  i18n.on("loaded", flush);
  i18n.on("failedLoading", (lng: string, ns: string) => {
    if (lng === FALLBACK) {
      failed.add(ns);
    }
    flush();
  });
};
