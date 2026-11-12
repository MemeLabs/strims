// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "i18next";

// No resources: i18next's key types need strictNullChecks, which is off
// here (`undefined extends string` then holds, and every key would need its
// namespace prefix), so keys are strings; src/lib/i18n.test.ts checks them
// against en.
declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
  }
}
