# Plan

Forward-looking work only. What exists is described by the code. How to work
on it is in `CONVENTIONS.md`.

## Remaining work

### App layer
Port the application services that sit on the core crates. The Go versions
(`internal/*`) are the behavioural reference.
- Video; see Video below.
- The other dao record types arrive with their services, and those that
  name a network with their part of its deletion. Until then, stores from
  Go keep these records unread:
  - the autoseed config and rules (10001, 10002): seeding the swarms of
    directory listings that a rule names by network, swarm id and salt,
    and the UI's autoseed frontend. Deleting a network deletes its rules;
  - swarm caches (11001–11003): a swarm's data and signatures under
    (swarm id, salt), so a reopened swarm starts from them. A chat room's
    asset swarm can start from its cache under (server key,
    `chat:assets`); until then a reopened room downloads its bundles
    again.
- Import of Go bbolt stores into `strims-kv-redb` for native users.

### Video
Gaps in the current services:
- Snippets go live without a thumbnail. Making one needs a decoder stage
  in the driver (ffmpeg or a library) over a segment's keyframe.
- The VideoIngressShare service isn't served. The ingress config's
  `service_network_keys` are stored, and pruned when a network is
  deleted, but nothing reads them.
- One ingest per channel holds within a process. Stores shared by several
  processes need the lease that spans them (see `strims-svc` below).
- A writer never publishes its last partial integrity unit, so viewers
  miss the end of a stream's last segment.
- A stream without audio may not play in the current UI, whose
  MediaSource codec string names both tracks.
- No edit list offsets the B-frame composition delay.
- A large backward jump in FLV tag times holds both tracks' timestamps
  until the tag times catch up, which stalls an encoder that restarts its
  clock mid-stream.
- RTMP: a connection publishes once; AMF3 commands and shared objects are
  ignored; the server never sends StreamBegin. A peer that stops reading
  is cut off only once its queued output passes the bound, and the
  server's replies are small. `NetVideoHost` sets no TCP options on RTMP
  connections.
- Egress publishes a swarm it adds only on the networks the node runs
  when the stream opens. A stream of a swarm the node already runs
  doesn't compare its URI's options. A cancelled `open_reader` can hold a
  reader slot until its swarm goes. Each Data message allocates.
- An HLS stream ends at its stream's first error, with no recovery. HLS
  has no LL-HLS parts or `EXT-X-INDEPENDENT-SEGMENTS`, and its target
  duration can grow during a stream.
- The UI can't open an HLS stream: nothing calls HLSEgress.OpenStream, so
  a playlist URL comes only through the RPC.

The target shape follows. Nothing in the current services needs rework to
reach it.

Pipeline:
- Ingest sources produce access units, and muxers consume them. An access
  unit is a track, decode and presentation timestamps, a keyframe flag and
  a payload. Each track has a decoder config: an RFC 6381 codec string
  plus avcC, hvcC or av1C, or AudioSpecificConfig or OpusHead. Sources are
  RTMP, browser capture and transcoder output. Muxers are the fMP4
  segmenter, HLS egress and the frame format.
- Transcoded renditions come from a codec stage (ffmpeg or a library) that
  takes and returns access units. Segmenting and muxing stay in Rust.
- Ingest writes each access unit to the swarm as it arrives. In the fMP4
  format that means a moof/mdat pair per access unit, or per small group,
  within a segment's record. Check that the current player and Go egress
  accept that before relying on it.
- Each access unit keeps its publish time next to its media timestamps.
  The deadline-aware request scheduler and the players' playout clocks
  both need it.

Wire formats:
- The listing's MIME type names the format, and readers dispatch on it. A
  new format gets a new MIME type.
- While Go nodes share networks, the fMP4 format stays playable by the
  current UI and by Go egress. That format is one chunkstream record per
  segment, `[u16 init length][init segment][media segment]`, starting at a
  keyframe, in 1 KiB chunks with BLAKE2b-256 Merkle trees and Ed25519 live
  signatures. The rule for cutting segments is a parameter. The codecs in
  the MIME type come from the decoder configs.
- The frame format is one chunkstream record per access unit, with the
  decoder configs repeated ahead of each keyframe. Readers join and
  recover at a keyframe, so a lost chunk costs the frames up to the next
  keyframe.

RTMP and codecs:
- H.264 with AAC is the rendition every viewer can play. A streamer's HEVC
  or AV1 passes through unchanged as another rendition. Each viewer picks
  one with `VideoDecoder.isConfigSupported` or `MediaSource.isTypeSupported`.
  VP9 isn't a target: OBS can't send it over RTMP, and it compresses no
  better than HEVC.
