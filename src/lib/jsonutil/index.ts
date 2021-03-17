import { Base64 } from "js-base64";

export default {
  stringify: (v: any): string =>
    JSON.stringify(
      v,
      (key: string, value: any): any => {
        if (typeof value === "bigint") return value.toString();
        if (value instanceof Uint8Array) return Base64.fromUint8Array(value);
        // eslint-disable-next-line
        return value;
      },
      2
    ),
};
