// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../assets/locales/en/translation.json";

import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import Backend from "i18next-http-backend";
import ICU from "i18next-icu";
import { initReactI18next } from "react-i18next";

void i18n
  .use(ICU)
  .use(Backend)
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: "en",
    supportedLngs: I18N_LANG,
    load: "languageOnly",
    debug: !IS_PRODUCTION,

    interpolation: {
      escapeValue: false,
    },

    backend: {
      queryStringParams: { "_v": GIT_HASH.substring(0, 7) },
    },
  });

if (import.meta.webpackHot) {
  import.meta.webpackHot.accept(
    ["../../assets/locales/en/translation.json"],
    () => void i18n.reloadResources().then(() => i18n.changeLanguage("en"))
  );
}