- Listing several renditions needs a listing schema change (protocol v3).

Egress and players:
- The UI egress stays format-agnostic: payload bytes, record ends and
  discontinuities. Parsing the format is the player's job. In
  `strims-wasm`, egress runs in the swarm's worker, so a player in that
  worker can read records without the frontend RPC.
- The browser floor is the current generation: WebCodecs, plus
  ManagedMediaSource on iOS 17.1+. With that floor, the service-worker HLS
  relay and the MPEG-TS decoder go. ManagedMediaSource needs
  `disableRemotePlayback`.
- The baseline player muxes the frame format into fMP4 for MediaSource or
  ManagedMediaSource.
- The low-latency player decodes with WebCodecs in the worker. It renders
  to an OffscreenCanvas that the UI transfers in and plays audio through an
  AudioWorklet. It owns its jitter buffer, a playout clock driven by audio,
  A/V sync and keyframe recovery. VideoTrackGenerator (Safari) and
  MediaStreamTrackGenerator (Chromium) give it a native `<video>` where
  they exist; Firefox has neither.
- Browser capture encodes with WebCodecs into the frame format, through the
  same writer as RTMP. Input frames come from MediaStreamTrackProcessor
  where it exists and from a canvas elsewhere. Audio is Opus, since Firefox
  has no AAC encoder.
- Native HLS egress is a muxer over access units, for external players.

### Targets
- `strims-wasm`: a browser worker driver that replaces the Go wasm build and
  its JS bridge, keeping the TS UI and its RPC. Uses the browser's WebSocket
  and RTCPeerConnection. The frontend's RPC connection sends each framed
  Call as one message and uses `strims_rpc::Config::frontend()`; raise its
  `max_inbound_calls` if the UI needs more. Like `Node::open_vpn_port`, it
  gives the services vpn ports for `strims_rpc::vpn`: a stream of
  deliveries and a sink of addressed messages.
  Its `strims_kv::Kv` is IndexedDB: one database per kv table, an object
  store named `data`, string keys and `Uint8Array` values. A task woken by
  an IndexedDB callback has to be polled before the transaction
  deactivates. Its `SessionFactory` starts a profile's node, replication,
  network, bootstrap, transfer, directory, chat, whisper and video
  controls and peer services, runs `strims_session::run_vnic` on its
  node, and binds the network, bootstrap, directory, chat and video
  frontends (`strims_node::app::NodeFactory` is the native equivalent).
  Its video control has no RTMP listener (`NoVideoHost`) and serves no
  HLS. Its `HttpFetch` for a directory server's embed loads is the
  browser's `fetch`, which the embed APIs may refuse under CORS; decide
  whether such a server goes without embed snippets or loads them
  through a proxy.
- `strims-svc` runs one profile's node per process, on redb, and serves
  the UI's frontend over WebSocket. Remaining:
  - a Postgres `Kv`;
  - profile stores shared by several processes (cross-process change
    events, and a lease that spans them);
  - several profiles per process, on TCP and WebSocket listeners their
    nodes share, routed by profile key. Until then, a sign-up on a
    process that runs another profile creates the profile, which can't
    start until the process restarts;
  - rate limits on SignUp and SignIn, which each cost a 100k-iteration
    PBKDF2;
  - redb moved off the executor if its fsyncs show in profiles;
  - TLS for the API and the HLS listener (see `wss://` on native nodes
    below); until then the dev page reaches the API through the webpack
    dev server's proxy;
  - Origin checks on the API's upgrade, which accepts any origin;
  - WebSocket pings on API connections, so a UI that goes without
    closing its connection releases it;
  - headless sessions started at boot, for bootstrap nodes;
  - an HTTP health check;
  - on the HLS listener: keep-alive, Range requests, CORS preflight, and
    a cap per address. One client's slow connections can hold every slot
    for up to 10 s on the request head or 60 s on the response.
- Desktop (Electron sidecar or Tauri) and mobile (uniffi). strims-node's
  TLS runs on ring, which compiles C, so each cross build needs a C
  toolchain for its target.

### strims-ppspp defects
- Require monotonic sign-all timestamps until v3 binds the bin.
- `add_stream_close` is unused.

### strims-chat defects
- A whisper whose state write fails stays ENQUEUED outside the delivery
  queue until the queue is reseeded (a lease change or a lagged store
  subscription).
