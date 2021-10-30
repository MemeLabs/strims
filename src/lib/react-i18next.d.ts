import "react-i18next";

import en from "../../assets/locales/en/translation.json";

declare module "react-i18next" {
  interface CustomTypeOptions {
    resources: {
      translation: typeof en;
    };
  }
}
