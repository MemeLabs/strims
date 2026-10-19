# Plan

Forward-looking work only. What exists is described by the code. How to work
on it is in `CONVENTIONS.md`.

## Remaining work

### App layer
- Video; see Video below.
- The UI's Debug service: metrics (`ReadMetrics`, `WatchMetrics`) from the
  controls' metrics structs, a profiling endpoint to answer `PProf`, and
  the debug config (12001) with its mock streams published on a network
  for testing. Deleting that network clears the config's network key.
  Until then the UI's debugger and debug settings get no answers.
- A server for the devtools UI (`src/devtools`): `DevToolsService`, and
  `CapConnService`, which lists, watches and loads ppspp capture logs.

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
- Autoseed: a seed that ends while streams or other seeds hold its swarm
  leaves the transfer published on the rule's network until they end. A
  rule that names a capture's or an ingest session's swarm seeds nothing.
  An ingest session that takes over a seeded swarm ends its seeds until
  the listings that match their rules next change.

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
  within a segment's record. Check that the current player accepts that
  before relying on it.
- Each access unit keeps its publish time next to its media timestamps.
  The deadline-aware request scheduler and the players' playout clocks
  both need it.

Wire formats:
- The listing's MIME type names the format, and readers dispatch on it. A
  new format gets a new MIME type.
- The fMP4 format stays playable by the current UI. That format is one
  chunkstream record per segment,
  `[u16 init length][init segment][media segment]`, starting at a
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
- Listing several renditions needs a listing schema change.

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
- `strims-wasm` runs a profile's node in a browser worker, and its PSI
  sessions in a second worker. Remaining:
  - the browser suites on Safari (iOS 17.1+) and Chromium. Only Firefox
    has run them, so elsewhere two things are unverified: IndexedDB
    transactions staying active across the executor's microtask polls,
    and data channels transferred to a worker;
  - one compiled module for both workers: the window compiles it once and
    posts the `WebAssembly.Module` to each, which needs the loader to
    export the asset URL;
  - `wasm-opt` in the release build; the shipped module is 5.3 MB, 1.6 MB
    gzipped;
  - WebRTC on strims-svc nodes, whose `AppConfig` has no WebRTC settings;
    until then browsers reach them over WebSocket only.
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
  - the UI and its node must come from one build, since the frontend
    API keeps no unknown fields: the UI could send its build id when it
    connects, and the API refuse another;
  - WebSocket pings on API connections, so a UI that goes without
    closing its connection releases it;
  - headless sessions started at boot, for bootstrap nodes;
  - an HTTP health check;
  - on the HLS listener: keep-alive, Range requests, CORS preflight, and
    a cap per address. One client's slow connections can hold every slot
    for up to 10 s on the request head or 60 s on the response.
  - invite shortlinks, for sign-up with little friction:
    `InviteLink.GetInvitation` over HTTP at `/api/invite` maps a short
    code to a stored invitation, and the UI's `/invite/<code>` page signs
    a newcomer up and into that network in one step. The retired Go
    service only read codes and nothing minted them, so this needs a way
    to create one too;
  - profile commands: list, add, remove, import and export profiles
    from the command line;
  - a container image. infra's leader, seeder and invites deployments
    still name the retired Go svc image.
- Desktop: the Electron app (`src/desktop`) spawns its node from
  `dist/desktop/svc`, which nothing builds; run strims-svc as that
  sidecar, or move to Tauri.
- Mobile: the `android/` and `ios/` apps have no node bridge; bind
  strims-node-core with uniffi.
- strims-node's TLS runs on ring, which compiles C, so each desktop and
  mobile cross build needs a C toolchain for its target.

### strims-ppspp defects
- Require monotonic sign-all timestamps until live signatures bind the
  bin (see Wire changes).
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
    controller.
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
  - make vpn `VerifyMode::Enforce` the default; it is `Off`;
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
- **Swarm cache cost.** Importing a chat room's asset cache verifies up
  to 32 MiB on the node task, and storing one seals up to 32 MiB on the
  executor. Measure both on wasm.
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
    swarm, named by the digests a snippet's signed body carries.
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
- **Store upgrades.** A node opens only a store at its `STORE_VERSION`,
  and nothing migrates one. Before a first release, decide between
  migrations and export and import.
- **WebRTC determinism.** DTLS bytes vary between same-seed runs, because
  dimpl's ECDHE and sctp-proto use OS randomness. Byte-level reproducibility
  needs seedable rngs upstream.

## Wire changes

No deployed peers constrain these, so each lands directly, without a dual
stack or a version gate.

### vnic
- Replace AES-CFB without a MAC with AEAD or Noise.
- Add proof of possession of the host key in the handshake.
- Make keepalives mandatory. Every node sends them already, so the
  tolerance for peers that never do (`IdleOk` and the drivers' readers
  that honour it) can go. Port 65535 stays reserved.

### vpn
- Make message IDs unique across networks, replacing the 16-bit seq that
  restarts on each start.
- Widen the 16-bit body length.
- Authenticate headers (AAD).
- Check trailer timestamps against replay.
- Bound trailer length and add a hop-limit field. 8 hops reach the far pairs
  that 5 can't.
- Authenticate unencrypted senders, which prevents reflection.
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

### Key derivation
Run the X25519 shared secrets that key vnic links and vpn messages through a
KDF before using them as AES keys, or use per-message keys.

### PSI
- Fix the cuckoo hash. Hash function 0 reaches only `nbins/gcd(256, nbins)`
  bins, so runs fail at random and need retries.
- Inputs shorter than 16 bytes are zero-padded, so different short inputs can
  collide.
