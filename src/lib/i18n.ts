import "../../assets/locales/en/translation.json";

import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import Backend from "i18next-http-backend";
import { initReactI18next } from "react-i18next";

void i18n
  .use(Backend)
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: "en",
    supportedLngs: ["en"],
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