- A server left unserved at `Config::max_served` starts only when its
  record changes or the control restarts.
- A client stream that moves between a served and a read server reads the
  event window again, so it can repeat recent messages.
- `WatchWhispers` doesn't resend the records that changed while it lagged;
  the UI reads them again with `ListWhispers`.

### Cutover
Ship the protocol-v3 changes below behind a version bump, then retire the Go
implementation along with the interop tests that exist only for it.

Once Rust is the default implementation and the Go compatibility checks
have served their purpose, serve the UI's Debug service: metrics
(`ReadMetrics`, `WatchMetrics`) from the controls' metrics structs, a
profiling endpoint to answer `PProf`, and the debug config (12001) with
its mock streams published on a network for testing. Deleting that
network clears the config's network key. Until then the UI's debugger
and debug settings get no answers.

While profiles have Go and Rust devices:
- Go sends a bootstrap in one message, capped at 512 KiB on both sides, so
  a larger Go store can't bootstrap a new device.
- Go times a write at the wall clock, so after a clock step or within one
  millisecond, concurrent writes can resolve differently on Go devices.
- Go stops replicating to a device it deletes before sending the
  deletion, so the device doesn't learn of it from Go.
- A Go device sends an empty bootstrap when its store has no chat UI
  config, and doesn't push writes made before it opens a peer, so those
  reach other devices only with later changes.
- A device imported by Go records itself under id 0.
- Stores below version 7 are refused; a Go client upgrades them first.
- Media listing records are keyed by the first 30 bytes of their swarm id;
  rekey them once stores no longer move to Go.

While networks have Go and Rust nodes:
- Go negotiates networks with a link one run at a time and without a
  deadline, so a run that loses the other side (an `Open` that never
  arrives, a stale key count) stops it negotiating with that link until
  the link closes.
- Go members ping a directory every 10–14 minutes whatever its client
  config says, so keep directory session timeouts at 15 minutes or more.
- A Go directory server sends a new listing without its user count, so
  views show no users until the listing changes or the next refresh.
- A Go reader of a stream that starts over requests nothing until the new
  stream is as long as the old one, so it can miss a chat's unified asset
  bundle (sent after a forced sync, or when deltas would pass the asset
  window) until later deltas follow.
- Go chat servers let only `admin_peer_keys` moderate, which no Go client
  sets; they mute only users who have sent a message there, and answer
  `GetMute` with no mute.

## Open questions

- **Congestion control.** LEDBAT is a scavenger and fits a live stream poorly.
  It needs a cwnd floor and randomized 64-chunk request groups, which cost
  about 70 ms of p99 lag in well-provisioned meshes. `BbrLike` (selected with
  `SwarmOptions::request_controller`) halves tail lag behind capped seeds and
  recovers from RTT steps in about a second. It doesn't become the default
  until these are resolved:
  - Behind a seed with less upload than the stream bitrate, it delivers 1.5–2.5
    points less, because viewers' requests overlap and the seed uploads fewer
    distinct chunks. Scheduler-level decorrelation (rarest-first) should help
    both controllers.
  - Its p99 is about 9 % worse on sparse lossy meshes.
  - On a shared parent it yields to LEDBAT, so a network should run one
    controller. Peers on the v2 protocol all run LEDBAT, which ties a default
    switch to the protocol-v3 cutover.
  - Next: a deadline-aware request scheduler. Each chunk's deadline is its
    publish time plus the target lag. A peer's expected arrival time is its
    backlog ÷ rate + min RTT. Assign each chunk to the earliest peer that meets
    its deadline, and skip chunks no peer can meet. `RequestController`
    already exposes the rate, min RTT and answer delay this needs.
- **Push/pull hybrid.** Draft behind `push_mode`, off by default. Open items:
  - convergence on sparse meshes;
  - coalescing HAVE-only frames;
  - seed capacity reservation;
  - reciprocity versus tree children (children get the minimum weight);
  - push bypassing congestion control;
  - cycles longer than two;
  - the default stream count;
  - validation in mixed Go/Rust swarms;
  - the upload meter assumes the driver drains at link speed, so it reads low
    under vnic QoS, and swarms share one meter but admit independently.
    strims-node could report link capacity;
  - cancel a missed chunk at the parent to cut duplicates;
  - interior stream assignment is first-come;
  - slow parents are detected late, so `sim_push_slow_parent`'s p99 can still
    pass 1 s on some seeds. The upload meter's 2 s busy window takes about
    1.5 s to reach `SHED_UTILIZATION`, a child's deadline stretches with its
    parent's lag up to `DEADLINE_MAX`, and `PRUNE_MIN_AGE` and `SWITCH_HOLD`
    shield a new parent. Shedding on push queue wait helps slow parents but
    hurts `sim_push_parent_dies` and sparse meshes;
  - a seed admits every subscriber at startup, before its store rate has
    ramped up.
