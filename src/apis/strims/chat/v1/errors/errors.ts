import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  COMBO_DUPLICATE = 5000,
  NOT_MODERATOR = 5001,
  MUTED = 5002,
  MESSAGE_TOO_LONG = 5003,
  TOO_FAST = 5004,
  TOO_LARGE = 5005,
  NETWORK_NOT_FOUND = 5006,
  NOT_JOINED = 5007,
  OTHER_NETWORK = 5008,
  SERVER_NOT_FOUND = 5009,
  TOO_MANY_ROOMS = 5010,
  TOO_MANY_STREAMS = 5011,
  NO_CALLER_CERTIFICATE = 5012,
  WHISPER_REFUSED = 5013,
  NOT_FOUND = 5014,
  INVALID_DURATION = 5015,
  NAME_EMPTY = 5016,
  NAME_TOO_LONG = 5017,
  RECORD_TOO_LARGE = 5018,
  ALIAS_TOO_LONG = 5019,
  TAG_COLOR_TOO_LONG = 5020,
  TOO_MANY_LABELS = 5021,
  INVALID_PROC_CHANCE = 5022,
  INVALID_PART = 5023,
  TOO_MANY_RECORDS = 5024,
  WHISPER_SUBJECT_UNDEFINED = 5025,
  INVALID_CERTIFICATE = 5026,
  WHISPER_TO_SELF = 5027,
  TOO_MANY_WHISPERS_PENDING = 5028,
}
/* @internal */
export const strims_chat_v1_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_chat_v1_errors_ErrorCode = ErrorCode;
