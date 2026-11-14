import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  UNIMPLEMENTED = 1000,
  METHOD_NOT_FOUND = 1001,
  REQUEST_DECODE_FAILED = 1002,
  TOO_MANY_CONCURRENT_CALLS = 1003,
  RESPONSE_TOO_LARGE = 1004,
  RESPONSE_DECODE_FAILED = 1005,
  CLOSED = 1006,
  NO_VALUE = 1007,
  UNKNOWN_KIND = 1008,
  DISCONNECTED = 1009,
  OVERFLOW = 1010,
  REQUEST_TOO_LARGE = 1011,
  TOO_MANY_CALLS = 1012,
  TIMEOUT = 1013,
}
/* @internal */
export const strims_rpc_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_rpc_errors_ErrorCode = ErrorCode;
