import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  TOO_MANY_MOCK_STREAMS = 10000,
  BITRATE_OUT_OF_RANGE = 10001,
  INTERVAL_OUT_OF_RANGE = 10002,
  TIMEOUT_OUT_OF_RANGE = 10003,
  SEGMENT_TOO_LARGE = 10004,
  NETWORK_NOT_FOUND = 10005,
}
/* @internal */
export const strims_debug_v1_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_debug_v1_errors_ErrorCode = ErrorCode;
