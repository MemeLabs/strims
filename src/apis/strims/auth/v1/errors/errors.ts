import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  CONNECTION_CLOSED = 7000,
  INVALID_PAIRING_TOKEN = 7001,
  MISSING_CREDENTIALS = 7002,
  NAME_REQUIRED = 7003,
  CANNOT_DELETE_CURRENT_DEVICE = 7004,
  CONFIG_MISSING = 7005,
}
/* @internal */
export const strims_auth_v1_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_auth_v1_errors_ErrorCode = ErrorCode;
