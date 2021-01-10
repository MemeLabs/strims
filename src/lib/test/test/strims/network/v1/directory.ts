import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate
} from "../../type/certificate";

export interface IGetDirectoryEventsRequest {
  networkKey?: Uint8Array;
}

export class GetDirectoryEventsRequest {
  networkKey: Uint8Array = new Uint8Array();

  constructor(v?: IGetDirectoryEventsRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: GetDirectoryEventsRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetDirectoryEventsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetDirectoryEventsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ITestDirectoryPublishRequest {
  networkKey?: Uint8Array;
}

export class TestDirectoryPublishRequest {
  networkKey: Uint8Array = new Uint8Array();

  constructor(v?: ITestDirectoryPublishRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: TestDirectoryPublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TestDirectoryPublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TestDirectoryPublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ITestDirectoryPublishResponse {
}

export class TestDirectoryPublishResponse {

  constructor(v?: ITestDirectoryPublishResponse) {
    // noop
  }

  static encode(m: TestDirectoryPublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TestDirectoryPublishResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new TestDirectoryPublishResponse();
  }
}

export interface IDirectoryListingSnippet {
  title?: string;
  description?: string;
  tags?: string[];
}

export class DirectoryListingSnippet {
  title: string = "";
  description: string = "";
  tags: string[] = [];

  constructor(v?: IDirectoryListingSnippet) {
    this.title = v?.title || "";
    this.description = v?.description || "";
    if (v?.tags) this.tags = v.tags;
  }

  static encode(m: DirectoryListingSnippet, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.title) w.uint32(10).string(m.title);
    if (m.description) w.uint32(18).string(m.description);
    for (const v of m.tags) w.uint32(26).string(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryListingSnippet {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryListingSnippet();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.title = r.string();
        break;
        case 2:
        m.description = r.string();
        break;
        case 3:
        m.tags.push(r.string())
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryListingMedia {
  startedAt?: bigint;
  mimeType?: string;
  bitrate?: number;
  swarmUri?: string;
}

export class DirectoryListingMedia {
  startedAt: bigint = BigInt(0);
  mimeType: string = "";
  bitrate: number = 0;
  swarmUri: string = "";

  constructor(v?: IDirectoryListingMedia) {
    this.startedAt = v?.startedAt || BigInt(0);
    this.mimeType = v?.mimeType || "";
    this.bitrate = v?.bitrate || 0;
    this.swarmUri = v?.swarmUri || "";
  }

  static encode(m: DirectoryListingMedia, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.startedAt) w.uint32(8).int64(m.startedAt);
    if (m.mimeType) w.uint32(18).string(m.mimeType);
    if (m.bitrate) w.uint32(24).uint32(m.bitrate);
    if (m.swarmUri) w.uint32(34).string(m.swarmUri);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryListingMedia {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryListingMedia();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.startedAt = r.int64();
        break;
        case 2:
        m.mimeType = r.string();
        break;
        case 3:
        m.bitrate = r.uint32();
        break;
        case 4:
        m.swarmUri = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryListingService {
  type?: string;
}

export class DirectoryListingService {
  type: string = "";

  constructor(v?: IDirectoryListingService) {
    this.type = v?.type || "";
  }

  static encode(m: DirectoryListingService, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.type) w.uint32(10).string(m.type);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryListingService {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryListingService();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.type = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryListing {
  creator?: strims_type_ICertificate;
  timestamp?: bigint;
  snippet?: IDirectoryListingSnippet;
  key?: Uint8Array;
  signature?: Uint8Array;
  content?: DirectoryListing.IContentOneOf
}

export class DirectoryListing {
  creator: strims_type_Certificate | undefined;
  timestamp: bigint = BigInt(0);
  snippet: DirectoryListingSnippet | undefined;
  key: Uint8Array = new Uint8Array();
  signature: Uint8Array = new Uint8Array();
  content: DirectoryListing.ContentOneOf;

  constructor(v?: IDirectoryListing) {
    this.creator = v?.creator && new strims_type_Certificate(v.creator);
    this.timestamp = v?.timestamp || BigInt(0);
    this.snippet = v?.snippet && new DirectoryListingSnippet(v.snippet);
    this.key = v?.key || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
    this.content = new DirectoryListing.ContentOneOf(v?.content);
  }

  static encode(m: DirectoryListing, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.creator) strims_type_Certificate.encode(m.creator, w.uint32(10).fork()).ldelim();
    if (m.timestamp) w.uint32(16).int64(m.timestamp);
    if (m.snippet) DirectoryListingSnippet.encode(m.snippet, w.uint32(26).fork()).ldelim();
    if (m.key) w.uint32(80010).bytes(m.key);
    if (m.signature) w.uint32(80018).bytes(m.signature);
    switch (m.content.case) {
      case 1001:
      DirectoryListingMedia.encode(m.content.media, w.uint32(8010).fork()).ldelim();
      break;
      case 1002:
      DirectoryListingService.encode(m.content.service, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryListing {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryListing();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.creator = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 2:
        m.timestamp = r.int64();
        break;
        case 3:
        m.snippet = DirectoryListingSnippet.decode(r, r.uint32());
        break;
        case 1001:
        m.content.media = DirectoryListingMedia.decode(r, r.uint32());
        break;
        case 1002:
        m.content.service = DirectoryListingService.decode(r, r.uint32());
        break;
        case 10001:
        m.key = r.bytes();
        break;
        case 10002:
        m.signature = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace DirectoryListing {
  export type IContentOneOf =
  { media: IDirectoryListingMedia }
  |{ service: IDirectoryListingService }
  ;

  export class ContentOneOf {
    private _media: DirectoryListingMedia | undefined;
    private _service: DirectoryListingService | undefined;
    private _case: ContentCase = 0;

    constructor(v?: IContentOneOf) {
      if (v && "media" in v) this.media = new DirectoryListingMedia(v.media);
      if (v && "service" in v) this.service = new DirectoryListingService(v.service);
    }

    public clear() {
      this._media = undefined;
      this._service = undefined;
      this._case = ContentCase.NOT_SET;
    }

    get case(): ContentCase {
      return this._case;
    }

    set media(v: DirectoryListingMedia) {
      this.clear();
      this._media = v;
      this._case = ContentCase.MEDIA;
    }

    get media(): DirectoryListingMedia {
      return this._media;
    }

    set service(v: DirectoryListingService) {
      this.clear();
      this._service = v;
      this._case = ContentCase.SERVICE;
    }

    get service(): DirectoryListingService {
      return this._service;
    }
  }

  export enum ContentCase {
    NOT_SET = 0,
    MEDIA = 1001,
    SERVICE = 1002,
  }

}

export interface IDirectoryEvent {
  body?: DirectoryEvent.IBodyOneOf
}

export class DirectoryEvent {
  body: DirectoryEvent.BodyOneOf;

  constructor(v?: IDirectoryEvent) {
    this.body = new DirectoryEvent.BodyOneOf(v?.body);
  }

  static encode(m: DirectoryEvent, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    switch (m.body.case) {
      case 1:
      DirectoryEvent.Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case 2:
      DirectoryEvent.Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case 3:
      DirectoryEvent.ViewerCountChange.encode(m.body.viewerCountChange, w.uint32(26).fork()).ldelim();
      break;
      case 4:
      DirectoryEvent.ViewerStateChange.encode(m.body.viewerStateChange, w.uint32(34).fork()).ldelim();
      break;
      case 5:
      DirectoryEvent.Ping.encode(m.body.ping, w.uint32(42).fork()).ldelim();
      break;
      case 6:
      DirectoryEvent.Padding.encode(m.body.padding, w.uint32(50).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body.publish = DirectoryEvent.Publish.decode(r, r.uint32());
        break;
        case 2:
        m.body.unpublish = DirectoryEvent.Unpublish.decode(r, r.uint32());
        break;
        case 3:
        m.body.viewerCountChange = DirectoryEvent.ViewerCountChange.decode(r, r.uint32());
        break;
        case 4:
        m.body.viewerStateChange = DirectoryEvent.ViewerStateChange.decode(r, r.uint32());
        break;
        case 5:
        m.body.ping = DirectoryEvent.Ping.decode(r, r.uint32());
        break;
        case 6:
        m.body.padding = DirectoryEvent.Padding.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace DirectoryEvent {
  export type IBodyOneOf =
  { publish: DirectoryEvent.IPublish }
  |{ unpublish: DirectoryEvent.IUnpublish }
  |{ viewerCountChange: DirectoryEvent.IViewerCountChange }
  |{ viewerStateChange: DirectoryEvent.IViewerStateChange }
  |{ ping: DirectoryEvent.IPing }
  |{ padding: DirectoryEvent.IPadding }
  ;

  export class BodyOneOf {
    private _publish: DirectoryEvent.Publish | undefined;
    private _unpublish: DirectoryEvent.Unpublish | undefined;
    private _viewerCountChange: DirectoryEvent.ViewerCountChange | undefined;
    private _viewerStateChange: DirectoryEvent.ViewerStateChange | undefined;
    private _ping: DirectoryEvent.Ping | undefined;
    private _padding: DirectoryEvent.Padding | undefined;
    private _case: BodyCase = 0;

    constructor(v?: IBodyOneOf) {
      if (v && "publish" in v) this.publish = new DirectoryEvent.Publish(v.publish);
      if (v && "unpublish" in v) this.unpublish = new DirectoryEvent.Unpublish(v.unpublish);
      if (v && "viewerCountChange" in v) this.viewerCountChange = new DirectoryEvent.ViewerCountChange(v.viewerCountChange);
      if (v && "viewerStateChange" in v) this.viewerStateChange = new DirectoryEvent.ViewerStateChange(v.viewerStateChange);
      if (v && "ping" in v) this.ping = new DirectoryEvent.Ping(v.ping);
      if (v && "padding" in v) this.padding = new DirectoryEvent.Padding(v.padding);
    }

    public clear() {
      this._publish = undefined;
      this._unpublish = undefined;
      this._viewerCountChange = undefined;
      this._viewerStateChange = undefined;
      this._ping = undefined;
      this._padding = undefined;
      this._case = BodyCase.NOT_SET;
    }

    get case(): BodyCase {
      return this._case;
    }

    set publish(v: DirectoryEvent.Publish) {
      this.clear();
      this._publish = v;
      this._case = BodyCase.PUBLISH;
    }

    get publish(): DirectoryEvent.Publish {
      return this._publish;
    }

    set unpublish(v: DirectoryEvent.Unpublish) {
      this.clear();
      this._unpublish = v;
      this._case = BodyCase.UNPUBLISH;
    }

    get unpublish(): DirectoryEvent.Unpublish {
      return this._unpublish;
    }

    set viewerCountChange(v: DirectoryEvent.ViewerCountChange) {
      this.clear();
      this._viewerCountChange = v;
      this._case = BodyCase.VIEWER_COUNT_CHANGE;
    }

    get viewerCountChange(): DirectoryEvent.ViewerCountChange {
      return this._viewerCountChange;
    }

    set viewerStateChange(v: DirectoryEvent.ViewerStateChange) {
      this.clear();
      this._viewerStateChange = v;
      this._case = BodyCase.VIEWER_STATE_CHANGE;
    }

    get viewerStateChange(): DirectoryEvent.ViewerStateChange {
      return this._viewerStateChange;
    }

    set ping(v: DirectoryEvent.Ping) {
      this.clear();
      this._ping = v;
      this._case = BodyCase.PING;
    }

    get ping(): DirectoryEvent.Ping {
      return this._ping;
    }

    set padding(v: DirectoryEvent.Padding) {
      this.clear();
      this._padding = v;
      this._case = BodyCase.PADDING;
    }

    get padding(): DirectoryEvent.Padding {
      return this._padding;
    }
  }

  export enum BodyCase {
    NOT_SET = 0,
    PUBLISH = 1,
    UNPUBLISH = 2,
    VIEWER_COUNT_CHANGE = 3,
    VIEWER_STATE_CHANGE = 4,
    PING = 5,
    PADDING = 6,
  }

  export interface IPublish {
    listing?: IDirectoryListing;
  }

  export class Publish {
    listing: DirectoryListing | undefined;

    constructor(v?: IPublish) {
      this.listing = v?.listing && new DirectoryListing(v.listing);
    }

    static encode(m: Publish, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.listing) DirectoryListing.encode(m.listing, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Publish {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Publish();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.listing = DirectoryListing.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IUnpublish {
    key?: Uint8Array;
  }

  export class Unpublish {
    key: Uint8Array = new Uint8Array();

    constructor(v?: IUnpublish) {
      this.key = v?.key || new Uint8Array();
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.key) w.uint32(10).bytes(m.key);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Unpublish {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Unpublish();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.key = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IViewerCountChange {
    key?: Uint8Array;
    count?: number;
  }

  export class ViewerCountChange {
    key: Uint8Array = new Uint8Array();
    count: number = 0;

    constructor(v?: IViewerCountChange) {
      this.key = v?.key || new Uint8Array();
      this.count = v?.count || 0;
    }

    static encode(m: ViewerCountChange, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.key) w.uint32(10).bytes(m.key);
      if (m.count) w.uint32(16).uint32(m.count);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): ViewerCountChange {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new ViewerCountChange();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.key = r.bytes();
          break;
          case 2:
          m.count = r.uint32();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IViewerStateChange {
    subject?: string;
    online?: boolean;
    viewingKeys?: Uint8Array[];
  }

  export class ViewerStateChange {
    subject: string = "";
    online: boolean = false;
    viewingKeys: Uint8Array[] = [];

    constructor(v?: IViewerStateChange) {
      this.subject = v?.subject || "";
      this.online = v?.online || false;
      if (v?.viewingKeys) this.viewingKeys = v.viewingKeys;
    }

    static encode(m: ViewerStateChange, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.subject) w.uint32(10).string(m.subject);
      if (m.online) w.uint32(16).bool(m.online);
      for (const v of m.viewingKeys) w.uint32(26).bytes(v);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): ViewerStateChange {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new ViewerStateChange();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.subject = r.string();
          break;
          case 2:
          m.online = r.bool();
          break;
          case 3:
          m.viewingKeys.push(r.bytes())
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IPing {
    time?: bigint;
  }

  export class Ping {
    time: bigint = BigInt(0);

    constructor(v?: IPing) {
      this.time = v?.time || BigInt(0);
    }

    static encode(m: Ping, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.time) w.uint32(8).int64(m.time);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Ping {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Ping();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.time = r.int64();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IPadding {
    data?: Uint8Array;
  }

  export class Padding {
    data: Uint8Array = new Uint8Array();

    constructor(v?: IPadding) {
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: Padding, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.data) w.uint32(10).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Padding {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Padding();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.data = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}

export interface IDirectoryPublishRequest {
  listing?: IDirectoryListing;
}

export class DirectoryPublishRequest {
  listing: DirectoryListing | undefined;

  constructor(v?: IDirectoryPublishRequest) {
    this.listing = v?.listing && new DirectoryListing(v.listing);
  }

  static encode(m: DirectoryPublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.listing) DirectoryListing.encode(m.listing, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryPublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryPublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.listing = DirectoryListing.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryPublishResponse {
}

export class DirectoryPublishResponse {

  constructor(v?: IDirectoryPublishResponse) {
    // noop
  }

  static encode(m: DirectoryPublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryPublishResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DirectoryPublishResponse();
  }
}

export interface IDirectoryUnpublishRequest {
  key?: Uint8Array;
}

export class DirectoryUnpublishRequest {
  key: Uint8Array = new Uint8Array();

  constructor(v?: IDirectoryUnpublishRequest) {
    this.key = v?.key || new Uint8Array();
  }

  static encode(m: DirectoryUnpublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.key) w.uint32(10).bytes(m.key);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryUnpublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryUnpublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.key = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryUnpublishResponse {
}

export class DirectoryUnpublishResponse {

  constructor(v?: IDirectoryUnpublishResponse) {
    // noop
  }

  static encode(m: DirectoryUnpublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryUnpublishResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DirectoryUnpublishResponse();
  }
}

export interface IDirectoryJoinRequest {
  key?: Uint8Array;
}

export class DirectoryJoinRequest {
  key: Uint8Array = new Uint8Array();

  constructor(v?: IDirectoryJoinRequest) {
    this.key = v?.key || new Uint8Array();
  }

  static encode(m: DirectoryJoinRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.key) w.uint32(10).bytes(m.key);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryJoinRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryJoinRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.key = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryJoinResponse {
}

export class DirectoryJoinResponse {

  constructor(v?: IDirectoryJoinResponse) {
    // noop
  }

  static encode(m: DirectoryJoinResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryJoinResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DirectoryJoinResponse();
  }
}

export interface IDirectoryPartRequest {
  key?: Uint8Array;
}

export class DirectoryPartRequest {
  key: Uint8Array = new Uint8Array();

  constructor(v?: IDirectoryPartRequest) {
    this.key = v?.key || new Uint8Array();
  }

  static encode(m: DirectoryPartRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.key) w.uint32(10).bytes(m.key);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryPartRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryPartRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.key = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryPartResponse {
}

export class DirectoryPartResponse {

  constructor(v?: IDirectoryPartResponse) {
    // noop
  }

  static encode(m: DirectoryPartResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryPartResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DirectoryPartResponse();
  }
}

export interface IDirectoryPingRequest {
}

export class DirectoryPingRequest {

  constructor(v?: IDirectoryPingRequest) {
    // noop
  }

  static encode(m: DirectoryPingRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryPingRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new DirectoryPingRequest();
  }
}

export interface IDirectoryPingResponse {
}

export class DirectoryPingResponse {

  constructor(v?: IDirectoryPingResponse) {
    // noop
  }

  static encode(m: DirectoryPingResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryPingResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DirectoryPingResponse();
  }
}

export interface IDirectoryFrontendOpenRequest {
  networkKey?: Uint8Array;
}

export class DirectoryFrontendOpenRequest {
  networkKey: Uint8Array = new Uint8Array();

  constructor(v?: IDirectoryFrontendOpenRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: DirectoryFrontendOpenRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryFrontendOpenRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryFrontendOpenRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryFrontendOpenResponse {
  event?: IDirectoryEvent;
}

export class DirectoryFrontendOpenResponse {
  event: DirectoryEvent | undefined;

  constructor(v?: IDirectoryFrontendOpenResponse) {
    this.event = v?.event && new DirectoryEvent(v.event);
  }

  static encode(m: DirectoryFrontendOpenResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.event) DirectoryEvent.encode(m.event, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryFrontendOpenResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryFrontendOpenResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.event = DirectoryEvent.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryFrontendTestRequest {
  networkKey?: Uint8Array;
}

export class DirectoryFrontendTestRequest {
  networkKey: Uint8Array = new Uint8Array();

  constructor(v?: IDirectoryFrontendTestRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: DirectoryFrontendTestRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryFrontendTestRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryFrontendTestRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDirectoryFrontendTestResponse {
}

export class DirectoryFrontendTestResponse {

  constructor(v?: IDirectoryFrontendTestResponse) {
    // noop
  }

  static encode(m: DirectoryFrontendTestResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryFrontendTestResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DirectoryFrontendTestResponse();
  }
}

