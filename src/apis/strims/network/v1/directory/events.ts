import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  ListingRecord as strims_network_v1_directory_ListingRecord,
  IListingRecord as strims_network_v1_directory_IListingRecord,
} from "./directory";

export type IListingRecordChangeEvent = {
  record?: strims_network_v1_directory_IListingRecord;
}

export class ListingRecordChangeEvent {
  record: strims_network_v1_directory_ListingRecord | undefined;

  constructor(v?: IListingRecordChangeEvent) {
    this.record = v?.record && new strims_network_v1_directory_ListingRecord(v.record);
  }

  static encode(m: ListingRecordChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.record) strims_network_v1_directory_ListingRecord.encode(m.record, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingRecordChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListingRecordChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.record = strims_network_v1_directory_ListingRecord.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

