// package: 
// file: api.proto

import * as jspb from "google-protobuf";

export class JoinSwarmRequest extends jspb.Message {
  getSwarmuri(): string;
  setSwarmuri(value: string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): JoinSwarmRequest.AsObject;
  static toObject(includeInstance: boolean, msg: JoinSwarmRequest): JoinSwarmRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: JoinSwarmRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): JoinSwarmRequest;
  static deserializeBinaryFromReader(message: JoinSwarmRequest, reader: jspb.BinaryReader): JoinSwarmRequest;
}

export namespace JoinSwarmRequest {
  export type AsObject = {
    swarmuri: string,
  }
}

export class JoinSwarmResponse extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): JoinSwarmResponse.AsObject;
  static toObject(includeInstance: boolean, msg: JoinSwarmResponse): JoinSwarmResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: JoinSwarmResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): JoinSwarmResponse;
  static deserializeBinaryFromReader(message: JoinSwarmResponse, reader: jspb.BinaryReader): JoinSwarmResponse;
}

export namespace JoinSwarmResponse {
  export type AsObject = {
  }
}

export class LeaveSwarmRequest extends jspb.Message {
  getSwarmuri(): string;
  setSwarmuri(value: string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): LeaveSwarmRequest.AsObject;
  static toObject(includeInstance: boolean, msg: LeaveSwarmRequest): LeaveSwarmRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: LeaveSwarmRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): LeaveSwarmRequest;
  static deserializeBinaryFromReader(message: LeaveSwarmRequest, reader: jspb.BinaryReader): LeaveSwarmRequest;
}

export namespace LeaveSwarmRequest {
  export type AsObject = {
    swarmuri: string,
  }
}

export class LeaveSwarmResponse extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): LeaveSwarmResponse.AsObject;
  static toObject(includeInstance: boolean, msg: LeaveSwarmResponse): LeaveSwarmResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: LeaveSwarmResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): LeaveSwarmResponse;
  static deserializeBinaryFromReader(message: LeaveSwarmResponse, reader: jspb.BinaryReader): LeaveSwarmResponse;
}

export namespace LeaveSwarmResponse {
  export type AsObject = {
  }
}

export class GetIngressStreamsRequest extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIngressStreamsRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIngressStreamsRequest): GetIngressStreamsRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIngressStreamsRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIngressStreamsRequest;
  static deserializeBinaryFromReader(message: GetIngressStreamsRequest, reader: jspb.BinaryReader): GetIngressStreamsRequest;
}

export namespace GetIngressStreamsRequest {
  export type AsObject = {
  }
}

export class GetIngressStreamsResponse extends jspb.Message {
  getSwarmuri(): string;
  setSwarmuri(value: string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIngressStreamsResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIngressStreamsResponse): GetIngressStreamsResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIngressStreamsResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIngressStreamsResponse;
  static deserializeBinaryFromReader(message: GetIngressStreamsResponse, reader: jspb.BinaryReader): GetIngressStreamsResponse;
}

export namespace GetIngressStreamsResponse {
  export type AsObject = {
    swarmuri: string,
  }
}

export class StartHLSIngressRequest extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): StartHLSIngressRequest.AsObject;
  static toObject(includeInstance: boolean, msg: StartHLSIngressRequest): StartHLSIngressRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: StartHLSIngressRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): StartHLSIngressRequest;
  static deserializeBinaryFromReader(message: StartHLSIngressRequest, reader: jspb.BinaryReader): StartHLSIngressRequest;
}

export namespace StartHLSIngressRequest {
  export type AsObject = {
  }
}

export class StartHLSIngressResponse extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): StartHLSIngressResponse.AsObject;
  static toObject(includeInstance: boolean, msg: StartHLSIngressResponse): StartHLSIngressResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: StartHLSIngressResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): StartHLSIngressResponse;
  static deserializeBinaryFromReader(message: StartHLSIngressResponse, reader: jspb.BinaryReader): StartHLSIngressResponse;
}

export namespace StartHLSIngressResponse {
  export type AsObject = {
  }
}

export class StartHLSEgressRequest extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): StartHLSEgressRequest.AsObject;
  static toObject(includeInstance: boolean, msg: StartHLSEgressRequest): StartHLSEgressRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: StartHLSEgressRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): StartHLSEgressRequest;
  static deserializeBinaryFromReader(message: StartHLSEgressRequest, reader: jspb.BinaryReader): StartHLSEgressRequest;
}

export namespace StartHLSEgressRequest {
  export type AsObject = {
  }
}

export class StartHLSEgressResponse extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): StartHLSEgressResponse.AsObject;
  static toObject(includeInstance: boolean, msg: StartHLSEgressResponse): StartHLSEgressResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: StartHLSEgressResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): StartHLSEgressResponse;
  static deserializeBinaryFromReader(message: StartHLSEgressResponse, reader: jspb.BinaryReader): StartHLSEgressResponse;
}

export namespace StartHLSEgressResponse {
  export type AsObject = {
  }
}

export class StopHLSEgressRequest extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): StopHLSEgressRequest.AsObject;
  static toObject(includeInstance: boolean, msg: StopHLSEgressRequest): StopHLSEgressRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: StopHLSEgressRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): StopHLSEgressRequest;
  static deserializeBinaryFromReader(message: StopHLSEgressRequest, reader: jspb.BinaryReader): StopHLSEgressRequest;
}

export namespace StopHLSEgressRequest {
  export type AsObject = {
  }
}

export class StopHLSEgressResponse extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): StopHLSEgressResponse.AsObject;
  static toObject(includeInstance: boolean, msg: StopHLSEgressResponse): StopHLSEgressResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: StopHLSEgressResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): StopHLSEgressResponse;
  static deserializeBinaryFromReader(message: StopHLSEgressResponse, reader: jspb.BinaryReader): StopHLSEgressResponse;
}

export namespace StopHLSEgressResponse {
  export type AsObject = {
  }
}

