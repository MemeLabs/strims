import UAParser from "ua-parser-js";

export const enum DeviceType {
  TV = "tv",
  Portable = "portable",
  PC = "pc",
}

export const [DEVICE_TYPE, OS] = ((): [DeviceType, string] => {
  const ua = new UAParser();
  return [
    {
      "console": DeviceType.TV,
      "smarttv": DeviceType.TV,
      "mobile": DeviceType.Portable,
      "tablet": DeviceType.Portable,
      "wearable": DeviceType.Portable,
      "embedded": DeviceType.Portable,
    }[ua.getDevice().type] ?? DeviceType.PC,
    ua.getOS().name,
  ];
})();

export const IS_PWA = !window.matchMedia("(display-mode: browser)").matches;
