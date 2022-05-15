// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "react-i18next";

import en from "../../assets/locales/en/translation.json";

declare module "react-i18next" {
  interface CustomTypeOptions {
    resources: {
      translation: typeof en;
    };
  }
}
