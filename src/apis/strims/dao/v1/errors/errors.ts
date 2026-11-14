import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export enum ErrorCode {
  UNDEFINED = 0,
  KV = 2000,
  SEAL = 2001,
  KEY = 2002,
  RECORD_DECODE = 2003,
  IDS_EXHAUSTED = 2004,
  INVALID_RANGES = 2005,
  RECORD_NOT_FOUND = 2006,
  UNIQUE_VIOLATION = 2007,
  UNSUPPORTED_STORE_VERSION = 2008,
  STORE_EXISTS = 2009,
  NO_REPLICA_ID = 2010,
  DELETED = 2011,
  SUPERSEDED = 2012,
  INVALID_CREDENTIALS = 2013,
  NAME_TAKEN = 2014,
  LEASE_BUSY = 2015,
  INVALID = 2016,
}
/* @internal */
export const strims_dao_v1_errors_ErrorCode = ErrorCode;
/* @internal */
export type strims_dao_v1_errors_ErrorCode = ErrorCode;
