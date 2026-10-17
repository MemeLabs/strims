# Plan

Forward-looking work only. What exists is described by the code. How to work
on it is in `CONVENTIONS.md`.

## Remaining work

### App layer
Port the application services that sit on the core crates. The Go versions
(`internal/*`) are the behavioural reference.
- Network membership: PSI negotiation via `strims-mpc` (with a deadline, and
  retry with fresh seeds), certificate exchange, and CA. This replaces the
  explicit `bind_peer_network`.
- Directory, chat, notifications.
- Video ingress (RTMP), egress, chunkstream, HLS.
- The other dao record types arrive with their services: certificate logs,
  network peers, alias reservations, directory records, chat servers,
  emotes, modifiers, tags and profiles, video channels and configs, vnic,
  autoseed, debug and bootstrap configs, the network UI config,
  notifications and swarm caches. With them come many-to-one indexes, the
  CA records a new owned network creates, and the cascades of a network's
  deletion; until then, stores from Go keep those records unread.
- Import of Go bbolt stores into `strims-kv-redb` for native users.

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
  deactivates. Its `SessionFactory` starts a profile's node, replication
  and peer services (`strims_node::app` is the native equivalent).
- `strims-svc`: the headless service. It needs a Postgres `Kv`; profile
  stores shared by several processes (cross-process change events, and a
  lease that spans them); rate limits on SignUp and SignIn, which each
  cost a 100k-iteration PBKDF2; and redb moved off the executor if its
  fsyncs show in profiles.
- Desktop (Electron sidecar or Tauri) and mobile (uniffi).

### strims-ppspp defects
- Require monotonic sign-all timestamps until v3 binds the bin.
- `add_stream_close` is unused.

### Cutover
Ship the protocol-v3 changes below behind a version bump, then retire the Go
implementation along with the interop tests that exist only for it.

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
- **TLS for `wss://` on native nodes.** rustls needs a crypto provider:
  aws-lc-rs and ring compile C, and the rustcrypto provider is experimental.
- **Link and peer housekeeping:**
  - an idle timeout for established peers that send keepalives;
  - cache host certificates until near expiry;
  - QoS: a per-link gate if per-session blocking shows up in profiles;
    per-link and per-class rate limits; whether a rate limit of 0 means
    unlimited; MTU vs exact backlog hints; a neutral starting weight for
    new non-ppspp writers;
  - vpn rejects messages larger than one frame (MTU − 4) at send time;
  - pick an app default for `max_peers`; at 0, peer exchange accepts nobody;
  - make vpn `VerifyMode::Enforce` the default once mixed networks have run
    clean under `Audit`;
  - lossy transports report loss, so direct vpn links can get a backup copy;
  - peer-exchange topology hints that suggest links to shorten long paths;
  - decide whether any app needs to stop a broadcast from being relayed;
  - peer exchange needs a tie-break (e.g. host id) for simultaneous
    `connect_peer` calls, which can drop the WebRTC link;
  - `connect_peer` policy belongs to the app layer; the node never initiates
    it. Transfer discovery asks to link to every host a search finds (16
    per search), with no budget across transfers or against `max_peers`;
  - a default upload cap for `strims-svc`;
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
  swarm. Set per-swarm budgets and a global cap before the wasm target ships.
- **Simulation coverage and fidelity.**
  - Waking an idle node to answer a query is a poll of its task and can shift
    later passes, so a sim's sampling cadence changes its trace. Make query
    wakes leave the schedule untouched, or keep treating the cadence as a
    scenario input.
  - Nodes stop reporting a channel's counters once it closes, so run totals
    and the clean-input check are lower bounds.
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
