import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  NOT_OPEN = 9000,
  REPLICA_DELETED = 9001,
  UNSUPPORTED_STORE_VERSION = 9002,
  INVALID_REPLICA_ID = 9003,
  OVER_LIMIT = 9004,
  INVALID_ANSWER = 9005,
}
/* @internal */
export const strims_replication_v1_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_replication_v1_errors_ErrorCode = ErrorCode;
