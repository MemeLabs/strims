import * as pb from "./pb";

// maintains a mapping of type names to classes that survives minification to
// allow encoding and decoding Any fields.

const messageTypes: Map<string, any> = new Map();
const messageTypeNames: Map<any, string> = new Map();

export const registerType = (name: string, type: any) => {
  messageTypes.set(name, type);
  messageTypeNames.set(type, name);
};

export const anyValueType = (msg: pb.google.protobuf.IAny): protobuf.Type => {
  const nameIndex = msg.type_url.lastIndexOf("/") + 1;
  return messageTypes.get(msg.type_url.substr(nameIndex));
};

export const typeName = (type: any): string => {
  return messageTypeNames.get(type);
};
