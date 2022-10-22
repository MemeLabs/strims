import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  UNKNOWN = 1,
  ALIAS_IN_USE = 2,
  ALIAS_CHANGE_COOLDOWN_VIOLATIED = 3,
  INVITATION_QUOTA_EXCEEDED = 4,
  INVITER_BANNED = 5,
}
/* @internal */
export const strims_network_v1_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_network_v1_errors_ErrorCode = ErrorCode;
