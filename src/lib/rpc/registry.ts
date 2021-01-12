import { Any } from "../../apis/google/protobuf/any";

// maintains a mapping of type names to classes that survives minification to
// allow encoding and decoding Any fields.

const messageTypes: Map<string, any> = new Map();
const messageTypeNames: Map<any, string> = new Map();

export const registerType = (name: string, type: any) => {
  messageTypes.set(name, type);
  messageTypeNames.set(type, name);
};

export const anyValueType = (msg: Any): any => {
  const nameIndex = msg.typeUrl.lastIndexOf("/") + 1;
  return messageTypes.get(msg.typeUrl.substr(nameIndex));
};

export const typeName = (type: any): string => {
  return messageTypeNames.get(type);
};
