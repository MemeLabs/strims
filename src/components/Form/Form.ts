import "./Form.scss";

import { FieldValues, UseControllerProps } from "react-hook-form";

export type CompatibleFieldPath<T extends FieldValues, V> = {
  [K in keyof T]: T[K] extends V ? K : never;
}[keyof T];

export type CompatibleUseControllerProps<T, V> = UseControllerProps<T> & {
  name: CompatibleFieldPath<T, V>;
};

export const isRequired = <T extends FieldValues>(rules: UseControllerProps<T>["rules"]) => {
  switch (typeof rules?.required) {
    case "undefined":
      return false;
    case "boolean":
      return rules.required;
    case "string":
      return true;
    default:
      return rules?.required?.value;
  }
};
