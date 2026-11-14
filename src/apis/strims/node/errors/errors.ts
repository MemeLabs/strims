import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  STOPPED = 3000,
  IO = 3001,
  DIAL = 3002,
  OWN_HOST = 3003,
  INVALID_URI = 3004,
  UNKNOWN_PEER = 3005,
  UNKNOWN_SWARM = 3006,
  CLOSED = 3007,
  UNSUPPORTED = 3008,
  RPC_TAKEN = 3009,
  RPC_OVERFLOW = 3010,
  CONFIG = 3011,
  VNIC = 3012,
  VPN = 3013,
  CERTIFICATE = 3014,
  RUNNER = 3015,
  SWARM = 3016,
  UNDERRUN = 3017,
  RESET = 3018,
  READ_STORE = 3019,
  REWIND_REFUSED = 3020,
}
/* @internal */
export const strims_node_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_node_errors_ErrorCode = ErrorCode;
