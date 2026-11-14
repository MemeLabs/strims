import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  INVALID_TRANSFER_ID = 8000,
  TRANSFERS_STOPPED = 8001,
}
/* @internal */
export const strims_transfer_v1_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_transfer_v1_errors_ErrorCode = ErrorCode;
