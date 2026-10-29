import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  google_protobuf_Int64Value,
  google_protobuf_IInt64Value,
  google_protobuf_UInt32Value,
  google_protobuf_IUInt32Value,
  google_protobuf_UInt64Value,
  google_protobuf_IUInt64Value,
} from "../../../google/protobuf/wrappers";

export type IMetrics = {
  time?: bigint;
  memory?: strims_debug_v1_IMemoryMetrics;
  node?: strims_debug_v1_INodeMetrics;
  network?: strims_debug_v1_INetworkMetrics;
  transfer?: strims_debug_v1_ITransferMetrics;
  replication?: strims_debug_v1_IReplicationMetrics;
  directory?: strims_debug_v1_IDirectoryNetworkMetrics[];
  chat?: strims_debug_v1_IChatMetrics;
  whispers?: strims_debug_v1_IWhisperMetrics;
  video?: strims_debug_v1_IVideoMetrics;
  debug?: strims_debug_v1_IDebugMetrics;
}

export class Metrics {
  time: bigint;
  memory: strims_debug_v1_MemoryMetrics | undefined;
  node: strims_debug_v1_NodeMetrics | undefined;
  network: strims_debug_v1_NetworkMetrics | undefined;
  transfer: strims_debug_v1_TransferMetrics | undefined;
  replication: strims_debug_v1_ReplicationMetrics | undefined;
  directory: strims_debug_v1_DirectoryNetworkMetrics[];
  chat: strims_debug_v1_ChatMetrics | undefined;
  whispers: strims_debug_v1_WhisperMetrics | undefined;
  video: strims_debug_v1_VideoMetrics | undefined;
  debug: strims_debug_v1_DebugMetrics | undefined;

  constructor(v?: IMetrics) {
    this.time = v?.time || BigInt(0);
    this.memory = v?.memory && new strims_debug_v1_MemoryMetrics(v.memory);
    this.node = v?.node && new strims_debug_v1_NodeMetrics(v.node);
    this.network = v?.network && new strims_debug_v1_NetworkMetrics(v.network);
    this.transfer = v?.transfer && new strims_debug_v1_TransferMetrics(v.transfer);
    this.replication = v?.replication && new strims_debug_v1_ReplicationMetrics(v.replication);
    this.directory = v?.directory ? v.directory.map(v => new strims_debug_v1_DirectoryNetworkMetrics(v)) : [];
    this.chat = v?.chat && new strims_debug_v1_ChatMetrics(v.chat);
    this.whispers = v?.whispers && new strims_debug_v1_WhisperMetrics(v.whispers);
    this.video = v?.video && new strims_debug_v1_VideoMetrics(v.video);
    this.debug = v?.debug && new strims_debug_v1_DebugMetrics(v.debug);
  }