- **Adversarial-node simulation.** Whole-node hostile-peer scenarios need a
  test-only hook in `strims-node`. Automated attempts to build it were stopped
  by a safety check. Waiting on the user.
- **TLS for `wss://` on native nodes.** Native nodes don't dial `wss://`
  yet. tokio-tungstenite can take the rustls `ClientConfig` that
  `NodeFactory` builds once for the node's HTTPS
  (`strims_node::fetch::tls_client_config`).
- **Link and peer housekeeping:**
  - an idle timeout for established peers that send keepalives;
  - cache host certificates until near expiry;
  - QoS: a per-link gate if per-session blocking shows up in profiles;
    per-link and per-class rate limits; MTU vs exact backlog hints; a
    neutral starting weight for new non-ppspp writers;
  - vpn rejects messages larger than one frame (MTU − 4) at send time;
  - make vpn `VerifyMode::Enforce` the default once mixed networks have run
    clean under `Audit`;
  - lossy transports report loss, so direct vpn links can get a backup copy;
  - peer-exchange topology hints that suggest links to shorten long paths;
  - decide whether any app needs to stop a broadcast from being relayed;
  - peer exchange needs a tie-break (e.g. host id) for simultaneous
    `connect_peer` calls, which can drop the WebRTC link, and so do two
    nodes whose bootstrap clients dial each other at once;
  - `connect_peer` policy belongs to the app layer; the node never initiates
    it. Transfer discovery asks to link to every host a search finds (16
    per search), with no budget across transfers or against `max_peers`;
  - lowering the peer budget keeps the peers already connected; decide
    whether it should close the excess, and which peers go;
  - peers that send invalid data or false HAVEs are only backed off; scoring
    or disconnecting them is app policy;
  - whether a ppspp frame error should drop the peer rather than close its
    transfers until it relinks, and a minimum link MTU, since chunks that
    don't fit a frame are dropped.
- **ppspp overhead.** `TimeSet` allocates per node (one ring entry per munro
  would do); the data queue's `BTreeMap` index allocates; `channel_stats`
  sizes binmaps from their debug dump. One chunk per request costs about 27 %
  at 1 KiB chunks; requesting runs of chunks needs no wire change.
- **WebRTC send buffering.** strims-webrtc's buffered amount includes
  unacked bytes, so the node's cap also limits throughput per RTT. It needs an
  unsent-only count, a configurable low threshold, per-connection RTT, cwnd
  and counters, a per-socket reflexive-candidate cache, and explicit
  offer/answer calls.
- **Key material hygiene.** Private keys live in prost `Vec<u8>` fields and
  aren't zeroized.
- **NAT traversal.** WebRTC has no ICE-TCP (RFC 4571) and no TURN, which str0m
  doesn't support. Decide what relaying native and browser nodes need.
- **wasm memory budget.** The default live window is 2^16 × 1 KiB = 64 MiB per
  swarm, a chat's asset swarm grows up to 32 MiB for each room a node
  reads or serves, and a viewed video swarm or a capture holds 32 MiB. Set
  per-swarm budgets and a global cap before the wasm target ships.
- **Simulation coverage and fidelity.**
  - Waking an idle node to answer a query is a poll of its task and can shift
    later passes, so a sim's sampling cadence changes its trace. Make query
    wakes leave the schedule untouched, or keep treating the cadence as a
    scenario input.
  - Nodes stop reporting a channel's counters once it closes, so run totals
    and the clean-input check are lower bounds.
- **Network membership.**
  - A peer that shares no network after a run, or after a `Close`, stays
    linked; decide whether membership should close it, given replication
    and transfers.
  - Renewals sign under a new root the CA doesn't log, so a full-chain
    `Find` can't rebuild a renewed certificate's chain.
  - The server config's `root_cert_ttl_secs` and `peer_cert_ttl_secs` are
    unused; every certificate lasts two years.
  - A member whose certificate has expired can't renew: the CA verifies it
    in full. Decide how a lapsed member rejoins.
  - The CA's records live on the device holding the server config and
    don't replicate.
  - The CA event swarm isn't run; it carries nothing yet.
  - A network whose record gains a server config after its first write
    gets neither the owner's CA records nor its directory admin record.
