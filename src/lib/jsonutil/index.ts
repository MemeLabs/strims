import { Base64 } from "js-base64";

export default {
  stringify: (v) =>
    JSON.stringify(
      v,
      (key, value) => {
        if (typeof value === "bigint") return value.toString();
        if (value instanceof Uint8Array) return Base64.fromUint8Array(value);
        return value;
      },
      2
    ),
};