  static encode(m: Metrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.time) w.uint32(8).int64(m.time);
    if (m.memory) strims_debug_v1_MemoryMetrics.encode(m.memory, w.uint32(18).fork()).ldelim();
    if (m.node) strims_debug_v1_NodeMetrics.encode(m.node, w.uint32(26).fork()).ldelim();
    if (m.network) strims_debug_v1_NetworkMetrics.encode(m.network, w.uint32(34).fork()).ldelim();
    if (m.transfer) strims_debug_v1_TransferMetrics.encode(m.transfer, w.uint32(42).fork()).ldelim();
    if (m.replication) strims_debug_v1_ReplicationMetrics.encode(m.replication, w.uint32(50).fork()).ldelim();
    for (const v of m.directory) strims_debug_v1_DirectoryNetworkMetrics.encode(v, w.uint32(58).fork()).ldelim();
    if (m.chat) strims_debug_v1_ChatMetrics.encode(m.chat, w.uint32(66).fork()).ldelim();
    if (m.whispers) strims_debug_v1_WhisperMetrics.encode(m.whispers, w.uint32(74).fork()).ldelim();
    if (m.video) strims_debug_v1_VideoMetrics.encode(m.video, w.uint32(82).fork()).ldelim();
    if (m.debug) strims_debug_v1_DebugMetrics.encode(m.debug, w.uint32(90).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Metrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Metrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.time = r.int64();
        break;
        case 2:
        m.memory = strims_debug_v1_MemoryMetrics.decode(r, r.uint32());
        break;
        case 3:
        m.node = strims_debug_v1_NodeMetrics.decode(r, r.uint32());
        break;
        case 4:
        m.network = strims_debug_v1_NetworkMetrics.decode(r, r.uint32());
        break;
        case 5:
        m.transfer = strims_debug_v1_TransferMetrics.decode(r, r.uint32());
        break;
        case 6:
        m.replication = strims_debug_v1_ReplicationMetrics.decode(r, r.uint32());
        break;
        case 7:
        m.directory.push(strims_debug_v1_DirectoryNetworkMetrics.decode(r, r.uint32()));
        break;
        case 8:
        m.chat = strims_debug_v1_ChatMetrics.decode(r, r.uint32());
        break;
        case 9:
        m.whispers = strims_debug_v1_WhisperMetrics.decode(r, r.uint32());
        break;
        case 10:
        m.video = strims_debug_v1_VideoMetrics.decode(r, r.uint32());
        break;
        case 11:
        m.debug = strims_debug_v1_DebugMetrics.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMemoryMetrics = {
  liveBytes?: bigint;
  peakBytes?: bigint;
  allocatedBytes?: bigint;
  allocations?: bigint;
  deallocations?: bigint;
  reallocations?: bigint;
  linearMemoryBytes?: bigint;
}

export class MemoryMetrics {
  liveBytes: bigint;
  peakBytes: bigint;
  allocatedBytes: bigint;
  allocations: bigint;
  deallocations: bigint;
  reallocations: bigint;
  linearMemoryBytes: bigint;

  constructor(v?: IMemoryMetrics) {
    this.liveBytes = v?.liveBytes || BigInt(0);
    this.peakBytes = v?.peakBytes || BigInt(0);
    this.allocatedBytes = v?.allocatedBytes || BigInt(0);
    this.allocations = v?.allocations || BigInt(0);
    this.deallocations = v?.deallocations || BigInt(0);
    this.reallocations = v?.reallocations || BigInt(0);
    this.linearMemoryBytes = v?.linearMemoryBytes || BigInt(0);
  }

  static encode(m: MemoryMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.liveBytes) w.uint32(8).uint64(m.liveBytes);
    if (m.peakBytes) w.uint32(16).uint64(m.peakBytes);
    if (m.allocatedBytes) w.uint32(24).uint64(m.allocatedBytes);
    if (m.allocations) w.uint32(32).uint64(m.allocations);
    if (m.deallocations) w.uint32(40).uint64(m.deallocations);
    if (m.reallocations) w.uint32(48).uint64(m.reallocations);
    if (m.linearMemoryBytes) w.uint32(56).uint64(m.linearMemoryBytes);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MemoryMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MemoryMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.liveBytes = r.uint64();
        break;
        case 2:
        m.peakBytes = r.uint64();
        break;
        case 3:
        m.allocatedBytes = r.uint64();
        break;
        case 4:
        m.allocations = r.uint64();
        break;
        case 5:
        m.deallocations = r.uint64();
        break;
        case 6:
        m.reallocations = r.uint64();
        break;
        case 7:
        m.linearMemoryBytes = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INodeMetrics = {
  host?: strims_debug_v1_IHostMetrics;
  vpn?: strims_debug_v1_IVpnMetrics;
  driver?: strims_debug_v1_IDriverMetrics;
  links?: strims_debug_v1_ILinkInfo[];
  runner?: strims_debug_v1_IRunnerMetrics;
  networks?: strims_debug_v1_INetworkInfo[];
}

export class NodeMetrics {
  host: strims_debug_v1_HostMetrics | undefined;
  vpn: strims_debug_v1_VpnMetrics | undefined;
  driver: strims_debug_v1_DriverMetrics | undefined;
  links: strims_debug_v1_LinkInfo[];
  runner: strims_debug_v1_RunnerMetrics | undefined;
  networks: strims_debug_v1_NetworkInfo[];

  constructor(v?: INodeMetrics) {
    this.host = v?.host && new strims_debug_v1_HostMetrics(v.host);
    this.vpn = v?.vpn && new strims_debug_v1_VpnMetrics(v.vpn);
    this.driver = v?.driver && new strims_debug_v1_DriverMetrics(v.driver);
    this.links = v?.links ? v.links.map(v => new strims_debug_v1_LinkInfo(v)) : [];
    this.runner = v?.runner && new strims_debug_v1_RunnerMetrics(v.runner);
    this.networks = v?.networks ? v.networks.map(v => new strims_debug_v1_NetworkInfo(v)) : [];
  }

  static encode(m: NodeMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.host) strims_debug_v1_HostMetrics.encode(m.host, w.uint32(10).fork()).ldelim();
    if (m.vpn) strims_debug_v1_VpnMetrics.encode(m.vpn, w.uint32(18).fork()).ldelim();
    if (m.driver) strims_debug_v1_DriverMetrics.encode(m.driver, w.uint32(26).fork()).ldelim();
    for (const v of m.links) strims_debug_v1_LinkInfo.encode(v, w.uint32(34).fork()).ldelim();
    if (m.runner) strims_debug_v1_RunnerMetrics.encode(m.runner, w.uint32(42).fork()).ldelim();
    for (const v of m.networks) strims_debug_v1_NetworkInfo.encode(v, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NodeMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NodeMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.host = strims_debug_v1_HostMetrics.decode(r, r.uint32());
        break;
        case 2:
        m.vpn = strims_debug_v1_VpnMetrics.decode(r, r.uint32());
        break;
        case 3:
        m.driver = strims_debug_v1_DriverMetrics.decode(r, r.uint32());
        break;
        case 4:
        m.links.push(strims_debug_v1_LinkInfo.decode(r, r.uint32()));
        break;
        case 5:
        m.runner = strims_debug_v1_RunnerMetrics.decode(r, r.uint32());
        break;
        case 6:
        m.networks.push(strims_debug_v1_NetworkInfo.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IHostMetrics = {
  linksActive?: bigint;
  frameReadCount?: bigint;
  frameReadBytes?: bigint;
  frameHandlerNotFoundCount?: bigint;
  frameWriteCount?: bigint;
  frameWriteBytes?: bigint;
  keepaliveWriteCount?: bigint;
  keepaliveReadCount?: bigint;
  handshakeTimeoutCount?: bigint;
  handshakeEvictCount?: bigint;
}

export class HostMetrics {
  linksActive: bigint;
  frameReadCount: bigint;
  frameReadBytes: bigint;
  frameHandlerNotFoundCount: bigint;
  frameWriteCount: bigint;
  frameWriteBytes: bigint;
  keepaliveWriteCount: bigint;
  keepaliveReadCount: bigint;
  handshakeTimeoutCount: bigint;
  handshakeEvictCount: bigint;

  constructor(v?: IHostMetrics) {
    this.linksActive = v?.linksActive || BigInt(0);
    this.frameReadCount = v?.frameReadCount || BigInt(0);
    this.frameReadBytes = v?.frameReadBytes || BigInt(0);
    this.frameHandlerNotFoundCount = v?.frameHandlerNotFoundCount || BigInt(0);
    this.frameWriteCount = v?.frameWriteCount || BigInt(0);
    this.frameWriteBytes = v?.frameWriteBytes || BigInt(0);
    this.keepaliveWriteCount = v?.keepaliveWriteCount || BigInt(0);
    this.keepaliveReadCount = v?.keepaliveReadCount || BigInt(0);
    this.handshakeTimeoutCount = v?.handshakeTimeoutCount || BigInt(0);
    this.handshakeEvictCount = v?.handshakeEvictCount || BigInt(0);
  }

  static encode(m: HostMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.linksActive) w.uint32(8).uint64(m.linksActive);
    if (m.frameReadCount) w.uint32(16).uint64(m.frameReadCount);
    if (m.frameReadBytes) w.uint32(24).uint64(m.frameReadBytes);
    if (m.frameHandlerNotFoundCount) w.uint32(32).uint64(m.frameHandlerNotFoundCount);
    if (m.frameWriteCount) w.uint32(40).uint64(m.frameWriteCount);
    if (m.frameWriteBytes) w.uint32(48).uint64(m.frameWriteBytes);
    if (m.keepaliveWriteCount) w.uint32(56).uint64(m.keepaliveWriteCount);
    if (m.keepaliveReadCount) w.uint32(64).uint64(m.keepaliveReadCount);
    if (m.handshakeTimeoutCount) w.uint32(72).uint64(m.handshakeTimeoutCount);
    if (m.handshakeEvictCount) w.uint32(80).uint64(m.handshakeEvictCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HostMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new HostMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.linksActive = r.uint64();
        break;
        case 2:
        m.frameReadCount = r.uint64();
        break;
        case 3:
        m.frameReadBytes = r.uint64();
        break;
        case 4:
        m.frameHandlerNotFoundCount = r.uint64();
        break;
        case 5:
        m.frameWriteCount = r.uint64();
        break;
        case 6:
        m.frameWriteBytes = r.uint64();
        break;
        case 7:
        m.keepaliveWriteCount = r.uint64();
        break;
        case 8:
        m.keepaliveReadCount = r.uint64();
        break;
        case 9:
        m.handshakeTimeoutCount = r.uint64();
        break;
        case 10:
        m.handshakeEvictCount = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVpnMetrics = {
  messagesSent?: bigint;
  messagesRelayed?: bigint;
  framesReceived?: bigint;
  framesQueued?: bigint;
  framesSent?: bigint;
  messagesDelivered?: bigint;
  duplicates?: bigint;
  malformed?: bigint;
  trailerMismatch?: bigint;
  compressedDropped?: bigint;
  verifyFailures?: bigint;
  unverifiedDropped?: bigint;
  hopLimit?: bigint;
  linkQueueFull?: bigint;
  tooLarge?: bigint;
  noHandler?: bigint;
  decryptFailures?: bigint;
  handlerErrors?: bigint;
  deliveriesDropped?: bigint;
  dedupEvictions?: bigint;
  shorterRelays?: bigint;
  learnedRoutes?: bigint;
  fallbackRoutes?: bigint;
  linkAnnouncementsSent?: bigint;
  linkAnnouncementsReceived?: bigint;
  recordsPushed?: bigint;
}

export class VpnMetrics {
  messagesSent: bigint;
  messagesRelayed: bigint;
  framesReceived: bigint;
  framesQueued: bigint;
  framesSent: bigint;
  messagesDelivered: bigint;
  duplicates: bigint;
  malformed: bigint;
  trailerMismatch: bigint;
  compressedDropped: bigint;
  verifyFailures: bigint;
  unverifiedDropped: bigint;
  hopLimit: bigint;
  linkQueueFull: bigint;
  tooLarge: bigint;
  noHandler: bigint;
  decryptFailures: bigint;
  handlerErrors: bigint;
  deliveriesDropped: bigint;
  dedupEvictions: bigint;
  shorterRelays: bigint;
  learnedRoutes: bigint;
  fallbackRoutes: bigint;
  linkAnnouncementsSent: bigint;
  linkAnnouncementsReceived: bigint;
  recordsPushed: bigint;

  constructor(v?: IVpnMetrics) {
    this.messagesSent = v?.messagesSent || BigInt(0);
    this.messagesRelayed = v?.messagesRelayed || BigInt(0);
    this.framesReceived = v?.framesReceived || BigInt(0);
    this.framesQueued = v?.framesQueued || BigInt(0);
    this.framesSent = v?.framesSent || BigInt(0);
    this.messagesDelivered = v?.messagesDelivered || BigInt(0);
    this.duplicates = v?.duplicates || BigInt(0);
    this.malformed = v?.malformed || BigInt(0);
    this.trailerMismatch = v?.trailerMismatch || BigInt(0);
    this.compressedDropped = v?.compressedDropped || BigInt(0);
    this.verifyFailures = v?.verifyFailures || BigInt(0);
    this.unverifiedDropped = v?.unverifiedDropped || BigInt(0);
    this.hopLimit = v?.hopLimit || BigInt(0);
    this.linkQueueFull = v?.linkQueueFull || BigInt(0);
    this.tooLarge = v?.tooLarge || BigInt(0);
    this.noHandler = v?.noHandler || BigInt(0);
    this.decryptFailures = v?.decryptFailures || BigInt(0);
    this.handlerErrors = v?.handlerErrors || BigInt(0);
    this.deliveriesDropped = v?.deliveriesDropped || BigInt(0);
    this.dedupEvictions = v?.dedupEvictions || BigInt(0);
    this.shorterRelays = v?.shorterRelays || BigInt(0);
    this.learnedRoutes = v?.learnedRoutes || BigInt(0);
    this.fallbackRoutes = v?.fallbackRoutes || BigInt(0);
    this.linkAnnouncementsSent = v?.linkAnnouncementsSent || BigInt(0);
    this.linkAnnouncementsReceived = v?.linkAnnouncementsReceived || BigInt(0);
    this.recordsPushed = v?.recordsPushed || BigInt(0);
  }

  static encode(m: VpnMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.messagesSent) w.uint32(8).uint64(m.messagesSent);
    if (m.messagesRelayed) w.uint32(16).uint64(m.messagesRelayed);
    if (m.framesReceived) w.uint32(24).uint64(m.framesReceived);
    if (m.framesQueued) w.uint32(32).uint64(m.framesQueued);
    if (m.framesSent) w.uint32(40).uint64(m.framesSent);
    if (m.messagesDelivered) w.uint32(48).uint64(m.messagesDelivered);
    if (m.duplicates) w.uint32(56).uint64(m.duplicates);
    if (m.malformed) w.uint32(64).uint64(m.malformed);
    if (m.trailerMismatch) w.uint32(72).uint64(m.trailerMismatch);
    if (m.compressedDropped) w.uint32(80).uint64(m.compressedDropped);
    if (m.verifyFailures) w.uint32(88).uint64(m.verifyFailures);
    if (m.unverifiedDropped) w.uint32(96).uint64(m.unverifiedDropped);
    if (m.hopLimit) w.uint32(104).uint64(m.hopLimit);
    if (m.linkQueueFull) w.uint32(112).uint64(m.linkQueueFull);
    if (m.tooLarge) w.uint32(120).uint64(m.tooLarge);
    if (m.noHandler) w.uint32(128).uint64(m.noHandler);
    if (m.decryptFailures) w.uint32(136).uint64(m.decryptFailures);
    if (m.handlerErrors) w.uint32(144).uint64(m.handlerErrors);
    if (m.deliveriesDropped) w.uint32(152).uint64(m.deliveriesDropped);
    if (m.dedupEvictions) w.uint32(160).uint64(m.dedupEvictions);
    if (m.shorterRelays) w.uint32(168).uint64(m.shorterRelays);
    if (m.learnedRoutes) w.uint32(176).uint64(m.learnedRoutes);
    if (m.fallbackRoutes) w.uint32(184).uint64(m.fallbackRoutes);
    if (m.linkAnnouncementsSent) w.uint32(192).uint64(m.linkAnnouncementsSent);
    if (m.linkAnnouncementsReceived) w.uint32(200).uint64(m.linkAnnouncementsReceived);
    if (m.recordsPushed) w.uint32(208).uint64(m.recordsPushed);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VpnMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VpnMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.messagesSent = r.uint64();
        break;
        case 2:
        m.messagesRelayed = r.uint64();
        break;
        case 3:
        m.framesReceived = r.uint64();
        break;
        case 4:
        m.framesQueued = r.uint64();
        break;
        case 5:
        m.framesSent = r.uint64();
        break;
        case 6:
        m.messagesDelivered = r.uint64();
        break;
        case 7:
        m.duplicates = r.uint64();
        break;
        case 8:
        m.malformed = r.uint64();
        break;
        case 9:
        m.trailerMismatch = r.uint64();
        break;
        case 10:
        m.compressedDropped = r.uint64();
        break;
        case 11:
        m.verifyFailures = r.uint64();
        break;
        case 12:
        m.unverifiedDropped = r.uint64();
        break;
        case 13:
        m.hopLimit = r.uint64();
        break;
        case 14:
        m.linkQueueFull = r.uint64();
        break;
        case 15:
        m.tooLarge = r.uint64();
        break;
        case 16:
        m.noHandler = r.uint64();
        break;
        case 17:
        m.decryptFailures = r.uint64();
        break;
        case 18:
        m.handlerErrors = r.uint64();
        break;
        case 19:
        m.deliveriesDropped = r.uint64();
        break;
        case 20:
        m.dedupEvictions = r.uint64();
        break;
        case 21:
        m.shorterRelays = r.uint64();
        break;
        case 22:
        m.learnedRoutes = r.uint64();
        break;
        case 23:
        m.fallbackRoutes = r.uint64();
        break;
        case 24:
        m.linkAnnouncementsSent = r.uint64();
        break;
        case 25:
        m.linkAnnouncementsReceived = r.uint64();
        break;
        case 26:
        m.recordsPushed = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDriverMetrics = {
  tcpDials?: bigint;
  tcpDialErrors?: bigint;
  wsDials?: bigint;
  wsDialErrors?: bigint;
  accepted?: bigint;
  acceptErrors?: bigint;
  transportFailures?: bigint;
  linksClosed?: bigint;
  bytesRead?: bigint;
  bytesWritten?: bigint;
  eventsDropped?: bigint;
  resultsDropped?: bigint;
  vpnFrameErrors?: bigint;
  transferErrors?: bigint;
  rpcBytesRead?: bigint;
  rpcBytesWritten?: bigint;
  rpcOverflows?: bigint;
  brokerBytesRead?: bigint;
  brokerBytesWritten?: bigint;
  brokerOverflows?: bigint;
  vpnPortDrops?: bigint;
  vpnSendErrors?: bigint;
  commands?: bigint;
  queries?: bigint;
  queryWakes?: bigint;
  loopIterations?: bigint;
  webrtcConnections?: bigint;
  webrtcOpened?: bigint;
  webrtcClosed?: bigint;
  udpDatagramsRead?: bigint;
  udpBytesRead?: bigint;
  udpDatagramsWritten?: bigint;
  udpBytesWritten?: bigint;
  udpSendDropped?: bigint;
  udpUnrouted?: bigint;
}

export class DriverMetrics {
  tcpDials: bigint;
  tcpDialErrors: bigint;
  wsDials: bigint;
  wsDialErrors: bigint;
  accepted: bigint;
  acceptErrors: bigint;
  transportFailures: bigint;
  linksClosed: bigint;
  bytesRead: bigint;
  bytesWritten: bigint;
  eventsDropped: bigint;
  resultsDropped: bigint;
  vpnFrameErrors: bigint;
  transferErrors: bigint;
  rpcBytesRead: bigint;
  rpcBytesWritten: bigint;
  rpcOverflows: bigint;
  brokerBytesRead: bigint;
  brokerBytesWritten: bigint;
  brokerOverflows: bigint;
  vpnPortDrops: bigint;
  vpnSendErrors: bigint;
  commands: bigint;
  queries: bigint;
  queryWakes: bigint;
  loopIterations: bigint;
  webrtcConnections: bigint;
  webrtcOpened: bigint;
  webrtcClosed: bigint;
  udpDatagramsRead: bigint;
  udpBytesRead: bigint;
  udpDatagramsWritten: bigint;
  udpBytesWritten: bigint;
  udpSendDropped: bigint;
  udpUnrouted: bigint;

  constructor(v?: IDriverMetrics) {
    this.tcpDials = v?.tcpDials || BigInt(0);
    this.tcpDialErrors = v?.tcpDialErrors || BigInt(0);
    this.wsDials = v?.wsDials || BigInt(0);
    this.wsDialErrors = v?.wsDialErrors || BigInt(0);
    this.accepted = v?.accepted || BigInt(0);
    this.acceptErrors = v?.acceptErrors || BigInt(0);
    this.transportFailures = v?.transportFailures || BigInt(0);
    this.linksClosed = v?.linksClosed || BigInt(0);
    this.bytesRead = v?.bytesRead || BigInt(0);
    this.bytesWritten = v?.bytesWritten || BigInt(0);
    this.eventsDropped = v?.eventsDropped || BigInt(0);
    this.resultsDropped = v?.resultsDropped || BigInt(0);
    this.vpnFrameErrors = v?.vpnFrameErrors || BigInt(0);
    this.transferErrors = v?.transferErrors || BigInt(0);
    this.rpcBytesRead = v?.rpcBytesRead || BigInt(0);
    this.rpcBytesWritten = v?.rpcBytesWritten || BigInt(0);
    this.rpcOverflows = v?.rpcOverflows || BigInt(0);
    this.brokerBytesRead = v?.brokerBytesRead || BigInt(0);
    this.brokerBytesWritten = v?.brokerBytesWritten || BigInt(0);
    this.brokerOverflows = v?.brokerOverflows || BigInt(0);
    this.vpnPortDrops = v?.vpnPortDrops || BigInt(0);
    this.vpnSendErrors = v?.vpnSendErrors || BigInt(0);
    this.commands = v?.commands || BigInt(0);
    this.queries = v?.queries || BigInt(0);
    this.queryWakes = v?.queryWakes || BigInt(0);
    this.loopIterations = v?.loopIterations || BigInt(0);
    this.webrtcConnections = v?.webrtcConnections || BigInt(0);
    this.webrtcOpened = v?.webrtcOpened || BigInt(0);
    this.webrtcClosed = v?.webrtcClosed || BigInt(0);
    this.udpDatagramsRead = v?.udpDatagramsRead || BigInt(0);
    this.udpBytesRead = v?.udpBytesRead || BigInt(0);
    this.udpDatagramsWritten = v?.udpDatagramsWritten || BigInt(0);
    this.udpBytesWritten = v?.udpBytesWritten || BigInt(0);
    this.udpSendDropped = v?.udpSendDropped || BigInt(0);
    this.udpUnrouted = v?.udpUnrouted || BigInt(0);
  }

  static encode(m: DriverMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tcpDials) w.uint32(8).uint64(m.tcpDials);
    if (m.tcpDialErrors) w.uint32(16).uint64(m.tcpDialErrors);
    if (m.wsDials) w.uint32(24).uint64(m.wsDials);
    if (m.wsDialErrors) w.uint32(32).uint64(m.wsDialErrors);
    if (m.accepted) w.uint32(40).uint64(m.accepted);
    if (m.acceptErrors) w.uint32(48).uint64(m.acceptErrors);
    if (m.transportFailures) w.uint32(56).uint64(m.transportFailures);
    if (m.linksClosed) w.uint32(64).uint64(m.linksClosed);
    if (m.bytesRead) w.uint32(72).uint64(m.bytesRead);
    if (m.bytesWritten) w.uint32(80).uint64(m.bytesWritten);
    if (m.eventsDropped) w.uint32(88).uint64(m.eventsDropped);
    if (m.resultsDropped) w.uint32(96).uint64(m.resultsDropped);
    if (m.vpnFrameErrors) w.uint32(104).uint64(m.vpnFrameErrors);
    if (m.transferErrors) w.uint32(112).uint64(m.transferErrors);
    if (m.rpcBytesRead) w.uint32(120).uint64(m.rpcBytesRead);
    if (m.rpcBytesWritten) w.uint32(128).uint64(m.rpcBytesWritten);
    if (m.rpcOverflows) w.uint32(136).uint64(m.rpcOverflows);
    if (m.brokerBytesRead) w.uint32(144).uint64(m.brokerBytesRead);
    if (m.brokerBytesWritten) w.uint32(152).uint64(m.brokerBytesWritten);
    if (m.brokerOverflows) w.uint32(160).uint64(m.brokerOverflows);
    if (m.vpnPortDrops) w.uint32(168).uint64(m.vpnPortDrops);
    if (m.vpnSendErrors) w.uint32(176).uint64(m.vpnSendErrors);
    if (m.commands) w.uint32(184).uint64(m.commands);
    if (m.queries) w.uint32(192).uint64(m.queries);
    if (m.queryWakes) w.uint32(200).uint64(m.queryWakes);
    if (m.loopIterations) w.uint32(208).uint64(m.loopIterations);
    if (m.webrtcConnections) w.uint32(216).uint64(m.webrtcConnections);
    if (m.webrtcOpened) w.uint32(224).uint64(m.webrtcOpened);
    if (m.webrtcClosed) w.uint32(232).uint64(m.webrtcClosed);
    if (m.udpDatagramsRead) w.uint32(240).uint64(m.udpDatagramsRead);
    if (m.udpBytesRead) w.uint32(248).uint64(m.udpBytesRead);
    if (m.udpDatagramsWritten) w.uint32(256).uint64(m.udpDatagramsWritten);
    if (m.udpBytesWritten) w.uint32(264).uint64(m.udpBytesWritten);
    if (m.udpSendDropped) w.uint32(272).uint64(m.udpSendDropped);
    if (m.udpUnrouted) w.uint32(280).uint64(m.udpUnrouted);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DriverMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DriverMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tcpDials = r.uint64();
        break;
        case 2:
        m.tcpDialErrors = r.uint64();
        break;
        case 3:
        m.wsDials = r.uint64();
        break;
        case 4:
        m.wsDialErrors = r.uint64();
        break;
        case 5:
        m.accepted = r.uint64();
        break;
        case 6:
        m.acceptErrors = r.uint64();
        break;
        case 7:
        m.transportFailures = r.uint64();
        break;
        case 8:
        m.linksClosed = r.uint64();
        break;
        case 9:
        m.bytesRead = r.uint64();
        break;
        case 10:
        m.bytesWritten = r.uint64();
        break;
        case 11:
        m.eventsDropped = r.uint64();
        break;
        case 12:
        m.resultsDropped = r.uint64();
        break;
        case 13:
        m.vpnFrameErrors = r.uint64();
        break;
        case 14:
        m.transferErrors = r.uint64();
        break;
        case 15:
        m.rpcBytesRead = r.uint64();
        break;
        case 16:
        m.rpcBytesWritten = r.uint64();
        break;
        case 17:
        m.rpcOverflows = r.uint64();
        break;
        case 18:
        m.brokerBytesRead = r.uint64();
        break;
        case 19:
        m.brokerBytesWritten = r.uint64();
        break;
        case 20:
        m.brokerOverflows = r.uint64();
        break;
        case 21:
        m.vpnPortDrops = r.uint64();
        break;
        case 22:
        m.vpnSendErrors = r.uint64();
        break;
        case 23:
        m.commands = r.uint64();
        break;
        case 24:
        m.queries = r.uint64();
        break;
        case 25:
        m.queryWakes = r.uint64();
        break;
        case 26:
        m.loopIterations = r.uint64();
        break;
        case 27:
        m.webrtcConnections = r.uint64();
        break;
        case 28:
        m.webrtcOpened = r.uint64();
        break;
        case 29:
        m.webrtcClosed = r.uint64();
        break;
        case 30:
        m.udpDatagramsRead = r.uint64();
        break;
        case 31:
        m.udpBytesRead = r.uint64();
        break;
        case 32:
        m.udpDatagramsWritten = r.uint64();
        break;
        case 33:
        m.udpBytesWritten = r.uint64();
        break;
        case 34:
        m.udpSendDropped = r.uint64();
        break;
        case 35:
        m.udpUnrouted = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ILinkMetrics = {
  readBytes?: bigint;
  readCount?: bigint;
  writeBytes?: bigint;
  writeCount?: bigint;
}

export class LinkMetrics {
  readBytes: bigint;
  readCount: bigint;
  writeBytes: bigint;
  writeCount: bigint;

  constructor(v?: ILinkMetrics) {
    this.readBytes = v?.readBytes || BigInt(0);
    this.readCount = v?.readCount || BigInt(0);
    this.writeBytes = v?.writeBytes || BigInt(0);
    this.writeCount = v?.writeCount || BigInt(0);
  }

  static encode(m: LinkMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.readBytes) w.uint32(8).uint64(m.readBytes);
    if (m.readCount) w.uint32(16).uint64(m.readCount);
    if (m.writeBytes) w.uint32(24).uint64(m.writeBytes);
    if (m.writeCount) w.uint32(32).uint64(m.writeCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LinkMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.readBytes = r.uint64();
        break;
        case 2:
        m.readCount = r.uint64();
        break;
        case 3:
        m.writeBytes = r.uint64();
        break;
        case 4:
        m.writeCount = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ILinkInfo = {
  hostId?: Uint8Array;
  kind?: strims_debug_v1_LinkKind;
  metrics?: strims_debug_v1_ILinkMetrics;
  dialed?: boolean;
  mtu?: bigint;
  transmitPending?: bigint;
  inputBuffered?: bigint;
  writeBlocked?: boolean;
  bufferedAmount?: bigint;
}

export class LinkInfo {
  hostId: Uint8Array;
  kind: strims_debug_v1_LinkKind;
  metrics: strims_debug_v1_LinkMetrics | undefined;
  dialed: boolean;
  mtu: bigint;
  transmitPending: bigint;
  inputBuffered: bigint;
  writeBlocked: boolean;
  bufferedAmount: bigint;

  constructor(v?: ILinkInfo) {
    this.hostId = v?.hostId || new Uint8Array();
    this.kind = v?.kind || 0;
    this.metrics = v?.metrics && new strims_debug_v1_LinkMetrics(v.metrics);
    this.dialed = v?.dialed || false;
    this.mtu = v?.mtu || BigInt(0);
    this.transmitPending = v?.transmitPending || BigInt(0);
    this.inputBuffered = v?.inputBuffered || BigInt(0);
    this.writeBlocked = v?.writeBlocked || false;
    this.bufferedAmount = v?.bufferedAmount || BigInt(0);
  }

  static encode(m: LinkInfo, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.hostId.length) w.uint32(10).bytes(m.hostId);
    if (m.kind) w.uint32(16).uint32(m.kind);
    if (m.metrics) strims_debug_v1_LinkMetrics.encode(m.metrics, w.uint32(26).fork()).ldelim();
    if (m.dialed) w.uint32(32).bool(m.dialed);
    if (m.mtu) w.uint32(40).uint64(m.mtu);
    if (m.transmitPending) w.uint32(48).uint64(m.transmitPending);
    if (m.inputBuffered) w.uint32(56).uint64(m.inputBuffered);
    if (m.writeBlocked) w.uint32(64).bool(m.writeBlocked);
    if (m.bufferedAmount) w.uint32(72).uint64(m.bufferedAmount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkInfo {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LinkInfo();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.hostId = r.bytes();
        break;
        case 2:
        m.kind = r.uint32();
        break;
        case 3:
        m.metrics = strims_debug_v1_LinkMetrics.decode(r, r.uint32());
        break;
        case 4:
        m.dialed = r.bool();
        break;
        case 5:
        m.mtu = r.uint64();
        break;
        case 6:
        m.transmitPending = r.uint64();
        break;
        case 7:
        m.inputBuffered = r.uint64();
        break;
        case 8:
        m.writeBlocked = r.bool();
        break;
        case 9:
        m.bufferedAmount = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IRunnerMetrics = {
  swarms?: strims_debug_v1_ISwarmInfo[];
  peers?: strims_debug_v1_IRunnerPeerInfo[];
}

export class RunnerMetrics {
  swarms: strims_debug_v1_SwarmInfo[];
  peers: strims_debug_v1_RunnerPeerInfo[];

  constructor(v?: IRunnerMetrics) {
    this.swarms = v?.swarms ? v.swarms.map(v => new strims_debug_v1_SwarmInfo(v)) : [];
    this.peers = v?.peers ? v.peers.map(v => new strims_debug_v1_RunnerPeerInfo(v)) : [];
  }

  static encode(m: RunnerMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.swarms) strims_debug_v1_SwarmInfo.encode(v, w.uint32(10).fork()).ldelim();
    for (const v of m.peers) strims_debug_v1_RunnerPeerInfo.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): RunnerMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new RunnerMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarms.push(strims_debug_v1_SwarmInfo.decode(r, r.uint32()));
        break;
        case 2:
        m.peers.push(strims_debug_v1_RunnerPeerInfo.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISwarmInfo = {
  swarm?: bigint;
  id?: Uint8Array;
  label?: string;
  channels?: bigint;
  store?: strims_debug_v1_IStoreInfo;
  scheduler?: strims_debug_v1_ISwarmSchedulerStats;
  push?: strims_debug_v1_IPushStats;
  readers?: bigint;
  readersWaiting?: bigint;
  publishers?: bigint;
}

export class SwarmInfo {
  swarm: bigint;
  id: Uint8Array;
  label: string;
  channels: bigint;
  store: strims_debug_v1_StoreInfo | undefined;
  scheduler: strims_debug_v1_SwarmSchedulerStats | undefined;
  push: strims_debug_v1_PushStats | undefined;
  readers: bigint;
  readersWaiting: bigint;
  publishers: bigint;

  constructor(v?: ISwarmInfo) {
    this.swarm = v?.swarm || BigInt(0);
    this.id = v?.id || new Uint8Array();
    this.label = v?.label || "";
    this.channels = v?.channels || BigInt(0);
    this.store = v?.store && new strims_debug_v1_StoreInfo(v.store);
    this.scheduler = v?.scheduler && new strims_debug_v1_SwarmSchedulerStats(v.scheduler);
    this.push = v?.push && new strims_debug_v1_PushStats(v.push);
    this.readers = v?.readers || BigInt(0);
    this.readersWaiting = v?.readersWaiting || BigInt(0);
    this.publishers = v?.publishers || BigInt(0);
  }

  static encode(m: SwarmInfo, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarm) w.uint32(8).uint64(m.swarm);
    if (m.id.length) w.uint32(18).bytes(m.id);
    if (m.label.length) w.uint32(26).string(m.label);
    if (m.channels) w.uint32(32).uint64(m.channels);
    if (m.store) strims_debug_v1_StoreInfo.encode(m.store, w.uint32(42).fork()).ldelim();
    if (m.scheduler) strims_debug_v1_SwarmSchedulerStats.encode(m.scheduler, w.uint32(50).fork()).ldelim();
    if (m.push) strims_debug_v1_PushStats.encode(m.push, w.uint32(58).fork()).ldelim();
    if (m.readers) w.uint32(64).uint64(m.readers);
    if (m.readersWaiting) w.uint32(72).uint64(m.readersWaiting);
    if (m.publishers) w.uint32(80).uint64(m.publishers);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SwarmInfo {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SwarmInfo();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarm = r.uint64();
        break;
        case 2:
        m.id = r.bytes();
        break;
        case 3:
        m.label = r.string();
        break;
        case 4:
        m.channels = r.uint64();
        break;
        case 5:
        m.store = strims_debug_v1_StoreInfo.decode(r, r.uint32());
        break;
        case 6:
        m.scheduler = strims_debug_v1_SwarmSchedulerStats.decode(r, r.uint32());
        break;
        case 7:
        m.push = strims_debug_v1_PushStats.decode(r, r.uint32());
        break;
        case 8:
        m.readers = r.uint64();
        break;
        case 9:
        m.readersWaiting = r.uint64();
        break;
        case 10:
        m.publishers = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IStoreInfo = {
  ready?: boolean;
  tail?: bigint;
  next?: bigint;
  window?: bigint;
  readableSeq?: bigint;
}

export class StoreInfo {
  ready: boolean;
  tail: bigint;
  next: bigint;
  window: bigint;
  readableSeq: bigint;

  constructor(v?: IStoreInfo) {
    this.ready = v?.ready || false;
    this.tail = v?.tail || BigInt(0);
    this.next = v?.next || BigInt(0);
    this.window = v?.window || BigInt(0);
    this.readableSeq = v?.readableSeq || BigInt(0);
  }

  static encode(m: StoreInfo, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.ready) w.uint32(8).bool(m.ready);
    if (m.tail) w.uint32(16).uint64(m.tail);
    if (m.next) w.uint32(24).uint64(m.next);
    if (m.window) w.uint32(32).uint64(m.window);
    if (m.readableSeq) w.uint32(40).uint64(m.readableSeq);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StoreInfo {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new StoreInfo();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.ready = r.bool();
        break;
        case 2:
        m.tail = r.uint64();
        break;
        case 3:
        m.next = r.uint64();
        break;
        case 4:
        m.window = r.uint64();
        break;
        case 5:
        m.readableSeq = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISwarmSchedulerStats = {
  firstChunkSet?: boolean;
  haveBinMax?: bigint;
  peerMaxHaveBin?: bigint;
  wastedChunks?: bigint;
  binTimeNodes?: bigint;
  haveMapSize?: bigint;
  requestMapSize?: bigint;
  streamSources?: bigint;
  subscribers?: bigint;
  chunksSent?: bigint;
  pushedChunks?: bigint;
  distinctChunksSent?: bigint;
}

export class SwarmSchedulerStats {
  firstChunkSet: boolean;
  haveBinMax: bigint;
  peerMaxHaveBin: bigint;
  wastedChunks: bigint;
  binTimeNodes: bigint;
  haveMapSize: bigint;
  requestMapSize: bigint;
  streamSources: bigint;
  subscribers: bigint;
  chunksSent: bigint;
  pushedChunks: bigint;
  distinctChunksSent: bigint;

  constructor(v?: ISwarmSchedulerStats) {
    this.firstChunkSet = v?.firstChunkSet || false;
    this.haveBinMax = v?.haveBinMax || BigInt(0);
    this.peerMaxHaveBin = v?.peerMaxHaveBin || BigInt(0);
    this.wastedChunks = v?.wastedChunks || BigInt(0);
    this.binTimeNodes = v?.binTimeNodes || BigInt(0);
    this.haveMapSize = v?.haveMapSize || BigInt(0);
    this.requestMapSize = v?.requestMapSize || BigInt(0);
    this.streamSources = v?.streamSources || BigInt(0);
    this.subscribers = v?.subscribers || BigInt(0);
    this.chunksSent = v?.chunksSent || BigInt(0);
    this.pushedChunks = v?.pushedChunks || BigInt(0);
    this.distinctChunksSent = v?.distinctChunksSent || BigInt(0);
  }

  static encode(m: SwarmSchedulerStats, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.firstChunkSet) w.uint32(8).bool(m.firstChunkSet);
    if (m.haveBinMax) w.uint32(16).uint64(m.haveBinMax);
    if (m.peerMaxHaveBin) w.uint32(24).uint64(m.peerMaxHaveBin);
    if (m.wastedChunks) w.uint32(32).uint64(m.wastedChunks);
    if (m.binTimeNodes) w.uint32(40).uint64(m.binTimeNodes);
    if (m.haveMapSize) w.uint32(48).uint64(m.haveMapSize);
    if (m.requestMapSize) w.uint32(56).uint64(m.requestMapSize);
    if (m.streamSources) w.uint32(64).uint64(m.streamSources);
    if (m.subscribers) w.uint32(72).uint64(m.subscribers);
    if (m.chunksSent) w.uint32(80).uint64(m.chunksSent);
    if (m.pushedChunks) w.uint32(88).uint64(m.pushedChunks);
    if (m.distinctChunksSent) w.uint32(96).uint64(m.distinctChunksSent);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SwarmSchedulerStats {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SwarmSchedulerStats();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.firstChunkSet = r.bool();
        break;
        case 2:
        m.haveBinMax = r.uint64();
        break;
        case 3:
        m.peerMaxHaveBin = r.uint64();
        break;
        case 4:
        m.wastedChunks = r.uint64();
        break;
        case 5:
        m.binTimeNodes = r.uint64();
        break;
        case 6:
        m.haveMapSize = r.uint64();
        break;
        case 7:
        m.requestMapSize = r.uint64();
        break;
        case 8:
        m.streamSources = r.uint64();
        break;
        case 9:
        m.subscribers = r.uint64();
        break;
        case 10:
        m.chunksSent = r.uint64();
        break;
        case 11:
        m.pushedChunks = r.uint64();
        break;
        case 12:
        m.distinctChunksSent = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPushParent = {
  peer?: google_protobuf_IUInt64Value;
  lag?: google_protobuf_IInt64Value;
  deadline?: google_protobuf_IInt64Value;
}

export class PushParent {
  peer: google_protobuf_UInt64Value | undefined;
  lag: google_protobuf_Int64Value | undefined;
  deadline: google_protobuf_Int64Value | undefined;

  constructor(v?: IPushParent) {
    this.peer = v?.peer && new google_protobuf_UInt64Value(v.peer);
    this.lag = v?.lag && new google_protobuf_Int64Value(v.lag);
    this.deadline = v?.deadline && new google_protobuf_Int64Value(v.deadline);
  }

  static encode(m: PushParent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peer) google_protobuf_UInt64Value.encode(m.peer, w.uint32(10).fork()).ldelim();
    if (m.lag) google_protobuf_Int64Value.encode(m.lag, w.uint32(18).fork()).ldelim();
    if (m.deadline) google_protobuf_Int64Value.encode(m.deadline, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PushParent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PushParent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peer = google_protobuf_UInt64Value.decode(r, r.uint32());
        break;
        case 2:
        m.lag = google_protobuf_Int64Value.decode(r, r.uint32());
        break;
        case 3:
        m.deadline = google_protobuf_Int64Value.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPushStats = {
  mode?: strims_debug_v1_PushMode;
  parents?: strims_debug_v1_IPushParent[];
  uploadRate?: bigint;
  uploadBusyPermille?: number;
  interior?: google_protobuf_IUInt32Value;
  children?: bigint;
  expected?: bigint;
  hits?: bigint;
  misses?: bigint;
  late?: bigint;
  unsolicited?: bigint;
  requestsSent?: bigint;
  opens?: bigint;
  declined?: bigint;
  openTimeouts?: bigint;
  shed?: bigint;
  pruned?: bigint;
  switched?: bigint;
  accepted?: bigint;
  rejected?: bigint;
  shedsSent?: bigint;
  staleDrops?: bigint;
  suppressed?: bigint;
}

export class PushStats {
  mode: strims_debug_v1_PushMode;
  parents: strims_debug_v1_PushParent[];
  uploadRate: bigint;
  uploadBusyPermille: number;
  interior: google_protobuf_UInt32Value | undefined;
  children: bigint;
  expected: bigint;
  hits: bigint;
  misses: bigint;
  late: bigint;
  unsolicited: bigint;
  requestsSent: bigint;
  opens: bigint;
  declined: bigint;
  openTimeouts: bigint;
  shed: bigint;
  pruned: bigint;
  switched: bigint;
  accepted: bigint;
  rejected: bigint;
  shedsSent: bigint;
  staleDrops: bigint;
  suppressed: bigint;

  constructor(v?: IPushStats) {
    this.mode = v?.mode || 0;
    this.parents = v?.parents ? v.parents.map(v => new strims_debug_v1_PushParent(v)) : [];
    this.uploadRate = v?.uploadRate || BigInt(0);
    this.uploadBusyPermille = v?.uploadBusyPermille || 0;
    this.interior = v?.interior && new google_protobuf_UInt32Value(v.interior);
    this.children = v?.children || BigInt(0);
    this.expected = v?.expected || BigInt(0);
    this.hits = v?.hits || BigInt(0);
    this.misses = v?.misses || BigInt(0);
    this.late = v?.late || BigInt(0);
    this.unsolicited = v?.unsolicited || BigInt(0);
    this.requestsSent = v?.requestsSent || BigInt(0);
    this.opens = v?.opens || BigInt(0);
    this.declined = v?.declined || BigInt(0);
    this.openTimeouts = v?.openTimeouts || BigInt(0);
    this.shed = v?.shed || BigInt(0);
    this.pruned = v?.pruned || BigInt(0);
    this.switched = v?.switched || BigInt(0);
    this.accepted = v?.accepted || BigInt(0);
    this.rejected = v?.rejected || BigInt(0);
    this.shedsSent = v?.shedsSent || BigInt(0);
    this.staleDrops = v?.staleDrops || BigInt(0);
    this.suppressed = v?.suppressed || BigInt(0);
  }

  static encode(m: PushStats, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.mode) w.uint32(8).uint32(m.mode);
    for (const v of m.parents) strims_debug_v1_PushParent.encode(v, w.uint32(18).fork()).ldelim();
    if (m.uploadRate) w.uint32(24).uint64(m.uploadRate);
    if (m.uploadBusyPermille) w.uint32(32).uint32(m.uploadBusyPermille);
    if (m.interior) google_protobuf_UInt32Value.encode(m.interior, w.uint32(42).fork()).ldelim();
    if (m.children) w.uint32(48).uint64(m.children);
    if (m.expected) w.uint32(56).uint64(m.expected);
    if (m.hits) w.uint32(64).uint64(m.hits);
    if (m.misses) w.uint32(72).uint64(m.misses);
    if (m.late) w.uint32(80).uint64(m.late);
    if (m.unsolicited) w.uint32(88).uint64(m.unsolicited);
    if (m.requestsSent) w.uint32(96).uint64(m.requestsSent);
    if (m.opens) w.uint32(104).uint64(m.opens);
    if (m.declined) w.uint32(112).uint64(m.declined);
    if (m.openTimeouts) w.uint32(120).uint64(m.openTimeouts);
    if (m.shed) w.uint32(128).uint64(m.shed);
    if (m.pruned) w.uint32(136).uint64(m.pruned);
    if (m.switched) w.uint32(144).uint64(m.switched);
    if (m.accepted) w.uint32(152).uint64(m.accepted);
    if (m.rejected) w.uint32(160).uint64(m.rejected);
    if (m.shedsSent) w.uint32(168).uint64(m.shedsSent);
    if (m.staleDrops) w.uint32(176).uint64(m.staleDrops);
    if (m.suppressed) w.uint32(184).uint64(m.suppressed);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PushStats {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PushStats();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.mode = r.uint32();
        break;
        case 2:
        m.parents.push(strims_debug_v1_PushParent.decode(r, r.uint32()));
        break;
        case 3:
        m.uploadRate = r.uint64();
        break;
        case 4:
        m.uploadBusyPermille = r.uint32();
        break;
        case 5:
        m.interior = google_protobuf_UInt32Value.decode(r, r.uint32());
        break;
        case 6:
        m.children = r.uint64();
        break;
        case 7:
        m.expected = r.uint64();
        break;
        case 8:
        m.hits = r.uint64();
        break;
        case 9:
        m.misses = r.uint64();
        break;
        case 10:
        m.late = r.uint64();
        break;
        case 11:
        m.unsolicited = r.uint64();
        break;
        case 12:
        m.requestsSent = r.uint64();
        break;
        case 13:
        m.opens = r.uint64();
        break;
        case 14:
        m.declined = r.uint64();
        break;
        case 15:
        m.openTimeouts = r.uint64();
        break;
        case 16:
        m.shed = r.uint64();
        break;
        case 17:
        m.pruned = r.uint64();
        break;
        case 18:
        m.switched = r.uint64();
        break;
        case 19:
        m.accepted = r.uint64();
        break;
        case 20:
        m.rejected = r.uint64();
        break;
        case 21:
        m.shedsSent = r.uint64();
        break;
        case 22:
        m.staleDrops = r.uint64();
        break;
        case 23:
        m.suppressed = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IRunnerPeerInfo = {
  hostId?: Uint8Array;
  qosWeight?: google_protobuf_IUInt64Value;
  queues?: strims_debug_v1_IPeerStats;
  metrics?: strims_debug_v1_IMetricsSnapshot;
  writer?: strims_debug_v1_IWriterInfo;
  channels?: strims_debug_v1_IChannelInfo[];
}

export class RunnerPeerInfo {
  hostId: Uint8Array;
  qosWeight: google_protobuf_UInt64Value | undefined;
  queues: strims_debug_v1_PeerStats | undefined;
  metrics: strims_debug_v1_MetricsSnapshot | undefined;
  writer: strims_debug_v1_WriterInfo | undefined;
  channels: strims_debug_v1_ChannelInfo[];

  constructor(v?: IRunnerPeerInfo) {
    this.hostId = v?.hostId || new Uint8Array();
    this.qosWeight = v?.qosWeight && new google_protobuf_UInt64Value(v.qosWeight);
    this.queues = v?.queues && new strims_debug_v1_PeerStats(v.queues);
    this.metrics = v?.metrics && new strims_debug_v1_MetricsSnapshot(v.metrics);
    this.writer = v?.writer && new strims_debug_v1_WriterInfo(v.writer);
    this.channels = v?.channels ? v.channels.map(v => new strims_debug_v1_ChannelInfo(v)) : [];
  }

  static encode(m: RunnerPeerInfo, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.hostId.length) w.uint32(10).bytes(m.hostId);
    if (m.qosWeight) google_protobuf_UInt64Value.encode(m.qosWeight, w.uint32(18).fork()).ldelim();
    if (m.queues) strims_debug_v1_PeerStats.encode(m.queues, w.uint32(26).fork()).ldelim();
    if (m.metrics) strims_debug_v1_MetricsSnapshot.encode(m.metrics, w.uint32(34).fork()).ldelim();
    if (m.writer) strims_debug_v1_WriterInfo.encode(m.writer, w.uint32(42).fork()).ldelim();
    for (const v of m.channels) strims_debug_v1_ChannelInfo.encode(v, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): RunnerPeerInfo {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new RunnerPeerInfo();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.hostId = r.bytes();
        break;
        case 2:
        m.qosWeight = google_protobuf_UInt64Value.decode(r, r.uint32());
        break;
        case 3:
        m.queues = strims_debug_v1_PeerStats.decode(r, r.uint32());
        break;
        case 4:
        m.metrics = strims_debug_v1_MetricsSnapshot.decode(r, r.uint32());
        break;
        case 5:
        m.writer = strims_debug_v1_WriterInfo.decode(r, r.uint32());
        break;
        case 6:
        m.channels.push(strims_debug_v1_ChannelInfo.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPeerStats = {
  wantsTransmit?: boolean;
  queuedChannels?: boolean;
  queuedData?: bigint[];
  queueRing?: bigint[];
  dropped?: bigint;
}

export class PeerStats {
  wantsTransmit: boolean;
  queuedChannels: boolean;
  queuedData: bigint[];
  queueRing: bigint[];
  dropped: bigint;

  constructor(v?: IPeerStats) {
    this.wantsTransmit = v?.wantsTransmit || false;
    this.queuedChannels = v?.queuedChannels || false;
    this.queuedData = v?.queuedData ? v.queuedData : [];
    this.queueRing = v?.queueRing ? v.queueRing : [];
    this.dropped = v?.dropped || BigInt(0);
  }

  static encode(m: PeerStats, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.wantsTransmit) w.uint32(8).bool(m.wantsTransmit);
    if (m.queuedChannels) w.uint32(16).bool(m.queuedChannels);
    m.queuedData.reduce((w, v) => w.uint64(v), w.uint32(26).fork()).ldelim();
    m.queueRing.reduce((w, v) => w.uint64(v), w.uint32(34).fork()).ldelim();
    if (m.dropped) w.uint32(40).uint64(m.dropped);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerStats {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerStats();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.wantsTransmit = r.bool();
        break;
        case 2:
        m.queuedChannels = r.bool();
        break;
        case 3:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.queuedData.push(r.uint64());
        break;
        case 4:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.queueRing.push(r.uint64());
        break;
        case 5:
        m.dropped = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ITransferMetricsSnapshot = {
  count?: bigint;
  rate?: bigint;
}

export class TransferMetricsSnapshot {
  count: bigint;
  rate: bigint;

  constructor(v?: ITransferMetricsSnapshot) {
    this.count = v?.count || BigInt(0);
    this.rate = v?.rate || BigInt(0);
  }

  static encode(m: TransferMetricsSnapshot, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.count) w.uint32(8).uint64(m.count);
    if (m.rate) w.uint32(16).uint64(m.rate);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferMetricsSnapshot {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TransferMetricsSnapshot();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.count = r.uint64();
        break;
        case 2:
        m.rate = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMetricsSnapshot = {
  read?: strims_debug_v1_ITransferMetricsSnapshot;
  write?: strims_debug_v1_ITransferMetricsSnapshot;
}

export class MetricsSnapshot {
  read: strims_debug_v1_TransferMetricsSnapshot | undefined;
  write: strims_debug_v1_TransferMetricsSnapshot | undefined;

  constructor(v?: IMetricsSnapshot) {
    this.read = v?.read && new strims_debug_v1_TransferMetricsSnapshot(v.read);
    this.write = v?.write && new strims_debug_v1_TransferMetricsSnapshot(v.write);
  }

  static encode(m: MetricsSnapshot, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.read) strims_debug_v1_TransferMetricsSnapshot.encode(m.read, w.uint32(10).fork()).ldelim();
    if (m.write) strims_debug_v1_TransferMetricsSnapshot.encode(m.write, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MetricsSnapshot {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MetricsSnapshot();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.read = strims_debug_v1_TransferMetricsSnapshot.decode(r, r.uint32());
        break;
        case 2:
        m.write = strims_debug_v1_TransferMetricsSnapshot.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWriterInfo = {
  weight?: bigint;
  backlogged?: boolean;
  blocked?: boolean;
}

export class WriterInfo {
  weight: bigint;
  backlogged: boolean;
  blocked: boolean;

  constructor(v?: IWriterInfo) {
    this.weight = v?.weight || BigInt(0);
    this.backlogged = v?.backlogged || false;
    this.blocked = v?.blocked || false;
  }

  static encode(m: WriterInfo, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.weight) w.uint32(8).uint64(m.weight);
    if (m.backlogged) w.uint32(16).bool(m.backlogged);
    if (m.blocked) w.uint32(24).bool(m.blocked);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WriterInfo {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WriterInfo();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.weight = r.uint64();
        break;
        case 2:
        m.backlogged = r.bool();
        break;
        case 3:
        m.blocked = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IChannelInfo = {
  swarm?: bigint;
  channel?: bigint;
  peerChannel?: bigint;
  metrics?: strims_debug_v1_IMetricsSnapshot;
  stats?: strims_debug_v1_IChannelStats;
}

export class ChannelInfo {
  swarm: bigint;
  channel: bigint;
  peerChannel: bigint;
  metrics: strims_debug_v1_MetricsSnapshot | undefined;
  stats: strims_debug_v1_ChannelStats | undefined;

  constructor(v?: IChannelInfo) {
    this.swarm = v?.swarm || BigInt(0);
    this.channel = v?.channel || BigInt(0);
    this.peerChannel = v?.peerChannel || BigInt(0);
    this.metrics = v?.metrics && new strims_debug_v1_MetricsSnapshot(v.metrics);
    this.stats = v?.stats && new strims_debug_v1_ChannelStats(v.stats);
  }

  static encode(m: ChannelInfo, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarm) w.uint32(8).uint64(m.swarm);
    if (m.channel) w.uint32(16).uint64(m.channel);
    if (m.peerChannel) w.uint32(24).uint64(m.peerChannel);
    if (m.metrics) strims_debug_v1_MetricsSnapshot.encode(m.metrics, w.uint32(34).fork()).ldelim();
    if (m.stats) strims_debug_v1_ChannelStats.encode(m.stats, w.uint32(42).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChannelInfo {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChannelInfo();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarm = r.uint64();
        break;
        case 2:
        m.channel = r.uint64();
        break;
        case 3:
        m.peerChannel = r.uint64();
        break;
        case 4:
        m.metrics = strims_debug_v1_MetricsSnapshot.decode(r, r.uint32());
        break;
        case 5:
        m.stats = strims_debug_v1_ChannelStats.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IChannelStats = {
  scheduler?: strims_debug_v1_IChannelSchedulerStats;
  inbound?: strims_debug_v1_IChannelMetrics;
  outbound?: strims_debug_v1_IChannelMetrics;
}

export class ChannelStats {
  scheduler: strims_debug_v1_ChannelSchedulerStats | undefined;
  inbound: strims_debug_v1_ChannelMetrics | undefined;
  outbound: strims_debug_v1_ChannelMetrics | undefined;

  constructor(v?: IChannelStats) {
    this.scheduler = v?.scheduler && new strims_debug_v1_ChannelSchedulerStats(v.scheduler);
    this.inbound = v?.inbound && new strims_debug_v1_ChannelMetrics(v.inbound);
    this.outbound = v?.outbound && new strims_debug_v1_ChannelMetrics(v.outbound);
  }

  static encode(m: ChannelStats, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.scheduler) strims_debug_v1_ChannelSchedulerStats.encode(m.scheduler, w.uint32(10).fork()).ldelim();
    if (m.inbound) strims_debug_v1_ChannelMetrics.encode(m.inbound, w.uint32(18).fork()).ldelim();
    if (m.outbound) strims_debug_v1_ChannelMetrics.encode(m.outbound, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChannelStats {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChannelStats();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.scheduler = strims_debug_v1_ChannelSchedulerStats.decode(r, r.uint32());
        break;
        case 2:
        m.inbound = strims_debug_v1_ChannelMetrics.decode(r, r.uint32());
        break;
        case 3:
        m.outbound = strims_debug_v1_ChannelMetrics.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IChannelSchedulerStats = {
  handshakeReceived?: boolean;
  choked?: boolean;
  controller?: strims_debug_v1_RequestControllerKind;
  cwnd?: bigint;
  flightSize?: bigint;
  cto?: bigint;
  rttMean?: bigint;
  minRtt?: bigint;
  deliveryRate?: bigint;
  pendingRequests?: bigint;
  requestTimeNodes?: bigint;
  extraMessages?: bigint;
  peerMaxHaveBin?: bigint;
  peerHaveMapSize?: bigint;
  haveMapSize?: bigint;
  cancelMapSize?: bigint;
}

export class ChannelSchedulerStats {
  handshakeReceived: boolean;
  choked: boolean;
  controller: strims_debug_v1_RequestControllerKind;
  cwnd: bigint;
  flightSize: bigint;
  cto: bigint;
  rttMean: bigint;
  minRtt: bigint;
  deliveryRate: bigint;
  pendingRequests: bigint;
  requestTimeNodes: bigint;
  extraMessages: bigint;
  peerMaxHaveBin: bigint;
  peerHaveMapSize: bigint;
  haveMapSize: bigint;
  cancelMapSize: bigint;

  constructor(v?: IChannelSchedulerStats) {
    this.handshakeReceived = v?.handshakeReceived || false;
    this.choked = v?.choked || false;
    this.controller = v?.controller || 0;
    this.cwnd = v?.cwnd || BigInt(0);
    this.flightSize = v?.flightSize || BigInt(0);
    this.cto = v?.cto || BigInt(0);
    this.rttMean = v?.rttMean || BigInt(0);
    this.minRtt = v?.minRtt || BigInt(0);
    this.deliveryRate = v?.deliveryRate || BigInt(0);
    this.pendingRequests = v?.pendingRequests || BigInt(0);
    this.requestTimeNodes = v?.requestTimeNodes || BigInt(0);
    this.extraMessages = v?.extraMessages || BigInt(0);
    this.peerMaxHaveBin = v?.peerMaxHaveBin || BigInt(0);
    this.peerHaveMapSize = v?.peerHaveMapSize || BigInt(0);
    this.haveMapSize = v?.haveMapSize || BigInt(0);
    this.cancelMapSize = v?.cancelMapSize || BigInt(0);
  }

  static encode(m: ChannelSchedulerStats, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.handshakeReceived) w.uint32(8).bool(m.handshakeReceived);
    if (m.choked) w.uint32(16).bool(m.choked);
    if (m.controller) w.uint32(24).uint32(m.controller);
    if (m.cwnd) w.uint32(32).int64(m.cwnd);
    if (m.flightSize) w.uint32(40).int64(m.flightSize);
    if (m.cto) w.uint32(48).int64(m.cto);
    if (m.rttMean) w.uint32(56).int64(m.rttMean);
    if (m.minRtt) w.uint32(64).int64(m.minRtt);
    if (m.deliveryRate) w.uint32(72).uint64(m.deliveryRate);
    if (m.pendingRequests) w.uint32(80).uint64(m.pendingRequests);
    if (m.requestTimeNodes) w.uint32(88).uint64(m.requestTimeNodes);
    if (m.extraMessages) w.uint32(96).uint64(m.extraMessages);
    if (m.peerMaxHaveBin) w.uint32(104).uint64(m.peerMaxHaveBin);
    if (m.peerHaveMapSize) w.uint32(112).uint64(m.peerHaveMapSize);
    if (m.haveMapSize) w.uint32(120).uint64(m.haveMapSize);
    if (m.cancelMapSize) w.uint32(128).uint64(m.cancelMapSize);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChannelSchedulerStats {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChannelSchedulerStats();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.handshakeReceived = r.bool();
        break;
        case 2:
        m.choked = r.bool();
        break;
        case 3:
        m.controller = r.uint32();
        break;
        case 4:
        m.cwnd = r.int64();
        break;
        case 5:
        m.flightSize = r.int64();
        break;
        case 6:
        m.cto = r.int64();
        break;
        case 7:
        m.rttMean = r.int64();
        break;
        case 8:
        m.minRtt = r.int64();
        break;
        case 9:
        m.deliveryRate = r.uint64();
        break;
        case 10:
        m.pendingRequests = r.uint64();
        break;
        case 11:
        m.requestTimeNodes = r.uint64();
        break;
        case 12:
        m.extraMessages = r.uint64();
        break;
        case 13:
        m.peerMaxHaveBin = r.uint64();
        break;
        case 14:
        m.peerHaveMapSize = r.uint64();
        break;
        case 15:
        m.haveMapSize = r.uint64();
        break;
        case 16:
        m.cancelMapSize = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IChannelMetrics = {
  handshake?: bigint;
  restart?: bigint;
  data?: bigint;
  chunk?: bigint;
  integrity?: bigint;
  signedIntegrity?: bigint;
  ack?: bigint;
  have?: bigint;
  request?: bigint;
  cancel?: bigint;
  choke?: bigint;
  unchoke?: bigint;
  ping?: bigint;
  pong?: bigint;
  streamRequest?: bigint;
  streamCancel?: bigint;
  streamOpen?: bigint;
  streamClose?: bigint;
  dataBytes?: bigint;
  overheadBytes?: bigint;
  invalidData?: bigint;
  invalidChunk?: bigint;
  invalidBytes?: bigint;
  rejected?: bigint;
  staleStreamOpen?: bigint;
  staleData?: bigint;
  announce?: bigint;
}

export class ChannelMetrics {
  handshake: bigint;
  restart: bigint;
  data: bigint;
  chunk: bigint;
  integrity: bigint;
  signedIntegrity: bigint;
  ack: bigint;
  have: bigint;
  request: bigint;
  cancel: bigint;
  choke: bigint;
  unchoke: bigint;
  ping: bigint;
  pong: bigint;
  streamRequest: bigint;
  streamCancel: bigint;
  streamOpen: bigint;
  streamClose: bigint;
  dataBytes: bigint;
  overheadBytes: bigint;
  invalidData: bigint;
  invalidChunk: bigint;
  invalidBytes: bigint;
  rejected: bigint;
  staleStreamOpen: bigint;
  staleData: bigint;
  announce: bigint;

  constructor(v?: IChannelMetrics) {
    this.handshake = v?.handshake || BigInt(0);
    this.restart = v?.restart || BigInt(0);
    this.data = v?.data || BigInt(0);
    this.chunk = v?.chunk || BigInt(0);
    this.integrity = v?.integrity || BigInt(0);
    this.signedIntegrity = v?.signedIntegrity || BigInt(0);
    this.ack = v?.ack || BigInt(0);
    this.have = v?.have || BigInt(0);
    this.request = v?.request || BigInt(0);
    this.cancel = v?.cancel || BigInt(0);
    this.choke = v?.choke || BigInt(0);
    this.unchoke = v?.unchoke || BigInt(0);
    this.ping = v?.ping || BigInt(0);
    this.pong = v?.pong || BigInt(0);
    this.streamRequest = v?.streamRequest || BigInt(0);
    this.streamCancel = v?.streamCancel || BigInt(0);
    this.streamOpen = v?.streamOpen || BigInt(0);
    this.streamClose = v?.streamClose || BigInt(0);
    this.dataBytes = v?.dataBytes || BigInt(0);
    this.overheadBytes = v?.overheadBytes || BigInt(0);
    this.invalidData = v?.invalidData || BigInt(0);
    this.invalidChunk = v?.invalidChunk || BigInt(0);
    this.invalidBytes = v?.invalidBytes || BigInt(0);
    this.rejected = v?.rejected || BigInt(0);
    this.staleStreamOpen = v?.staleStreamOpen || BigInt(0);
    this.staleData = v?.staleData || BigInt(0);
    this.announce = v?.announce || BigInt(0);
  }

  static encode(m: ChannelMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.handshake) w.uint32(8).uint64(m.handshake);
    if (m.restart) w.uint32(16).uint64(m.restart);
    if (m.data) w.uint32(24).uint64(m.data);
    if (m.chunk) w.uint32(32).uint64(m.chunk);
    if (m.integrity) w.uint32(40).uint64(m.integrity);
    if (m.signedIntegrity) w.uint32(48).uint64(m.signedIntegrity);
    if (m.ack) w.uint32(56).uint64(m.ack);
    if (m.have) w.uint32(64).uint64(m.have);
    if (m.request) w.uint32(72).uint64(m.request);
    if (m.cancel) w.uint32(80).uint64(m.cancel);
    if (m.choke) w.uint32(88).uint64(m.choke);
    if (m.unchoke) w.uint32(96).uint64(m.unchoke);
    if (m.ping) w.uint32(104).uint64(m.ping);
    if (m.pong) w.uint32(112).uint64(m.pong);
    if (m.streamRequest) w.uint32(120).uint64(m.streamRequest);
    if (m.streamCancel) w.uint32(128).uint64(m.streamCancel);
    if (m.streamOpen) w.uint32(136).uint64(m.streamOpen);
    if (m.streamClose) w.uint32(144).uint64(m.streamClose);
    if (m.dataBytes) w.uint32(152).uint64(m.dataBytes);
    if (m.overheadBytes) w.uint32(160).uint64(m.overheadBytes);
    if (m.invalidData) w.uint32(168).uint64(m.invalidData);
    if (m.invalidChunk) w.uint32(176).uint64(m.invalidChunk);
    if (m.invalidBytes) w.uint32(184).uint64(m.invalidBytes);
    if (m.rejected) w.uint32(192).uint64(m.rejected);
    if (m.staleStreamOpen) w.uint32(200).uint64(m.staleStreamOpen);
    if (m.staleData) w.uint32(208).uint64(m.staleData);
    if (m.announce) w.uint32(216).uint64(m.announce);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChannelMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChannelMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.handshake = r.uint64();
        break;
        case 2:
        m.restart = r.uint64();
        break;
        case 3:
        m.data = r.uint64();
        break;
        case 4:
        m.chunk = r.uint64();
        break;
        case 5:
        m.integrity = r.uint64();
        break;
        case 6:
        m.signedIntegrity = r.uint64();
        break;
        case 7:
        m.ack = r.uint64();
        break;
        case 8:
        m.have = r.uint64();
        break;
        case 9:
        m.request = r.uint64();
        break;
        case 10:
        m.cancel = r.uint64();
        break;
        case 11:
        m.choke = r.uint64();
        break;
        case 12:
        m.unchoke = r.uint64();
        break;
        case 13:
        m.ping = r.uint64();
        break;
        case 14:
        m.pong = r.uint64();
        break;
        case 15:
        m.streamRequest = r.uint64();
        break;
        case 16:
        m.streamCancel = r.uint64();
        break;
        case 17:
        m.streamOpen = r.uint64();
        break;
        case 18:
        m.streamClose = r.uint64();
        break;
        case 19:
        m.dataBytes = r.uint64();
        break;
        case 20:
        m.overheadBytes = r.uint64();
        break;
        case 21:
        m.invalidData = r.uint64();
        break;
        case 22:
        m.invalidChunk = r.uint64();
        break;
        case 23:
        m.invalidBytes = r.uint64();
        break;
        case 24:
        m.rejected = r.uint64();
        break;
        case 25:
        m.staleStreamOpen = r.uint64();
        break;
        case 26:
        m.staleData = r.uint64();
        break;
        case 27:
        m.announce = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetworkInfo = {
  network?: bigint;
  links?: bigint;
  routedLinks?: bigint;
  queued?: bigint;
  topology?: bigint;
}

export class NetworkInfo {
  network: bigint;
  links: bigint;
  routedLinks: bigint;
  queued: bigint;
  topology: bigint;

  constructor(v?: INetworkInfo) {
    this.network = v?.network || BigInt(0);
    this.links = v?.links || BigInt(0);
    this.routedLinks = v?.routedLinks || BigInt(0);
    this.queued = v?.queued || BigInt(0);
    this.topology = v?.topology || BigInt(0);
  }

  static encode(m: NetworkInfo, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) w.uint32(8).uint64(m.network);
    if (m.links) w.uint32(16).uint64(m.links);
    if (m.routedLinks) w.uint32(24).uint64(m.routedLinks);
    if (m.queued) w.uint32(32).uint64(m.queued);
    if (m.topology) w.uint32(40).uint64(m.topology);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkInfo {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkInfo();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = r.uint64();
        break;
        case 2:
        m.links = r.uint64();
        break;
        case 3:
        m.routedLinks = r.uint64();
        break;
        case 4:
        m.queued = r.uint64();
        break;
        case 5:
        m.topology = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetworkMetrics = {
  runs?: bigint;
  runsCompleted?: bigint;
  runsFailed?: bigint;
  cuckooRetries?: bigint;
  runsPaced?: bigint;
  linksPaired?: bigint;
  linksOpened?: bigint;
  bindingsRefused?: bigint;
  brokerBytesRead?: bigint;
  brokerBytesWritten?: bigint;
  callErrors?: bigint;
  hostErrors?: bigint;
}

export class NetworkMetrics {
  runs: bigint;
  runsCompleted: bigint;
  runsFailed: bigint;
  cuckooRetries: bigint;
  runsPaced: bigint;
  linksPaired: bigint;
  linksOpened: bigint;
  bindingsRefused: bigint;
  brokerBytesRead: bigint;
  brokerBytesWritten: bigint;
  callErrors: bigint;
  hostErrors: bigint;

  constructor(v?: INetworkMetrics) {
    this.runs = v?.runs || BigInt(0);
    this.runsCompleted = v?.runsCompleted || BigInt(0);
    this.runsFailed = v?.runsFailed || BigInt(0);
    this.cuckooRetries = v?.cuckooRetries || BigInt(0);
    this.runsPaced = v?.runsPaced || BigInt(0);
    this.linksPaired = v?.linksPaired || BigInt(0);
    this.linksOpened = v?.linksOpened || BigInt(0);
    this.bindingsRefused = v?.bindingsRefused || BigInt(0);
    this.brokerBytesRead = v?.brokerBytesRead || BigInt(0);
    this.brokerBytesWritten = v?.brokerBytesWritten || BigInt(0);
    this.callErrors = v?.callErrors || BigInt(0);
    this.hostErrors = v?.hostErrors || BigInt(0);
  }

  static encode(m: NetworkMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.runs) w.uint32(8).uint64(m.runs);
    if (m.runsCompleted) w.uint32(16).uint64(m.runsCompleted);
    if (m.runsFailed) w.uint32(24).uint64(m.runsFailed);
    if (m.cuckooRetries) w.uint32(32).uint64(m.cuckooRetries);
    if (m.runsPaced) w.uint32(40).uint64(m.runsPaced);
    if (m.linksPaired) w.uint32(48).uint64(m.linksPaired);
    if (m.linksOpened) w.uint32(56).uint64(m.linksOpened);
    if (m.bindingsRefused) w.uint32(64).uint64(m.bindingsRefused);
    if (m.brokerBytesRead) w.uint32(72).uint64(m.brokerBytesRead);
    if (m.brokerBytesWritten) w.uint32(80).uint64(m.brokerBytesWritten);
    if (m.callErrors) w.uint32(88).uint64(m.callErrors);
    if (m.hostErrors) w.uint32(96).uint64(m.hostErrors);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.runs = r.uint64();
        break;
        case 2:
        m.runsCompleted = r.uint64();
        break;
        case 3:
        m.runsFailed = r.uint64();
        break;
        case 4:
        m.cuckooRetries = r.uint64();
        break;
        case 5:
        m.runsPaced = r.uint64();
        break;
        case 6:
        m.linksPaired = r.uint64();
        break;
        case 7:
        m.linksOpened = r.uint64();
        break;
        case 8:
        m.bindingsRefused = r.uint64();
        break;
        case 9:
        m.brokerBytesRead = r.uint64();
        break;
        case 10:
        m.brokerBytesWritten = r.uint64();
        break;
        case 11:
        m.callErrors = r.uint64();
        break;
        case 12:
        m.hostErrors = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ITransferMetrics = {
  announcesSent?: bigint;
  announcesAccepted?: bigint;
  announcesReceived?: bigint;
  announcesAnswered?: bigint;
  callErrors?: bigint;
  runs?: bigint;
  stops?: bigint;
  runErrors?: bigint;
  closesSent?: bigint;
  closesReceived?: bigint;
  indexPublishes?: bigint;
  searches?: bigint;
  searchErrors?: bigint;
  connects?: bigint;
  hostErrors?: bigint;
}

export class TransferMetrics {
  announcesSent: bigint;
  announcesAccepted: bigint;
  announcesReceived: bigint;
  announcesAnswered: bigint;
  callErrors: bigint;
  runs: bigint;
  stops: bigint;
  runErrors: bigint;
  closesSent: bigint;
  closesReceived: bigint;
  indexPublishes: bigint;
  searches: bigint;
  searchErrors: bigint;
  connects: bigint;
  hostErrors: bigint;

  constructor(v?: ITransferMetrics) {
    this.announcesSent = v?.announcesSent || BigInt(0);
    this.announcesAccepted = v?.announcesAccepted || BigInt(0);
    this.announcesReceived = v?.announcesReceived || BigInt(0);
    this.announcesAnswered = v?.announcesAnswered || BigInt(0);
    this.callErrors = v?.callErrors || BigInt(0);
    this.runs = v?.runs || BigInt(0);
    this.stops = v?.stops || BigInt(0);
    this.runErrors = v?.runErrors || BigInt(0);
    this.closesSent = v?.closesSent || BigInt(0);
    this.closesReceived = v?.closesReceived || BigInt(0);
    this.indexPublishes = v?.indexPublishes || BigInt(0);
    this.searches = v?.searches || BigInt(0);
    this.searchErrors = v?.searchErrors || BigInt(0);
    this.connects = v?.connects || BigInt(0);
    this.hostErrors = v?.hostErrors || BigInt(0);
  }

  static encode(m: TransferMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.announcesSent) w.uint32(8).uint64(m.announcesSent);
    if (m.announcesAccepted) w.uint32(16).uint64(m.announcesAccepted);
    if (m.announcesReceived) w.uint32(24).uint64(m.announcesReceived);
    if (m.announcesAnswered) w.uint32(32).uint64(m.announcesAnswered);
    if (m.callErrors) w.uint32(40).uint64(m.callErrors);
    if (m.runs) w.uint32(48).uint64(m.runs);
    if (m.stops) w.uint32(56).uint64(m.stops);
    if (m.runErrors) w.uint32(64).uint64(m.runErrors);
    if (m.closesSent) w.uint32(72).uint64(m.closesSent);
    if (m.closesReceived) w.uint32(80).uint64(m.closesReceived);
    if (m.indexPublishes) w.uint32(88).uint64(m.indexPublishes);
    if (m.searches) w.uint32(96).uint64(m.searches);
    if (m.searchErrors) w.uint32(104).uint64(m.searchErrors);
    if (m.connects) w.uint32(112).uint64(m.connects);
    if (m.hostErrors) w.uint32(120).uint64(m.hostErrors);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TransferMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TransferMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.announcesSent = r.uint64();
        break;
        case 2:
        m.announcesAccepted = r.uint64();
        break;
        case 3:
        m.announcesReceived = r.uint64();
        break;
        case 4:
        m.announcesAnswered = r.uint64();
        break;
        case 5:
        m.callErrors = r.uint64();
        break;
        case 6:
        m.runs = r.uint64();
        break;
        case 7:
        m.stops = r.uint64();
        break;
        case 8:
        m.runErrors = r.uint64();
        break;
        case 9:
        m.closesSent = r.uint64();
        break;
        case 10:
        m.closesReceived = r.uint64();
        break;
        case 11:
        m.indexPublishes = r.uint64();
        break;
        case 12:
        m.searches = r.uint64();
        break;
        case 13:
        m.searchErrors = r.uint64();
        break;
        case 14:
        m.connects = r.uint64();
        break;
        case 15:
        m.hostErrors = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IReplicationMetrics = {
  leases?: bigint;
  opens?: bigint;
  bootstrapsSent?: bigint;
  bootstrapsReceived?: bigint;
  eventsSent?: bigint;
  eventsReceived?: bigint;
  syncsSent?: bigint;
  syncsReceived?: bigint;
  logsSent?: bigint;
  logsReceived?: bigint;
  refused?: bigint;
  peerErrors?: bigint;
  storeErrors?: bigint;
  gcRuns?: bigint;
  logsCollected?: bigint;
  publishes?: bigint;
  lookups?: bigint;
  connects?: bigint;
}

export class ReplicationMetrics {
  leases: bigint;
  opens: bigint;
  bootstrapsSent: bigint;
  bootstrapsReceived: bigint;
  eventsSent: bigint;
  eventsReceived: bigint;
  syncsSent: bigint;
  syncsReceived: bigint;
  logsSent: bigint;
  logsReceived: bigint;
  refused: bigint;
  peerErrors: bigint;
  storeErrors: bigint;
  gcRuns: bigint;
  logsCollected: bigint;
  publishes: bigint;
  lookups: bigint;
  connects: bigint;

  constructor(v?: IReplicationMetrics) {
    this.leases = v?.leases || BigInt(0);
    this.opens = v?.opens || BigInt(0);
    this.bootstrapsSent = v?.bootstrapsSent || BigInt(0);
    this.bootstrapsReceived = v?.bootstrapsReceived || BigInt(0);
    this.eventsSent = v?.eventsSent || BigInt(0);
    this.eventsReceived = v?.eventsReceived || BigInt(0);
    this.syncsSent = v?.syncsSent || BigInt(0);
    this.syncsReceived = v?.syncsReceived || BigInt(0);
    this.logsSent = v?.logsSent || BigInt(0);
    this.logsReceived = v?.logsReceived || BigInt(0);
    this.refused = v?.refused || BigInt(0);
    this.peerErrors = v?.peerErrors || BigInt(0);
    this.storeErrors = v?.storeErrors || BigInt(0);
    this.gcRuns = v?.gcRuns || BigInt(0);
    this.logsCollected = v?.logsCollected || BigInt(0);
    this.publishes = v?.publishes || BigInt(0);
    this.lookups = v?.lookups || BigInt(0);
    this.connects = v?.connects || BigInt(0);
  }

  static encode(m: ReplicationMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.leases) w.uint32(8).uint64(m.leases);
    if (m.opens) w.uint32(16).uint64(m.opens);
    if (m.bootstrapsSent) w.uint32(24).uint64(m.bootstrapsSent);
    if (m.bootstrapsReceived) w.uint32(32).uint64(m.bootstrapsReceived);
    if (m.eventsSent) w.uint32(40).uint64(m.eventsSent);
    if (m.eventsReceived) w.uint32(48).uint64(m.eventsReceived);
    if (m.syncsSent) w.uint32(56).uint64(m.syncsSent);
    if (m.syncsReceived) w.uint32(64).uint64(m.syncsReceived);
    if (m.logsSent) w.uint32(72).uint64(m.logsSent);
    if (m.logsReceived) w.uint32(80).uint64(m.logsReceived);
    if (m.refused) w.uint32(88).uint64(m.refused);
    if (m.peerErrors) w.uint32(96).uint64(m.peerErrors);
    if (m.storeErrors) w.uint32(104).uint64(m.storeErrors);
    if (m.gcRuns) w.uint32(112).uint64(m.gcRuns);
    if (m.logsCollected) w.uint32(120).uint64(m.logsCollected);
    if (m.publishes) w.uint32(128).uint64(m.publishes);
    if (m.lookups) w.uint32(136).uint64(m.lookups);
    if (m.connects) w.uint32(144).uint64(m.connects);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ReplicationMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ReplicationMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.leases = r.uint64();
        break;
        case 2:
        m.opens = r.uint64();
        break;
        case 3:
        m.bootstrapsSent = r.uint64();
        break;
        case 4:
        m.bootstrapsReceived = r.uint64();
        break;
        case 5:
        m.eventsSent = r.uint64();
        break;
        case 6:
        m.eventsReceived = r.uint64();
        break;
        case 7:
        m.syncsSent = r.uint64();
        break;
        case 8:
        m.syncsReceived = r.uint64();
        break;
        case 9:
        m.logsSent = r.uint64();
        break;
        case 10:
        m.logsReceived = r.uint64();
        break;
        case 11:
        m.refused = r.uint64();
        break;
        case 12:
        m.peerErrors = r.uint64();
        break;
        case 13:
        m.storeErrors = r.uint64();
        break;
        case 14:
        m.gcRuns = r.uint64();
        break;
        case 15:
        m.logsCollected = r.uint64();
        break;
        case 16:
        m.publishes = r.uint64();
        break;
        case 17:
        m.lookups = r.uint64();
        break;
        case 18:
        m.connects = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDirectoryNetworkMetrics = {
  networkKey?: Uint8Array;
  metrics?: strims_debug_v1_IDirectoryMetrics;
}

export class DirectoryNetworkMetrics {
  networkKey: Uint8Array;
  metrics: strims_debug_v1_DirectoryMetrics | undefined;

  constructor(v?: IDirectoryNetworkMetrics) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.metrics = v?.metrics && new strims_debug_v1_DirectoryMetrics(v.metrics);
  }

  static encode(m: DirectoryNetworkMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.metrics) strims_debug_v1_DirectoryMetrics.encode(m.metrics, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryNetworkMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryNetworkMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.metrics = strims_debug_v1_DirectoryMetrics.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDirectoryMetrics = {
  server?: strims_debug_v1_IDirectoryServerMetrics;
  view?: strims_debug_v1_IDirectoryViewMetrics;
  serves?: bigint;
  pings?: bigint;
  pingErrors?: bigint;
  discontinuities?: bigint;
  assetBundles?: bigint;
  snippetsServed?: bigint;
  snippetsRefused?: bigint;
  embedBatches?: bigint;
  embedBatchErrors?: bigint;
  hostErrors?: bigint;
  storeErrors?: bigint;
}

export class DirectoryMetrics {
  server: strims_debug_v1_DirectoryServerMetrics | undefined;
  view: strims_debug_v1_DirectoryViewMetrics | undefined;
  serves: bigint;
  pings: bigint;
  pingErrors: bigint;
  discontinuities: bigint;
  assetBundles: bigint;
  snippetsServed: bigint;
  snippetsRefused: bigint;
  embedBatches: bigint;
  embedBatchErrors: bigint;
  hostErrors: bigint;
  storeErrors: bigint;

  constructor(v?: IDirectoryMetrics) {
    this.server = v?.server && new strims_debug_v1_DirectoryServerMetrics(v.server);
    this.view = v?.view && new strims_debug_v1_DirectoryViewMetrics(v.view);
    this.serves = v?.serves || BigInt(0);
    this.pings = v?.pings || BigInt(0);
    this.pingErrors = v?.pingErrors || BigInt(0);
    this.discontinuities = v?.discontinuities || BigInt(0);
    this.assetBundles = v?.assetBundles || BigInt(0);
    this.snippetsServed = v?.snippetsServed || BigInt(0);
    this.snippetsRefused = v?.snippetsRefused || BigInt(0);
    this.embedBatches = v?.embedBatches || BigInt(0);
    this.embedBatchErrors = v?.embedBatchErrors || BigInt(0);
    this.hostErrors = v?.hostErrors || BigInt(0);
    this.storeErrors = v?.storeErrors || BigInt(0);
  }

  static encode(m: DirectoryMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) strims_debug_v1_DirectoryServerMetrics.encode(m.server, w.uint32(10).fork()).ldelim();
    if (m.view) strims_debug_v1_DirectoryViewMetrics.encode(m.view, w.uint32(18).fork()).ldelim();
    if (m.serves) w.uint32(24).uint64(m.serves);
    if (m.pings) w.uint32(32).uint64(m.pings);
    if (m.pingErrors) w.uint32(40).uint64(m.pingErrors);
    if (m.discontinuities) w.uint32(48).uint64(m.discontinuities);
    if (m.assetBundles) w.uint32(56).uint64(m.assetBundles);
    if (m.snippetsServed) w.uint32(64).uint64(m.snippetsServed);
    if (m.snippetsRefused) w.uint32(72).uint64(m.snippetsRefused);
    if (m.embedBatches) w.uint32(80).uint64(m.embedBatches);
    if (m.embedBatchErrors) w.uint32(88).uint64(m.embedBatchErrors);
    if (m.hostErrors) w.uint32(96).uint64(m.hostErrors);
    if (m.storeErrors) w.uint32(104).uint64(m.storeErrors);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = strims_debug_v1_DirectoryServerMetrics.decode(r, r.uint32());
        break;
        case 2:
        m.view = strims_debug_v1_DirectoryViewMetrics.decode(r, r.uint32());
        break;
        case 3:
        m.serves = r.uint64();
        break;
        case 4:
        m.pings = r.uint64();
        break;
        case 5:
        m.pingErrors = r.uint64();
        break;
        case 6:
        m.discontinuities = r.uint64();
        break;
        case 7:
        m.assetBundles = r.uint64();
        break;
        case 8:
        m.snippetsServed = r.uint64();
        break;
        case 9:
        m.snippetsRefused = r.uint64();
        break;
        case 10:
        m.embedBatches = r.uint64();
        break;
        case 11:
        m.embedBatchErrors = r.uint64();
        break;
        case 12:
        m.hostErrors = r.uint64();
        break;
        case 13:
        m.storeErrors = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDirectoryServerMetrics = {
  broadcasts?: bigint;
  records?: bigint;
  events?: bigint;
  refreshes?: bigint;
  refreshed?: bigint;
  listingsCreated?: bigint;
  listingsRemoved?: bigint;
  usersCreated?: bigint;
  usersRemoved?: bigint;
  sessionsCreated?: bigint;
  sessionsExpired?: bigint;
  sessionsEvicted?: bigint;
  sessionsDisplaced?: bigint;
  refused?: bigint;
  subscriptions?: bigint;
  snippetResponses?: bigint;
  staleResponses?: bigint;
  snippetsShown?: bigint;
  snippetsUnverified?: bigint;
  snippetsStale?: bigint;
  snippetsEnded?: bigint;
  snippetTimeouts?: bigint;
  snippetsRejected?: bigint;
  snippetFloods?: bigint;
  snippetsOversized?: bigint;
  embedLoads?: bigint;
  embedSnippetsShown?: bigint;
  embedSnippetsRejected?: bigint;
  staleEmbedBatches?: bigint;
}

export class DirectoryServerMetrics {
  broadcasts: bigint;
  records: bigint;
  events: bigint;
  refreshes: bigint;
  refreshed: bigint;
  listingsCreated: bigint;
  listingsRemoved: bigint;
  usersCreated: bigint;
  usersRemoved: bigint;
  sessionsCreated: bigint;
  sessionsExpired: bigint;
  sessionsEvicted: bigint;
  sessionsDisplaced: bigint;
  refused: bigint;
  subscriptions: bigint;
  snippetResponses: bigint;
  staleResponses: bigint;
  snippetsShown: bigint;
  snippetsUnverified: bigint;
  snippetsStale: bigint;
  snippetsEnded: bigint;
  snippetTimeouts: bigint;
  snippetsRejected: bigint;
  snippetFloods: bigint;
  snippetsOversized: bigint;
  embedLoads: bigint;
  embedSnippetsShown: bigint;
  embedSnippetsRejected: bigint;
  staleEmbedBatches: bigint;

  constructor(v?: IDirectoryServerMetrics) {
    this.broadcasts = v?.broadcasts || BigInt(0);
    this.records = v?.records || BigInt(0);
    this.events = v?.events || BigInt(0);
    this.refreshes = v?.refreshes || BigInt(0);
    this.refreshed = v?.refreshed || BigInt(0);
    this.listingsCreated = v?.listingsCreated || BigInt(0);
    this.listingsRemoved = v?.listingsRemoved || BigInt(0);
    this.usersCreated = v?.usersCreated || BigInt(0);
    this.usersRemoved = v?.usersRemoved || BigInt(0);
    this.sessionsCreated = v?.sessionsCreated || BigInt(0);
    this.sessionsExpired = v?.sessionsExpired || BigInt(0);
    this.sessionsEvicted = v?.sessionsEvicted || BigInt(0);
    this.sessionsDisplaced = v?.sessionsDisplaced || BigInt(0);
    this.refused = v?.refused || BigInt(0);
    this.subscriptions = v?.subscriptions || BigInt(0);
    this.snippetResponses = v?.snippetResponses || BigInt(0);
    this.staleResponses = v?.staleResponses || BigInt(0);
    this.snippetsShown = v?.snippetsShown || BigInt(0);
    this.snippetsUnverified = v?.snippetsUnverified || BigInt(0);
    this.snippetsStale = v?.snippetsStale || BigInt(0);
    this.snippetsEnded = v?.snippetsEnded || BigInt(0);
    this.snippetTimeouts = v?.snippetTimeouts || BigInt(0);
    this.snippetsRejected = v?.snippetsRejected || BigInt(0);
    this.snippetFloods = v?.snippetFloods || BigInt(0);
    this.snippetsOversized = v?.snippetsOversized || BigInt(0);
    this.embedLoads = v?.embedLoads || BigInt(0);
    this.embedSnippetsShown = v?.embedSnippetsShown || BigInt(0);
    this.embedSnippetsRejected = v?.embedSnippetsRejected || BigInt(0);
    this.staleEmbedBatches = v?.staleEmbedBatches || BigInt(0);
  }

  static encode(m: DirectoryServerMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.broadcasts) w.uint32(8).uint64(m.broadcasts);
    if (m.records) w.uint32(16).uint64(m.records);
    if (m.events) w.uint32(24).uint64(m.events);
    if (m.refreshes) w.uint32(32).uint64(m.refreshes);
    if (m.refreshed) w.uint32(40).uint64(m.refreshed);
    if (m.listingsCreated) w.uint32(48).uint64(m.listingsCreated);
    if (m.listingsRemoved) w.uint32(56).uint64(m.listingsRemoved);
    if (m.usersCreated) w.uint32(64).uint64(m.usersCreated);
    if (m.usersRemoved) w.uint32(72).uint64(m.usersRemoved);
    if (m.sessionsCreated) w.uint32(80).uint64(m.sessionsCreated);
    if (m.sessionsExpired) w.uint32(88).uint64(m.sessionsExpired);
    if (m.sessionsEvicted) w.uint32(96).uint64(m.sessionsEvicted);
    if (m.sessionsDisplaced) w.uint32(104).uint64(m.sessionsDisplaced);
    if (m.refused) w.uint32(112).uint64(m.refused);
    if (m.subscriptions) w.uint32(120).uint64(m.subscriptions);
    if (m.snippetResponses) w.uint32(128).uint64(m.snippetResponses);
    if (m.staleResponses) w.uint32(136).uint64(m.staleResponses);
    if (m.snippetsShown) w.uint32(144).uint64(m.snippetsShown);
    if (m.snippetsUnverified) w.uint32(152).uint64(m.snippetsUnverified);
    if (m.snippetsStale) w.uint32(160).uint64(m.snippetsStale);
    if (m.snippetsEnded) w.uint32(168).uint64(m.snippetsEnded);
    if (m.snippetTimeouts) w.uint32(176).uint64(m.snippetTimeouts);
    if (m.snippetsRejected) w.uint32(184).uint64(m.snippetsRejected);
    if (m.snippetFloods) w.uint32(192).uint64(m.snippetFloods);
    if (m.snippetsOversized) w.uint32(200).uint64(m.snippetsOversized);
    if (m.embedLoads) w.uint32(208).uint64(m.embedLoads);
    if (m.embedSnippetsShown) w.uint32(216).uint64(m.embedSnippetsShown);
    if (m.embedSnippetsRejected) w.uint32(224).uint64(m.embedSnippetsRejected);
    if (m.staleEmbedBatches) w.uint32(232).uint64(m.staleEmbedBatches);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryServerMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryServerMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.broadcasts = r.uint64();
        break;
        case 2:
        m.records = r.uint64();
        break;
        case 3:
        m.events = r.uint64();
        break;
        case 4:
        m.refreshes = r.uint64();
        break;
        case 5:
        m.refreshed = r.uint64();
        break;
        case 6:
        m.listingsCreated = r.uint64();
        break;
        case 7:
        m.listingsRemoved = r.uint64();
        break;
        case 8:
        m.usersCreated = r.uint64();
        break;
        case 9:
        m.usersRemoved = r.uint64();
        break;
        case 10:
        m.sessionsCreated = r.uint64();
        break;
        case 11:
        m.sessionsExpired = r.uint64();
        break;
        case 12:
        m.sessionsEvicted = r.uint64();
        break;
        case 13:
        m.sessionsDisplaced = r.uint64();
        break;
        case 14:
        m.refused = r.uint64();
        break;
        case 15:
        m.subscriptions = r.uint64();
        break;
        case 16:
        m.snippetResponses = r.uint64();
        break;
        case 17:
        m.staleResponses = r.uint64();
        break;
        case 18:
        m.snippetsShown = r.uint64();
        break;
        case 19:
        m.snippetsUnverified = r.uint64();
        break;
        case 20:
        m.snippetsStale = r.uint64();
        break;
        case 21:
        m.snippetsEnded = r.uint64();
        break;
        case 22:
        m.snippetTimeouts = r.uint64();
        break;
        case 23:
        m.snippetsRejected = r.uint64();
        break;
        case 24:
        m.snippetFloods = r.uint64();
        break;
        case 25:
        m.snippetsOversized = r.uint64();
        break;
        case 26:
        m.embedLoads = r.uint64();
        break;
        case 27:
        m.embedSnippetsShown = r.uint64();
        break;
        case 28:
        m.embedSnippetsRejected = r.uint64();
        break;
        case 29:
        m.staleEmbedBatches = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDirectoryViewMetrics = {
  records?: bigint;
  events?: bigint;
  resets?: bigint;
  malformedRecords?: bigint;
  malformedEvents?: bigint;
  unknownListings?: bigint;
  listingsDropped?: bigint;
  usersDropped?: bigint;
  idsDropped?: bigint;
  malformedSnippets?: bigint;
  imagesDropped?: bigint;
  snippetsDropped?: bigint;
  oversizedListings?: bigint;
  eventsDropped?: bigint;
}

export class DirectoryViewMetrics {
  records: bigint;
  events: bigint;
  resets: bigint;
  malformedRecords: bigint;
  malformedEvents: bigint;
  unknownListings: bigint;
  listingsDropped: bigint;
  usersDropped: bigint;
  idsDropped: bigint;
  malformedSnippets: bigint;
  imagesDropped: bigint;
  snippetsDropped: bigint;
  oversizedListings: bigint;
  eventsDropped: bigint;

  constructor(v?: IDirectoryViewMetrics) {
    this.records = v?.records || BigInt(0);
    this.events = v?.events || BigInt(0);
    this.resets = v?.resets || BigInt(0);
    this.malformedRecords = v?.malformedRecords || BigInt(0);
    this.malformedEvents = v?.malformedEvents || BigInt(0);
    this.unknownListings = v?.unknownListings || BigInt(0);
    this.listingsDropped = v?.listingsDropped || BigInt(0);
    this.usersDropped = v?.usersDropped || BigInt(0);
    this.idsDropped = v?.idsDropped || BigInt(0);
    this.malformedSnippets = v?.malformedSnippets || BigInt(0);
    this.imagesDropped = v?.imagesDropped || BigInt(0);
    this.snippetsDropped = v?.snippetsDropped || BigInt(0);
    this.oversizedListings = v?.oversizedListings || BigInt(0);
    this.eventsDropped = v?.eventsDropped || BigInt(0);
  }

  static encode(m: DirectoryViewMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.records) w.uint32(8).uint64(m.records);
    if (m.events) w.uint32(16).uint64(m.events);
    if (m.resets) w.uint32(24).uint64(m.resets);
    if (m.malformedRecords) w.uint32(32).uint64(m.malformedRecords);
    if (m.malformedEvents) w.uint32(40).uint64(m.malformedEvents);
    if (m.unknownListings) w.uint32(48).uint64(m.unknownListings);
    if (m.listingsDropped) w.uint32(56).uint64(m.listingsDropped);
    if (m.usersDropped) w.uint32(64).uint64(m.usersDropped);
    if (m.idsDropped) w.uint32(72).uint64(m.idsDropped);
    if (m.malformedSnippets) w.uint32(80).uint64(m.malformedSnippets);
    if (m.imagesDropped) w.uint32(88).uint64(m.imagesDropped);
    if (m.snippetsDropped) w.uint32(96).uint64(m.snippetsDropped);
    if (m.oversizedListings) w.uint32(104).uint64(m.oversizedListings);
    if (m.eventsDropped) w.uint32(112).uint64(m.eventsDropped);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DirectoryViewMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DirectoryViewMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.records = r.uint64();
        break;
        case 2:
        m.events = r.uint64();
        break;
        case 3:
        m.resets = r.uint64();
        break;
        case 4:
        m.malformedRecords = r.uint64();
        break;
        case 5:
        m.malformedEvents = r.uint64();
        break;
        case 6:
        m.unknownListings = r.uint64();
        break;
        case 7:
        m.listingsDropped = r.uint64();
        break;
        case 8:
        m.usersDropped = r.uint64();
        break;
        case 9:
        m.idsDropped = r.uint64();
        break;
        case 10:
        m.malformedSnippets = r.uint64();
        break;
        case 11:
        m.imagesDropped = r.uint64();
        break;
        case 12:
        m.snippetsDropped = r.uint64();
        break;
        case 13:
        m.oversizedListings = r.uint64();
        break;
        case 14:
        m.eventsDropped = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IChatMetrics = {
  serves?: bigint;
  reads?: bigint;
  eventsWritten?: bigint;
  assetBundles?: bigint;
  assetResets?: bigint;
  assetsTooLarge?: bigint;
  listingPublishes?: bigint;
  listingErrors?: bigint;
  localCalls?: bigint;
  remoteCalls?: bigint;
  serversDropped?: bigint;
  streamsRefused?: bigint;
  cacheLoads?: bigint;
  cacheRefusals?: bigint;
  cacheWrites?: bigint;
  writeErrors?: bigint;
  hostErrors?: bigint;
  storeErrors?: bigint;
}

export class ChatMetrics {
  serves: bigint;
  reads: bigint;
  eventsWritten: bigint;
  assetBundles: bigint;
  assetResets: bigint;
  assetsTooLarge: bigint;
  listingPublishes: bigint;
  listingErrors: bigint;
  localCalls: bigint;
  remoteCalls: bigint;
  serversDropped: bigint;
  streamsRefused: bigint;
  cacheLoads: bigint;
  cacheRefusals: bigint;
  cacheWrites: bigint;
  writeErrors: bigint;
  hostErrors: bigint;
  storeErrors: bigint;

  constructor(v?: IChatMetrics) {
    this.serves = v?.serves || BigInt(0);
    this.reads = v?.reads || BigInt(0);
    this.eventsWritten = v?.eventsWritten || BigInt(0);
    this.assetBundles = v?.assetBundles || BigInt(0);
    this.assetResets = v?.assetResets || BigInt(0);
    this.assetsTooLarge = v?.assetsTooLarge || BigInt(0);
    this.listingPublishes = v?.listingPublishes || BigInt(0);
    this.listingErrors = v?.listingErrors || BigInt(0);
    this.localCalls = v?.localCalls || BigInt(0);
    this.remoteCalls = v?.remoteCalls || BigInt(0);
    this.serversDropped = v?.serversDropped || BigInt(0);
    this.streamsRefused = v?.streamsRefused || BigInt(0);
    this.cacheLoads = v?.cacheLoads || BigInt(0);
    this.cacheRefusals = v?.cacheRefusals || BigInt(0);
    this.cacheWrites = v?.cacheWrites || BigInt(0);
    this.writeErrors = v?.writeErrors || BigInt(0);
    this.hostErrors = v?.hostErrors || BigInt(0);
    this.storeErrors = v?.storeErrors || BigInt(0);
  }

  static encode(m: ChatMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serves) w.uint32(8).uint64(m.serves);
    if (m.reads) w.uint32(16).uint64(m.reads);
    if (m.eventsWritten) w.uint32(24).uint64(m.eventsWritten);
    if (m.assetBundles) w.uint32(32).uint64(m.assetBundles);
    if (m.assetResets) w.uint32(40).uint64(m.assetResets);
    if (m.assetsTooLarge) w.uint32(48).uint64(m.assetsTooLarge);
    if (m.listingPublishes) w.uint32(56).uint64(m.listingPublishes);
    if (m.listingErrors) w.uint32(64).uint64(m.listingErrors);
    if (m.localCalls) w.uint32(72).uint64(m.localCalls);
    if (m.remoteCalls) w.uint32(80).uint64(m.remoteCalls);
    if (m.serversDropped) w.uint32(88).uint64(m.serversDropped);
    if (m.streamsRefused) w.uint32(96).uint64(m.streamsRefused);
    if (m.cacheLoads) w.uint32(104).uint64(m.cacheLoads);
    if (m.cacheRefusals) w.uint32(112).uint64(m.cacheRefusals);
    if (m.cacheWrites) w.uint32(120).uint64(m.cacheWrites);
    if (m.writeErrors) w.uint32(128).uint64(m.writeErrors);
    if (m.hostErrors) w.uint32(136).uint64(m.hostErrors);
    if (m.storeErrors) w.uint32(144).uint64(m.storeErrors);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ChatMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ChatMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serves = r.uint64();
        break;
        case 2:
        m.reads = r.uint64();
        break;
        case 3:
        m.eventsWritten = r.uint64();
        break;
        case 4:
        m.assetBundles = r.uint64();
        break;
        case 5:
        m.assetResets = r.uint64();
        break;
        case 6:
        m.assetsTooLarge = r.uint64();
        break;
        case 7:
        m.listingPublishes = r.uint64();
        break;
        case 8:
        m.listingErrors = r.uint64();
        break;
        case 9:
        m.localCalls = r.uint64();
        break;
        case 10:
        m.remoteCalls = r.uint64();
        break;
        case 11:
        m.serversDropped = r.uint64();
        break;
        case 12:
        m.streamsRefused = r.uint64();
        break;
        case 13:
        m.cacheLoads = r.uint64();
        break;
        case 14:
        m.cacheRefusals = r.uint64();
        break;
        case 15:
        m.cacheWrites = r.uint64();
        break;
        case 16:
        m.writeErrors = r.uint64();
        break;
        case 17:
        m.hostErrors = r.uint64();
        break;
        case 18:
        m.storeErrors = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWhisperMetrics = {
  received?: bigint;
  duplicates?: bigint;
  refused?: bigint;
  rateLimited?: bigint;
  enqueued?: bigint;
  attempts?: bigint;
  delivered?: bigint;
  failed?: bigint;
  leases?: bigint;
  portsOpened?: bigint;
  pending?: bigint;
  inFlight?: bigint;
  hostErrors?: bigint;
  storeErrors?: bigint;
}

export class WhisperMetrics {
  received: bigint;
  duplicates: bigint;
  refused: bigint;
  rateLimited: bigint;
  enqueued: bigint;
  attempts: bigint;
  delivered: bigint;
  failed: bigint;
  leases: bigint;
  portsOpened: bigint;
  pending: bigint;
  inFlight: bigint;
  hostErrors: bigint;
  storeErrors: bigint;

  constructor(v?: IWhisperMetrics) {
    this.received = v?.received || BigInt(0);
    this.duplicates = v?.duplicates || BigInt(0);
    this.refused = v?.refused || BigInt(0);
    this.rateLimited = v?.rateLimited || BigInt(0);
    this.enqueued = v?.enqueued || BigInt(0);
    this.attempts = v?.attempts || BigInt(0);
    this.delivered = v?.delivered || BigInt(0);
    this.failed = v?.failed || BigInt(0);
    this.leases = v?.leases || BigInt(0);
    this.portsOpened = v?.portsOpened || BigInt(0);
    this.pending = v?.pending || BigInt(0);
    this.inFlight = v?.inFlight || BigInt(0);
    this.hostErrors = v?.hostErrors || BigInt(0);
    this.storeErrors = v?.storeErrors || BigInt(0);
  }

  static encode(m: WhisperMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.received) w.uint32(8).uint64(m.received);
    if (m.duplicates) w.uint32(16).uint64(m.duplicates);
    if (m.refused) w.uint32(24).uint64(m.refused);
    if (m.rateLimited) w.uint32(32).uint64(m.rateLimited);
    if (m.enqueued) w.uint32(40).uint64(m.enqueued);
    if (m.attempts) w.uint32(48).uint64(m.attempts);
    if (m.delivered) w.uint32(56).uint64(m.delivered);
    if (m.failed) w.uint32(64).uint64(m.failed);
    if (m.leases) w.uint32(72).uint64(m.leases);
    if (m.portsOpened) w.uint32(80).uint64(m.portsOpened);
    if (m.pending) w.uint32(88).uint64(m.pending);
    if (m.inFlight) w.uint32(96).uint64(m.inFlight);
    if (m.hostErrors) w.uint32(104).uint64(m.hostErrors);
    if (m.storeErrors) w.uint32(112).uint64(m.storeErrors);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WhisperMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.received = r.uint64();
        break;
        case 2:
        m.duplicates = r.uint64();
        break;
        case 3:
        m.refused = r.uint64();
        break;
        case 4:
        m.rateLimited = r.uint64();
        break;
        case 5:
        m.enqueued = r.uint64();
        break;
        case 6:
        m.attempts = r.uint64();
        break;
        case 7:
        m.delivered = r.uint64();
        break;
        case 8:
        m.failed = r.uint64();
        break;
        case 9:
        m.leases = r.uint64();
        break;
        case 10:
        m.portsOpened = r.uint64();
        break;
        case 11:
        m.pending = r.uint64();
        break;
        case 12:
        m.inFlight = r.uint64();
        break;
        case 13:
        m.hostErrors = r.uint64();
        break;
        case 14:
        m.storeErrors = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoMetrics = {
  storeLags?: bigint;
  storeErrors?: bigint;
  hostErrors?: bigint;
  droppedRecords?: bigint;
  writeErrors?: bigint;
  reopens?: bigint;
  hlsStreams?: bigint;
  hlsSegments?: bigint;
  hlsSkippedRecords?: bigint;
  autoseedSkipped?: bigint;
  ingress?: strims_debug_v1_IIngressMetrics;
}

export class VideoMetrics {
  storeLags: bigint;
  storeErrors: bigint;
  hostErrors: bigint;
  droppedRecords: bigint;
  writeErrors: bigint;
  reopens: bigint;
  hlsStreams: bigint;
  hlsSegments: bigint;
  hlsSkippedRecords: bigint;
  autoseedSkipped: bigint;
  ingress: strims_debug_v1_IngressMetrics | undefined;

  constructor(v?: IVideoMetrics) {
    this.storeLags = v?.storeLags || BigInt(0);
    this.storeErrors = v?.storeErrors || BigInt(0);
    this.hostErrors = v?.hostErrors || BigInt(0);
    this.droppedRecords = v?.droppedRecords || BigInt(0);
    this.writeErrors = v?.writeErrors || BigInt(0);
    this.reopens = v?.reopens || BigInt(0);
    this.hlsStreams = v?.hlsStreams || BigInt(0);
    this.hlsSegments = v?.hlsSegments || BigInt(0);
    this.hlsSkippedRecords = v?.hlsSkippedRecords || BigInt(0);
    this.autoseedSkipped = v?.autoseedSkipped || BigInt(0);
    this.ingress = v?.ingress && new strims_debug_v1_IngressMetrics(v.ingress);
  }

  static encode(m: VideoMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.storeLags) w.uint32(8).uint64(m.storeLags);
    if (m.storeErrors) w.uint32(16).uint64(m.storeErrors);
    if (m.hostErrors) w.uint32(24).uint64(m.hostErrors);
    if (m.droppedRecords) w.uint32(32).uint64(m.droppedRecords);
    if (m.writeErrors) w.uint32(40).uint64(m.writeErrors);
    if (m.reopens) w.uint32(48).uint64(m.reopens);
    if (m.hlsStreams) w.uint32(56).uint64(m.hlsStreams);
    if (m.hlsSegments) w.uint32(64).uint64(m.hlsSegments);
    if (m.hlsSkippedRecords) w.uint32(72).uint64(m.hlsSkippedRecords);
    if (m.autoseedSkipped) w.uint32(80).uint64(m.autoseedSkipped);
    if (m.ingress) strims_debug_v1_IngressMetrics.encode(m.ingress, w.uint32(90).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.storeLags = r.uint64();
        break;
        case 2:
        m.storeErrors = r.uint64();
        break;
        case 3:
        m.hostErrors = r.uint64();
        break;
        case 4:
        m.droppedRecords = r.uint64();
        break;
        case 5:
        m.writeErrors = r.uint64();
        break;
        case 6:
        m.reopens = r.uint64();
        break;
        case 7:
        m.hlsStreams = r.uint64();
        break;
        case 8:
        m.hlsSegments = r.uint64();
        break;
        case 9:
        m.hlsSkippedRecords = r.uint64();
        break;
        case 10:
        m.autoseedSkipped = r.uint64();
        break;
        case 11:
        m.ingress = strims_debug_v1_IngressMetrics.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IIngressMetrics = {
  listens?: bigint;
  listenErrors?: bigint;
  connections?: bigint;
  rejectedConnections?: bigint;
  refused?: bigint;
  sessions?: bigint;
  replaced?: bigint;
  segments?: bigint;
  bytes?: bigint;
  droppedUnits?: bigint;
  metadata?: bigint;
  deadlines?: bigint;
  mediaErrors?: bigint;
  writeErrors?: bigint;
  earlySigns?: bigint;
}

export class IngressMetrics {
  listens: bigint;
  listenErrors: bigint;
  connections: bigint;
  rejectedConnections: bigint;
  refused: bigint;
  sessions: bigint;
  replaced: bigint;
  segments: bigint;
  bytes: bigint;
  droppedUnits: bigint;
  metadata: bigint;
  deadlines: bigint;
  mediaErrors: bigint;
  writeErrors: bigint;
  earlySigns: bigint;

  constructor(v?: IIngressMetrics) {
    this.listens = v?.listens || BigInt(0);
    this.listenErrors = v?.listenErrors || BigInt(0);
    this.connections = v?.connections || BigInt(0);
    this.rejectedConnections = v?.rejectedConnections || BigInt(0);
    this.refused = v?.refused || BigInt(0);
    this.sessions = v?.sessions || BigInt(0);
    this.replaced = v?.replaced || BigInt(0);
    this.segments = v?.segments || BigInt(0);
    this.bytes = v?.bytes || BigInt(0);
    this.droppedUnits = v?.droppedUnits || BigInt(0);
    this.metadata = v?.metadata || BigInt(0);
    this.deadlines = v?.deadlines || BigInt(0);
    this.mediaErrors = v?.mediaErrors || BigInt(0);
    this.writeErrors = v?.writeErrors || BigInt(0);
    this.earlySigns = v?.earlySigns || BigInt(0);
  }

  static encode(m: IngressMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.listens) w.uint32(8).uint64(m.listens);
    if (m.listenErrors) w.uint32(16).uint64(m.listenErrors);
    if (m.connections) w.uint32(24).uint64(m.connections);
    if (m.rejectedConnections) w.uint32(32).uint64(m.rejectedConnections);
    if (m.refused) w.uint32(40).uint64(m.refused);
    if (m.sessions) w.uint32(48).uint64(m.sessions);
    if (m.replaced) w.uint32(56).uint64(m.replaced);
    if (m.segments) w.uint32(64).uint64(m.segments);
    if (m.bytes) w.uint32(72).uint64(m.bytes);
    if (m.droppedUnits) w.uint32(80).uint64(m.droppedUnits);
    if (m.metadata) w.uint32(88).uint64(m.metadata);
    if (m.deadlines) w.uint32(96).uint64(m.deadlines);
    if (m.mediaErrors) w.uint32(104).uint64(m.mediaErrors);
    if (m.writeErrors) w.uint32(112).uint64(m.writeErrors);
    if (m.earlySigns) w.uint32(120).uint64(m.earlySigns);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): IngressMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new IngressMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.listens = r.uint64();
        break;
        case 2:
        m.listenErrors = r.uint64();
        break;
        case 3:
        m.connections = r.uint64();
        break;
        case 4:
        m.rejectedConnections = r.uint64();
        break;
        case 5:
        m.refused = r.uint64();
        break;
        case 6:
        m.sessions = r.uint64();
        break;
        case 7:
        m.replaced = r.uint64();
        break;
        case 8:
        m.segments = r.uint64();
        break;
        case 9:
        m.bytes = r.uint64();
        break;
        case 10:
        m.droppedUnits = r.uint64();
        break;
        case 11:
        m.metadata = r.uint64();
        break;
        case 12:
        m.deadlines = r.uint64();
        break;
        case 13:
        m.mediaErrors = r.uint64();
        break;
        case 14:
        m.writeErrors = r.uint64();
        break;
        case 15:
        m.earlySigns = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDebugMetrics = {
  mockStreamsStarted?: bigint;
  mockStreamsEnded?: bigint;
  segmentsWritten?: bigint;
  writeErrors?: bigint;
  readersStarted?: bigint;
  readersEnded?: bigint;
  segmentsRead?: bigint;
  readErrors?: bigint;
  hostErrors?: bigint;
  storeErrors?: bigint;
  storeLags?: bigint;
  mockStreams?: strims_debug_v1_IMockStream[];
  readers?: strims_debug_v1_IMockStreamReader[];
}

export class DebugMetrics {
  mockStreamsStarted: bigint;
  mockStreamsEnded: bigint;
  segmentsWritten: bigint;
  writeErrors: bigint;
  readersStarted: bigint;
  readersEnded: bigint;
  segmentsRead: bigint;
  readErrors: bigint;
  hostErrors: bigint;
  storeErrors: bigint;
  storeLags: bigint;
  mockStreams: strims_debug_v1_MockStream[];
  readers: strims_debug_v1_MockStreamReader[];

  constructor(v?: IDebugMetrics) {
    this.mockStreamsStarted = v?.mockStreamsStarted || BigInt(0);
    this.mockStreamsEnded = v?.mockStreamsEnded || BigInt(0);
    this.segmentsWritten = v?.segmentsWritten || BigInt(0);
    this.writeErrors = v?.writeErrors || BigInt(0);
    this.readersStarted = v?.readersStarted || BigInt(0);
    this.readersEnded = v?.readersEnded || BigInt(0);
    this.segmentsRead = v?.segmentsRead || BigInt(0);
    this.readErrors = v?.readErrors || BigInt(0);
    this.hostErrors = v?.hostErrors || BigInt(0);
    this.storeErrors = v?.storeErrors || BigInt(0);
    this.storeLags = v?.storeLags || BigInt(0);
    this.mockStreams = v?.mockStreams ? v.mockStreams.map(v => new strims_debug_v1_MockStream(v)) : [];
    this.readers = v?.readers ? v.readers.map(v => new strims_debug_v1_MockStreamReader(v)) : [];
  }

  static encode(m: DebugMetrics, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.mockStreamsStarted) w.uint32(8).uint64(m.mockStreamsStarted);
    if (m.mockStreamsEnded) w.uint32(16).uint64(m.mockStreamsEnded);
    if (m.segmentsWritten) w.uint32(24).uint64(m.segmentsWritten);
    if (m.writeErrors) w.uint32(32).uint64(m.writeErrors);
    if (m.readersStarted) w.uint32(40).uint64(m.readersStarted);
    if (m.readersEnded) w.uint32(48).uint64(m.readersEnded);
    if (m.segmentsRead) w.uint32(56).uint64(m.segmentsRead);
    if (m.readErrors) w.uint32(64).uint64(m.readErrors);
    if (m.hostErrors) w.uint32(72).uint64(m.hostErrors);
    if (m.storeErrors) w.uint32(80).uint64(m.storeErrors);
    if (m.storeLags) w.uint32(88).uint64(m.storeLags);
    for (const v of m.mockStreams) strims_debug_v1_MockStream.encode(v, w.uint32(98).fork()).ldelim();
    for (const v of m.readers) strims_debug_v1_MockStreamReader.encode(v, w.uint32(106).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DebugMetrics {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DebugMetrics();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.mockStreamsStarted = r.uint64();
        break;
        case 2:
        m.mockStreamsEnded = r.uint64();
        break;
        case 3:
        m.segmentsWritten = r.uint64();
        break;
        case 4:
        m.writeErrors = r.uint64();
        break;
        case 5:
        m.readersStarted = r.uint64();
        break;
        case 6:
        m.readersEnded = r.uint64();
        break;
        case 7:
        m.segmentsRead = r.uint64();
        break;
        case 8:
        m.readErrors = r.uint64();
        break;
        case 9:
        m.hostErrors = r.uint64();
        break;
        case 10:
        m.storeErrors = r.uint64();
        break;
        case 11:
        m.storeLags = r.uint64();
        break;
        case 12:
        m.mockStreams.push(strims_debug_v1_MockStream.decode(r, r.uint32()));
        break;
        case 13:
        m.readers.push(strims_debug_v1_MockStreamReader.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMockStream = {
  id?: bigint;
  networkKey?: Uint8Array;
  swarmUri?: string;
  bitrateKbps?: number;
  segmentIntervalMs?: number;
  timeoutMs?: number;
  startedAt?: bigint;
  segments?: bigint;
  bytesWritten?: bigint;
  listingId?: google_protobuf_IUInt64Value;
}

export class MockStream {
  id: bigint;
  networkKey: Uint8Array;
  swarmUri: string;
  bitrateKbps: number;
  segmentIntervalMs: number;
  timeoutMs: number;
  startedAt: bigint;
  segments: bigint;
  bytesWritten: bigint;
  listingId: google_protobuf_UInt64Value | undefined;

  constructor(v?: IMockStream) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.swarmUri = v?.swarmUri || "";
    this.bitrateKbps = v?.bitrateKbps || 0;
    this.segmentIntervalMs = v?.segmentIntervalMs || 0;
    this.timeoutMs = v?.timeoutMs || 0;
    this.startedAt = v?.startedAt || BigInt(0);
    this.segments = v?.segments || BigInt(0);
    this.bytesWritten = v?.bytesWritten || BigInt(0);
    this.listingId = v?.listingId && new google_protobuf_UInt64Value(v.listingId);
  }

  static encode(m: MockStream, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.swarmUri.length) w.uint32(26).string(m.swarmUri);
    if (m.bitrateKbps) w.uint32(32).uint32(m.bitrateKbps);
    if (m.segmentIntervalMs) w.uint32(40).uint32(m.segmentIntervalMs);
    if (m.timeoutMs) w.uint32(48).uint32(m.timeoutMs);
    if (m.startedAt) w.uint32(56).int64(m.startedAt);
    if (m.segments) w.uint32(64).uint64(m.segments);
    if (m.bytesWritten) w.uint32(72).uint64(m.bytesWritten);
    if (m.listingId) google_protobuf_UInt64Value.encode(m.listingId, w.uint32(82).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MockStream {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MockStream();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.networkKey = r.bytes();
        break;
        case 3:
        m.swarmUri = r.string();
        break;
        case 4:
        m.bitrateKbps = r.uint32();
        break;
        case 5:
        m.segmentIntervalMs = r.uint32();
        break;
        case 6:
        m.timeoutMs = r.uint32();
        break;
        case 7:
        m.startedAt = r.int64();
        break;
        case 8:
        m.segments = r.uint64();
        break;
        case 9:
        m.bytesWritten = r.uint64();
        break;
        case 10:
        m.listingId = google_protobuf_UInt64Value.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMockStreamReader = {
  networkKey?: Uint8Array;
  listingId?: bigint;
  swarmId?: Uint8Array;
  id?: google_protobuf_IUInt64Value;
  segments?: bigint;
  malformed?: bigint;
  discontinuities?: bigint;
  latency?: google_protobuf_IInt64Value;
  lastSegmentAt?: google_protobuf_IInt64Value;
  startedAt?: bigint;
}

export class MockStreamReader {
  networkKey: Uint8Array;
  listingId: bigint;
  swarmId: Uint8Array;
  id: google_protobuf_UInt64Value | undefined;
  segments: bigint;
  malformed: bigint;
  discontinuities: bigint;
  latency: google_protobuf_Int64Value | undefined;
  lastSegmentAt: google_protobuf_Int64Value | undefined;
  startedAt: bigint;

  constructor(v?: IMockStreamReader) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.listingId = v?.listingId || BigInt(0);
    this.swarmId = v?.swarmId || new Uint8Array();
    this.id = v?.id && new google_protobuf_UInt64Value(v.id);
    this.segments = v?.segments || BigInt(0);
    this.malformed = v?.malformed || BigInt(0);
    this.discontinuities = v?.discontinuities || BigInt(0);
    this.latency = v?.latency && new google_protobuf_Int64Value(v.latency);
    this.lastSegmentAt = v?.lastSegmentAt && new google_protobuf_Int64Value(v.lastSegmentAt);
    this.startedAt = v?.startedAt || BigInt(0);
  }

  static encode(m: MockStreamReader, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.listingId) w.uint32(16).uint64(m.listingId);
    if (m.swarmId.length) w.uint32(26).bytes(m.swarmId);
    if (m.id) google_protobuf_UInt64Value.encode(m.id, w.uint32(34).fork()).ldelim();
    if (m.segments) w.uint32(40).uint64(m.segments);
    if (m.malformed) w.uint32(48).uint64(m.malformed);
    if (m.discontinuities) w.uint32(56).uint64(m.discontinuities);
    if (m.latency) google_protobuf_Int64Value.encode(m.latency, w.uint32(66).fork()).ldelim();
    if (m.lastSegmentAt) google_protobuf_Int64Value.encode(m.lastSegmentAt, w.uint32(74).fork()).ldelim();
    if (m.startedAt) w.uint32(80).int64(m.startedAt);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MockStreamReader {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MockStreamReader();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.listingId = r.uint64();
        break;
        case 3:
        m.swarmId = r.bytes();
        break;
        case 4:
        m.id = google_protobuf_UInt64Value.decode(r, r.uint32());
        break;
        case 5:
        m.segments = r.uint64();
        break;
        case 6:
        m.malformed = r.uint64();
        break;
        case 7:
        m.discontinuities = r.uint64();
        break;
        case 8:
        m.latency = google_protobuf_Int64Value.decode(r, r.uint32());
        break;
        case 9:
        m.lastSegmentAt = google_protobuf_Int64Value.decode(r, r.uint32());
        break;
        case 10:
        m.startedAt = r.int64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export enum LinkKind {
  LINK_KIND_OTHER = 0,
  LINK_KIND_TCP = 1,
  LINK_KIND_WS = 2,
  LINK_KIND_WEBRTC = 3,
}
export enum PushMode {
  PUSH_MODE_OFF = 0,
  PUSH_MODE_EAGER_HAVE = 1,
  PUSH_MODE_HYBRID = 2,
}
export enum RequestControllerKind {
  REQUEST_CONTROLLER_KIND_DEFAULT = 0,
  REQUEST_CONTROLLER_KIND_LEDBAT = 1,
  REQUEST_CONTROLLER_KIND_BBR_LIKE = 2,
}
/* @internal */
export const strims_debug_v1_Metrics = Metrics;
/* @internal */
export type strims_debug_v1_Metrics = Metrics;
/* @internal */
export type strims_debug_v1_IMetrics = IMetrics;
/* @internal */
export const strims_debug_v1_MemoryMetrics = MemoryMetrics;
/* @internal */
export type strims_debug_v1_MemoryMetrics = MemoryMetrics;
/* @internal */
export type strims_debug_v1_IMemoryMetrics = IMemoryMetrics;
/* @internal */
export const strims_debug_v1_NodeMetrics = NodeMetrics;
/* @internal */
export type strims_debug_v1_NodeMetrics = NodeMetrics;
/* @internal */
export type strims_debug_v1_INodeMetrics = INodeMetrics;
/* @internal */
export const strims_debug_v1_HostMetrics = HostMetrics;
/* @internal */
export type strims_debug_v1_HostMetrics = HostMetrics;
/* @internal */
export type strims_debug_v1_IHostMetrics = IHostMetrics;
/* @internal */
export const strims_debug_v1_VpnMetrics = VpnMetrics;
/* @internal */
export type strims_debug_v1_VpnMetrics = VpnMetrics;
/* @internal */
export type strims_debug_v1_IVpnMetrics = IVpnMetrics;
/* @internal */
export const strims_debug_v1_DriverMetrics = DriverMetrics;
/* @internal */
export type strims_debug_v1_DriverMetrics = DriverMetrics;
/* @internal */
export type strims_debug_v1_IDriverMetrics = IDriverMetrics;
/* @internal */
export const strims_debug_v1_LinkMetrics = LinkMetrics;
/* @internal */
export type strims_debug_v1_LinkMetrics = LinkMetrics;
/* @internal */
export type strims_debug_v1_ILinkMetrics = ILinkMetrics;
/* @internal */
export const strims_debug_v1_LinkInfo = LinkInfo;
/* @internal */
export type strims_debug_v1_LinkInfo = LinkInfo;
/* @internal */
export type strims_debug_v1_ILinkInfo = ILinkInfo;
/* @internal */
export const strims_debug_v1_RunnerMetrics = RunnerMetrics;
/* @internal */
export type strims_debug_v1_RunnerMetrics = RunnerMetrics;
/* @internal */
export type strims_debug_v1_IRunnerMetrics = IRunnerMetrics;
/* @internal */
export const strims_debug_v1_SwarmInfo = SwarmInfo;
/* @internal */
export type strims_debug_v1_SwarmInfo = SwarmInfo;
/* @internal */
export type strims_debug_v1_ISwarmInfo = ISwarmInfo;
/* @internal */
export const strims_debug_v1_StoreInfo = StoreInfo;
/* @internal */
export type strims_debug_v1_StoreInfo = StoreInfo;
/* @internal */
export type strims_debug_v1_IStoreInfo = IStoreInfo;
/* @internal */
export const strims_debug_v1_SwarmSchedulerStats = SwarmSchedulerStats;
/* @internal */
export type strims_debug_v1_SwarmSchedulerStats = SwarmSchedulerStats;
/* @internal */
export type strims_debug_v1_ISwarmSchedulerStats = ISwarmSchedulerStats;
/* @internal */
export const strims_debug_v1_PushParent = PushParent;
/* @internal */
export type strims_debug_v1_PushParent = PushParent;
/* @internal */
export type strims_debug_v1_IPushParent = IPushParent;
/* @internal */
export const strims_debug_v1_PushStats = PushStats;
/* @internal */
export type strims_debug_v1_PushStats = PushStats;
/* @internal */
export type strims_debug_v1_IPushStats = IPushStats;
/* @internal */
export const strims_debug_v1_RunnerPeerInfo = RunnerPeerInfo;
/* @internal */
export type strims_debug_v1_RunnerPeerInfo = RunnerPeerInfo;
/* @internal */
export type strims_debug_v1_IRunnerPeerInfo = IRunnerPeerInfo;
/* @internal */
export const strims_debug_v1_PeerStats = PeerStats;
/* @internal */
export type strims_debug_v1_PeerStats = PeerStats;
/* @internal */
export type strims_debug_v1_IPeerStats = IPeerStats;
/* @internal */
export const strims_debug_v1_TransferMetricsSnapshot = TransferMetricsSnapshot;
/* @internal */
export type strims_debug_v1_TransferMetricsSnapshot = TransferMetricsSnapshot;
/* @internal */
export type strims_debug_v1_ITransferMetricsSnapshot = ITransferMetricsSnapshot;
/* @internal */
export const strims_debug_v1_MetricsSnapshot = MetricsSnapshot;
/* @internal */
export type strims_debug_v1_MetricsSnapshot = MetricsSnapshot;
/* @internal */
export type strims_debug_v1_IMetricsSnapshot = IMetricsSnapshot;
/* @internal */
export const strims_debug_v1_WriterInfo = WriterInfo;
/* @internal */
export type strims_debug_v1_WriterInfo = WriterInfo;
/* @internal */
export type strims_debug_v1_IWriterInfo = IWriterInfo;
/* @internal */
export const strims_debug_v1_ChannelInfo = ChannelInfo;
/* @internal */
export type strims_debug_v1_ChannelInfo = ChannelInfo;
/* @internal */
export type strims_debug_v1_IChannelInfo = IChannelInfo;
/* @internal */
export const strims_debug_v1_ChannelStats = ChannelStats;
/* @internal */
export type strims_debug_v1_ChannelStats = ChannelStats;
/* @internal */
export type strims_debug_v1_IChannelStats = IChannelStats;
/* @internal */
export const strims_debug_v1_ChannelSchedulerStats = ChannelSchedulerStats;
/* @internal */
export type strims_debug_v1_ChannelSchedulerStats = ChannelSchedulerStats;
/* @internal */
export type strims_debug_v1_IChannelSchedulerStats = IChannelSchedulerStats;
/* @internal */
export const strims_debug_v1_ChannelMetrics = ChannelMetrics;
/* @internal */
export type strims_debug_v1_ChannelMetrics = ChannelMetrics;
/* @internal */
export type strims_debug_v1_IChannelMetrics = IChannelMetrics;
/* @internal */
export const strims_debug_v1_NetworkInfo = NetworkInfo;
/* @internal */
export type strims_debug_v1_NetworkInfo = NetworkInfo;
/* @internal */
export type strims_debug_v1_INetworkInfo = INetworkInfo;
/* @internal */
export const strims_debug_v1_NetworkMetrics = NetworkMetrics;
/* @internal */
export type strims_debug_v1_NetworkMetrics = NetworkMetrics;
/* @internal */
export type strims_debug_v1_INetworkMetrics = INetworkMetrics;
/* @internal */
export const strims_debug_v1_TransferMetrics = TransferMetrics;
/* @internal */
export type strims_debug_v1_TransferMetrics = TransferMetrics;
/* @internal */
export type strims_debug_v1_ITransferMetrics = ITransferMetrics;
/* @internal */
export const strims_debug_v1_ReplicationMetrics = ReplicationMetrics;
/* @internal */
export type strims_debug_v1_ReplicationMetrics = ReplicationMetrics;
/* @internal */
export type strims_debug_v1_IReplicationMetrics = IReplicationMetrics;
/* @internal */
export const strims_debug_v1_DirectoryNetworkMetrics = DirectoryNetworkMetrics;
/* @internal */
export type strims_debug_v1_DirectoryNetworkMetrics = DirectoryNetworkMetrics;
/* @internal */
export type strims_debug_v1_IDirectoryNetworkMetrics = IDirectoryNetworkMetrics;
/* @internal */
export const strims_debug_v1_DirectoryMetrics = DirectoryMetrics;
/* @internal */
export type strims_debug_v1_DirectoryMetrics = DirectoryMetrics;
/* @internal */
export type strims_debug_v1_IDirectoryMetrics = IDirectoryMetrics;
/* @internal */
export const strims_debug_v1_DirectoryServerMetrics = DirectoryServerMetrics;
/* @internal */
export type strims_debug_v1_DirectoryServerMetrics = DirectoryServerMetrics;
/* @internal */
export type strims_debug_v1_IDirectoryServerMetrics = IDirectoryServerMetrics;
/* @internal */
export const strims_debug_v1_DirectoryViewMetrics = DirectoryViewMetrics;
/* @internal */
export type strims_debug_v1_DirectoryViewMetrics = DirectoryViewMetrics;
/* @internal */
export type strims_debug_v1_IDirectoryViewMetrics = IDirectoryViewMetrics;
/* @internal */
export const strims_debug_v1_ChatMetrics = ChatMetrics;
/* @internal */
export type strims_debug_v1_ChatMetrics = ChatMetrics;
/* @internal */
export type strims_debug_v1_IChatMetrics = IChatMetrics;
/* @internal */
export const strims_debug_v1_WhisperMetrics = WhisperMetrics;
/* @internal */
export type strims_debug_v1_WhisperMetrics = WhisperMetrics;
/* @internal */
export type strims_debug_v1_IWhisperMetrics = IWhisperMetrics;
/* @internal */
export const strims_debug_v1_VideoMetrics = VideoMetrics;
/* @internal */
export type strims_debug_v1_VideoMetrics = VideoMetrics;
/* @internal */
export type strims_debug_v1_IVideoMetrics = IVideoMetrics;
/* @internal */
export const strims_debug_v1_IngressMetrics = IngressMetrics;
/* @internal */
export type strims_debug_v1_IngressMetrics = IngressMetrics;
/* @internal */
export type strims_debug_v1_IIngressMetrics = IIngressMetrics;
/* @internal */
export const strims_debug_v1_DebugMetrics = DebugMetrics;
/* @internal */
export type strims_debug_v1_DebugMetrics = DebugMetrics;
/* @internal */
export type strims_debug_v1_IDebugMetrics = IDebugMetrics;
/* @internal */
export const strims_debug_v1_MockStream = MockStream;
/* @internal */
export type strims_debug_v1_MockStream = MockStream;
/* @internal */
export type strims_debug_v1_IMockStream = IMockStream;
/* @internal */
export const strims_debug_v1_MockStreamReader = MockStreamReader;
/* @internal */
export type strims_debug_v1_MockStreamReader = MockStreamReader;
/* @internal */
export type strims_debug_v1_IMockStreamReader = IMockStreamReader;
/* @internal */
export const strims_debug_v1_LinkKind = LinkKind;
/* @internal */
export type strims_debug_v1_LinkKind = LinkKind;
/* @internal */
export const strims_debug_v1_PushMode = PushMode;
/* @internal */
export type strims_debug_v1_PushMode = PushMode;
/* @internal */
export const strims_debug_v1_RequestControllerKind = RequestControllerKind;
/* @internal */
export type strims_debug_v1_RequestControllerKind = RequestControllerKind;