- **Bootstrap.**
  - A published membership has no alias, and the CA keeps no record of
    it (no `NetworkPeers` entry), so its holder can't renew at the CA.
    The publisher could record it.
  - A peer is asked `GetPublishEnabled` once per link, so a change to
    its bootstrap config reaches linked peers when they relink.
  - Native nodes dial bootstrap clients over `ws://` only, with
    `allow_insecure_ws`.
  - `PeerService.ListNetworks` is unimplemented.
- **Directory.**
  - A refresh rewrites every listing, snippet images included, into an
    event swarm whose window is 2 MiB. Images may belong in the asset
    swarm (v3).
  - Snippet servers answer any member; decide whether only the directory
    server's host may subscribe.
  - A member whose event reader falls behind resets its view; only the
    fake-host tests cover that, not whole nodes.
  - A listing keeper stopped while its publish is in flight can leave the
    listing on the server until the publisher's session expires.
  - AngelThump's API takes one username per request, so every embed load
    costs a request per AngelThump listing, up to `Limits::max_listings`.
  - Embed loads don't track the APIs' quotas (YouTube's daily units,
    Twitch's rate limit): a load runs every `embed_load_interval` however
    many listings there are, and a refused batch is retried only at the
    next load.
- **Replication.**
  - A device that never syncs again keeps every log from collection;
    deleting the device frees them. Decide whether logs should expire.
  - Tombstones last as long as their logs, so after collection a write
    from a replica that missed a deletion can bring a record back.
  - Two devices that each record the same network under different ids
    keep their own record, since replicas can't see which one holds the
    CA.
  - Change events don't cross browser tabs sharing a store, so other tabs'
    views go stale until they reload.
- **WebRTC determinism.** DTLS bytes vary between same-seed runs, because
  dimpl's ECDHE and sctp-proto use OS randomness. Byte-level reproducibility
  needs seedable rngs upstream.

## Protocol v3 backlog

These are wire changes, held until the version bump.

### vnic
- Replace AES-CFB without a MAC with AEAD or Noise.
- Add proof of possession of the host key in the handshake.
- Make keepalives mandatory: every node sends them and announces support in
  `PeerInit`. Port 65535 stays reserved.

### vpn
- Make message IDs unique across networks, replacing the 16-bit seq that
  restarts on each start.
- Widen the 16-bit body length.
- Authenticate headers (AAD).
- Check trailer timestamps against replay.
- Bound trailer length and add a hop-limit field. 8 hops reach the far pairs
  that 5 can't.
- Authenticate unencrypted senders, which prevents reflection.
- Bind the peer-index record's host fields.
- Don't trust the publisher's clock in hash-table conflicts.
- Add a signed link-state message with a capability bit, and cheap per-hop
  authentication.
- Use or remove `Rejection`.

### ppspp
- Include the bin in live signatures; today chunks and munros can be
  replayed at other positions.
- Make HAVE claims verifiable.
- Stamp DATA at send time, not after the sender's queue.
- STREAM_REQUEST: add an expiry or lease plus a rate limit. STREAM_CLOSE:
  add a reason.
- Announce the interior stream and spare capacity.
- Mark pushed DATA and echo send times.
- Add a hop count or depth to STREAM_OPEN or HAVE.
- Add an epoch counter: `Writer::reset` within one millisecond keeps the
  epoch, so peers don't reset.
- Bump the protocol version.

### network
- Give runs an id, and answer `Negotiate` with the key count, so a stale
  message can't join a later run.
- Match `Open` bindings by network key, not position.
- Frame the broker stream per run, so a failed run's bytes can't reach the
  next.

### transfer
- Add the channel to `Close`: a close can't name the announce it undoes,
  so a delayed one stops a later binding, and each side must send one
  call per (peer, transfer) at a time.

### Protobuf evolution
prost drops unknown fields, so re-encoding a newer peer's message loses
them. Decide how forward compatibility works before changing any schema.

### Certificates and signed messages
Length-prefix or domain-separate the signed encodings; today fields can
shift across boundaries.

### Key derivation
Run the X25519 shared secrets that key vnic links and vpn messages through a
KDF before using them as AES keys, or use per-message keys.

### PSI
- Fix the cuckoo hash. Hash function 0 reaches only `nbins/gcd(256, nbins)`
  bins, so runs fail at random and need retries.
- Inputs shorter than 16 bytes are zero-padded, so different short inputs can
  collide.
