// package: 
// file: rpc.proto

import * as jspb from "google-protobuf";
import * as google_protobuf_any_pb from "google-protobuf/google/protobuf/any_pb";

export class Call extends jspb.Message {
  getId(): number;
  setId(value: number): void;

  getParentId(): number;
  setParentId(value: number): void;

  getMethod(): string;
  setMethod(value: string): void;

  hasArgument(): boolean;
  clearArgument(): void;
  getArgument(): google_protobuf_any_pb.Any | undefined;
  setArgument(value?: google_protobuf_any_pb.Any): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Call.AsObject;
  static toObject(includeInstance: boolean, msg: Call): Call.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: Call, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Call;
  static deserializeBinaryFromReader(message: Call, reader: jspb.BinaryReader): Call;
}

export namespace Call {
  export type AsObject = {
    id: number,
    parentId: number,
    method: string,
    argument?: google_protobuf_any_pb.Any.AsObject,
  }
}

export class Error extends jspb.Message {
  getMessage(): string;
  setMessage(value: string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Error.AsObject;
  static toObject(includeInstance: boolean, msg: Error): Error.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: Error, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Error;
  static deserializeBinaryFromReader(message: Error, reader: jspb.BinaryReader): Error;
}

export namespace Error {
  export type AsObject = {
    message: string,
  }
}

export class Undefined extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Undefined.AsObject;
  static toObject(includeInstance: boolean, msg: Undefined): Undefined.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: Undefined, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Undefined;
  static deserializeBinaryFromReader(message: Undefined, reader: jspb.BinaryReader): Undefined;
}

export namespace Undefined {
  export type AsObject = {
  }
}

export class Close extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Close.AsObject;
  static toObject(includeInstance: boolean, msg: Close): Close.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: Close, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Close;
  static deserializeBinaryFromReader(message: Close, reader: jspb.BinaryReader): Close;
}

export namespace Close {
  export type AsObject = {
  }
}

