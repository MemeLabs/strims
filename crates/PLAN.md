# Plan

Forward-looking work only. What exists is described by the code. How to work
on it is in `CONVENTIONS.md`.

## Remaining work

### App layer
- Video; see Video below.
- A server for the devtools UI (`src/devtools`): `DevToolsService`, and
  `CapConnService`, which lists, watches and loads ppspp capture logs.

### Video
Live video is the unit format (`strims_video::units`) end to end: RTMP and
browser capture write it, swarms carry it, and the native player
(MediaSource), the opt-in low-latency player (WebCodecs) and HLS egress
read it.
- **Services.**
  - The VideoIngressShare service isn't served; it needs a design
    (discovery under a key and salt, the caller's identity from its vpn
    certificate, quotas on the host's ingest and upload, the client and its
    UI). The ingress form hides its `service_network_keys` until then.
  - One ingest per channel holds within a process. Stores shared by several
    processes need the lease that spans them (see `strims-svc` under
    Targets).
  - RTMP: a connection publishes once (OBS, ffmpeg and Larix reconnect to
    publish again); shared objects are ignored; a peer that stops reading
    is cut off only once its queued output passes the bound.
  - Egress: each Data message of the RPC path allocates. On the writer's
    own node, an RPC stream opened well into a capture replays about the
    whole live window at once (about 2 MB in its first 4 s at 25 s in);
    find where its reader starts. Without a listing, one stream with other
    swarm options moves a slot's other holders until the next allowed
    reopen.
  - `SwarmReader` and `SwarmWriter` close themselves on drop with
    `try_send`, so a full node command queue loses the close and leaks the
    slot; a cancelled `create_swarm` or `add_swarm` leaks the same way.
  - The signing rule (`UnitSigning::for_stream`) is untested between 0.75
    and 1.5 Mbit/s and above 6.3 Mbit/s. A farm A/B at 1 Mbit/s (`suite.py`'s
    `sign` tiers) settles whether 2 or 4 chunks lag less there. Streams
    whose bitrate swings widely (VBR scenes) pad more at its 300 ms: the
    sweep kept 8-chunk padding within a tenth of such a stream only at an
    interval of about 500 ms.
- **Stream ends.** A viewer that joins after a stream's END starts at the
  newest chunk, padding, and never sees it. A writer keeps its swarm
  `END_LINGER` after signing its END, blind to whether any peer took it;
  keeping it until peers hold the END's chunk needs a ppspp API reporting
  which peers hold a bin.
- **Players.**
  - The deadline-aware request scheduler (Swarm, item 4) reads the native
    player's playout position, from `UnitsPlayer`'s map of presentation to
    publish times.
  - `UnitsPlayer`'s window path batches by hand; move it onto `feed.ts`'s
    `WindowBatches`.
  - HE-AAC and PCE-layout audio get no silence fill, in the player or HLS:
    their holes stretch the last frame, which puts Chromium out of sync by
    the hole. HLS fills holes only retroactively, so audio that stops for
    good while video goes on ends its segments short.
  - Untested: iOS 17's window ManagedMediaSource fallback, the iPhone's A/V
    sync through the silence fill, and the rendition choice on an iPad,
    with a real directory's listing latency, and while casting. A
    rendition the worker's MediaSource refuses after the window accepted
    its type fails through the player's restarts and isn't remembered.
  - Measure the iPhone's playback energy (native player and swarm node)
    with pymobiledevice3 once the phone can be plugged into a host.
  - The low-latency player stays opt-in (`?player=lowlatency`): it saves
    0.2-0.3 s for about 1.5x the CPU on Chromium and 2-3x on Firefox, which
    has no track generator. Revisit if Firefox ships `VideoTrackGenerator`;
    before it becomes a default anywhere, check whether its ±2% resampled
    rate control is audible and measure device CPU and iOS's display
    latency under `VideoTrackGenerator`.
  - `VideoControls`' `remoteButtons` slot holds the rendition menu too;
    name it for what it holds.
- **Capture.** Turning the iPhone during a capture, and screen capture on
  real devices, are untested; a capture started upside down
  (landscape-secondary) isn't caught by the aspect check.
- **Renditions.** Autoseed rules seed only a listing's main swarm, not its
  renditions. OBS multitrack sources, switching renditions by bandwidth,
  and large forward jumps in FLV tag times are left.
- **Subtitles.** CC1 only; 708 is carried but not decoded; AV1 sources'
  T.35 captions are lost in their H.264 renditions. Safari's in-band 608
  ignores the erase code, so each caption shows until the next. ffmpeg
  reading the multivariant playlist skips the first subtitle segment. One
  subtitle language: `onTextData`'s `trackid` and `language` are ignored.
- **Cast and AirPlay** play HLS egress; neither has played on a real
  receiver.
  - Check on a Chromecast that its default receiver plays Opus in fMP4
    HLS, low-latency playlists and the subtitle renditions, and whether it
    fetches from a plain-HTTP LAN address.
  - Check on an Apple TV that the handoff plays, which URL the receiver
    fetches, and whether iOS reports a receiver available before its first
    use: with uxplay it read "not-available" until picked once, which
    would hide the AirPlay button.
  - A receiver that keeps failing flips the video between wireless and
    local every few seconds; debounce, or give up after a few.
  - Cast's media title and thumbnail; an `hls_reopens` metric through
    node-core's plumbing.
  - HTTP/2 connections can hold a slot idle for 30 s, which makes a
    per-address connection cap on strims-svc's listeners matter more.
  - A custom web receiver running the wasm node and `UnitsPlayer` would
    cast a stream no reachable node serves, if the receiver runtime has
    WebRTC data channels and WebAssembly and a Chromecast's CPU carries the
    node; build it only if that case matters.

### Targets
- `strims-wasm` runs a profile's node in a browser worker, and its PSI
  sessions in a second worker. Remaining:
  - the browser suites on Safari (iOS 17.1+) and Chromium. Only Firefox
    has run them, so elsewhere two things are unverified: IndexedDB
    transactions staying active across the executor's microtask polls,
    and data channels transferred to a worker;
  - real devices in the suites and the testbed. A phone, or Chrome or
    Firefox on Windows where Docker's headless builds fall short (no
    hardware decoding, no HEVC), opens `e2e.html` itself over the LAN and
    dials its control socket. The `Site` then needs a LAN listener on
    HTTPS with a dev CA (`hack/tls`), since WebCodecs and AudioWorklet
    need a secure context. Safari starts audio and grants the screen wake
    lock only from a tap, so one tap does both, and the page never
    navigates so the tap lasts. The page forwards its console, and a tab
    iOS kills shows up as a reload. Nothing restarts it from the socket's
    side; pymobiledevice3 over USB could. A device's link is outside the
    testbed's shaped network;
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
  - a Prometheus `/metrics` endpoint rendering the Debug service's
    snapshot. infra's `mock-stream-exporter` scrapes `:1971/metrics` for
    delimited `io.prometheus.client.MetricFamily` messages and reads the
    gauge `strims_debug_mock_stream_latency_ms{id,swarm_id}`, each mock
    stream reader's last segment latency;
  - on the HLS listener: keep-alive, Range requests, CORS preflight, and
    a cap per address. One client's slow connections can hold every slot
    for up to 10 s on the request head or 60 s on the response.
  - invite shortlinks, for sign-up with little friction:
    `InviteLink.GetInvitation` over HTTP at `/api/invite` maps a short
    code to a stored invitation, and the UI's `/invite/<code>` page signs
    a newcomer up and into that network in one step, and a way to mint
    codes;
  - profile commands: list, add, remove, import and export profiles
    from the command line;
  - a container image, for infra's leader, seeder and invites
    deployments.
- Desktop: the Electron app (`src/desktop`) spawns its node from
  `dist/desktop/svc`, which nothing builds; run strims-svc as that
  sidecar, or move to Tauri.
- Mobile: the `android/` and `ios/` apps have no node bridge; bind
  strims-node-core with uniffi.
- strims-node's TLS runs on ring, which compiles C, so each desktop and
  mobile cross build needs a C toolchain for its target.

### Capture
- strims-wasm nodes capture nothing: the browser driver has no file to
  write (OPFS, or dumps streamed to the UI).
- A capture runs for its node's life, from a driver flag. A runtime
  toggle (the Debug service's config) is a stored-record change.
- Tell a pull answer from a duplicate push in the receiving side's tap;
  today that takes the sender's dump.
- A mode that keeps DATA messages' headers and drops their payloads.
  Dumps grow about as fast as the node's traffic, nearly all of it
  incompressible payload; this would cut them about eightfold.

### Fuzzing
- Property tests bound most parsers, but nothing is fuzzed with coverage
  guidance, and random bytes rarely pass a magic number or a length check.
  Give every parser of bytes from peers, the network, publishers or files
  a cargo-fuzz target at the entry point that reaches it (a nightly
  toolchain beside stable, for the targets alone), seeded from the golden
  vectors and the tests' synthetic inputs:
  - ppspp's wire messages and the capture reader; strims-chunkstream's
    layouts; strims-rpc's framing and the protobuf messages it decodes;
    vnic's frames and session descriptions; vpn, network and kademlia
    messages; directory listings and transfer records; chat's message
    parser; dao's stream keys; strims-crypto's key and signature
    decoding; strims-webrtc's STUN parsing.
  - Video: the RTMP server's byte stream, which reaches FLV, AMF0, avcC,
    SPS, AudioSpecificConfig and OpusHead; the unit-format reader over
    the chunked layout; the HLS routes; and, on branch `video-target`,
    strims-player's remuxer and demuxer.
  - A short run of each target in `hack/check.sh --full`, and long
    campaigns on the sim farm (quoted with `farm.py estimate`). Each
    crash becomes a regression test beside its parser's property tests.

### Simulation farm
- `long_run`'s heap grows about 4 MB over its 10 minutes even without
  its crashes, in allocations under 400 KiB (no larger one grows), which
  puts it near its check's bound; find whether the nodes or the harness
  hold it.

### Swarm
Hybrid (`PushMode::Hybrid`) is the design: push trees carry the stream
and pull repairs what they miss. Its seed is the adaptive seed
(`SeedAllocation::Adaptive` with `SeedQos::ByAllocation`), which also
holds the defences against greedy peers. The groups are in priority
order, and so are the items in each. Ideas already tried and dropped are
in `TRIED.md`.

1. **Ship it.** Swarms run Hybrid with the adaptive seed by default, and
   each later trial should measure against the defaults it would change.
   - Choose a media swarm's stream count from its bitrate. Fewer streams
     do better at 2-4 Mbit/s (1 or 4 against 16 behind a cable seed and in
     the ppspp sim's A/B scenarios) and 16 at 8-12 Mbit/s; media swarms
     run 16 meanwhile. An ingest could take the
     bitrate from the publisher's metadata, a capture from its encoder.
     The matrices that chose 4 KiB chunks and 32 KiB munros ran 10 s
     streams, which weigh startup heavily: rerun `defaults-cable` and
     `defaults-ab` at 2-minute streams to confirm or revise them and the
     stream count.
   - Without a known capacity the upload meter still trails a known
     uplink. Measure the gap on the robust tier by role
     (`STRIMS_SIM_KNOWN_UPLINK=seed` and `=viewers`) before choosing the
     next meter change.
   - A relay with few links reads a standing queue once each backs up a
     window, so its drained reading is what those links deliver, not its
     uplink: star-3-10's viewers (20 Mbit/s uplinks) read about 550 KB/s
     and declined extra streams at half that. Clamping on the least latest
     excess across its links lost a thin relay's own queue (`TRIED.md`).
   - One peer whose link blocks keeps its data queued, so the meter reads
     the node busy: its parent sheds subscribers and an adaptive seed's
     budget falls to that link's rate. Count busy only while every peer
     that had data queued still has (judge it in
     `wan_slow_viewer_adaptive_uncapped` with `STRIMS_SIM_TCP`). The
     Docker testbed's media runs (branch `video-target`; outputs in
     `target/video/evidence/testbed-viewers-runs/`) show it on the real
     stack: one viewer on a 4.5 Mbit/s downlink in a five-node mesh at
     3 Mbit/s took the seed's children from 32 to 1 and put every viewer
     4-13 s behind for 30 s (runs `slow-a` and `slow-b`).
   - On the Docker testbed a 10 Mbit/s seed of a 3 Mbit/s stream in a
     five-node mesh grows its push children from 16 to 50 every 15-20 s
     and fills its uplink, and every viewer's arrivals then run 0.5-1.3 s
     late (up to 2.1 s): most of that mesh's stalls. Reproduce it in the
     sim before choosing a fix (runs `mesh-b` and `mesh-margin`).
   - Pushes queue toward a peer whose connection is backed up past one
     window, to ppspp's cap, and are dropped as stale only when written;
     and a viewer that falls a window behind sends its neighbours messages
     they count as invalid (strims-sim's slow viewer).
   - After its uplink halves, an adaptive seed's viewers take 10-17 s to
     recover (the robust tier's halve-x-10 cells,
     `collapse_halving_lock_in`). Before the halving the seed runs near its
     full uplink with copies to spare: buffers below the runner keep its
     busy time near zero, so its budget reads twenty times its rate and
     only the window bounds its copies, and the halving lands on a backlog
     the budget sees a second or two late. Find a headroom signal the
     seed's own munro bursts don't trip (the relay bound does: `TRIED.md`).
2. **A residential seed at a real bitrate.** Behind a cable seed at
   6-8 Mbit/s honest viewers lag several times what they do behind fiber.
   - **Pull from a constrained seed.** Bound the answers a constrained
     seed sends for each fresh chunk. With 100 viewers at 6 Mbit/s and
     1 KiB chunks (the ppspp sim's ignored `wan_cable_seed`) its push
     tree held, but its other neighbours pulled the newest chunks from it:
     it sent each chunk 4 times, its uplink stayed 89 % busy and the
     median viewer lagged 4 s. Recheck with paced answers and 4 KiB
     chunks. Candidates, none a wire change: answer a
     fresh chunk to a bounded number of neighbours, by the seed's own
     count of what it sent; a viewer's requests to a peer whose answers
     lag (its own measurement) yield to peers that announce the chunk,
     and queued ones are cancelled; gate pull from the seed by deadline.
   - **Joins' backlogs.** On the Docker testbed, while a viewer joining
     fetches its first GOP's backlog (130-600 KB), the others' live
     arrivals run up to 1-1.5 s late (runs `mesh-a` at 23-26 s, `mesh-b`
     at 26 s). Find whether the seed's uplink or the joiner's requests
     take the time, and whether backlog answers should yield to pushes.
     Backlog chunks are never announced, so viewers joining the same GOP
     can't serve each other; all of it comes from the seed
     (`collapse_join_backlog_hold`).
   - **Holders' shares under pull.** `SeedQos::ByAllocation` gives a lone
     holder 1000 / (1000 + 50 × the other neighbours) of the uplink,
     which with 32 neighbours is barely what a 6 Mbit/s copy needs. A
     holder's weight should cover its pushes whatever the neighbour
     count.
   - **Window steps.** The window and the budget step in whole copies,
     too coarse for the larger chunks that higher bitrates need to keep
     protocol overhead reasonable; a finer step needs the holder rules (a
     holder takes every stream) to allow it. Once doubled, the window
     keeps two copies even where the uplink pays for one (strims-sim's
     cross traffic, whose seeds commit about twice their cap).
   - **Budget hysteresis in copies.** The budget's hysteresis is in
     slots, so at a copy boundary a holder joins and leaves with each
     swing.
   - **Protocol overhead.** Find where the bytes go at the media swarms'
     4 KiB chunks, from a capture. Coalesce HAVE-only frames. One chunk
     per request costs about 27 % at 1 KiB chunks; requesting runs of
     chunks needs no wire change. Ping per peer, not per channel: every
     channel to a peer shares its link, but each pings once a second for
     its controller, so peers that share a media, chat and directory
     swarm ping three times over. One ping loop per peer in the runner
     would feed every channel's controller to that peer, riding the first
     channel payload written to it, with no wire change.
     It changes what the controllers see: gate it on the ppspp sim's A/B
     matrix and strims-sim's WAN cases.
3. **Greedy peers.** Free riders and liars cost honest viewers more tail
   lag than anything but a constrained seed, most of it on the child
   side.
   - **Pull from liars.** A request that timed out may go back to the
     same channel after `REQUEST_BACKOFF` (200 ms), and LEDBAT's floor
     reopens its window, so liars among a viewer's few neighbours keep
     holding its requests (the ppspp sim's lossy mobile viewers lag
     about 9 s at p99 against 2 s without them).
   - **HAVEs past the head.** A HAVE far past the stream's head moves a
     non-seed's newest bin time and prunes its bin times, so deadlines
     and HAVE lags lose their start until the stream catches up. It
     needs a bound on how far past the newest chunk a peer may announce.
   - **Liars after a bar.** Reports bar a liar a few seconds after it
     takes a copy, but the bar lapses after `HOLDER_BAR` and the liar
     takes a copy again, at the price of its would-be children's grafts
     and misses until the reports bar it once more (WanSuperPeer at
     60 s). Decide whether a holder barred as unserved stays out longer,
     or holds again only once reports show it serving as a relay.
   - **Answers to silent neighbours.** A seed answers the requests of a
     neighbour that never announces what it receives like anyone's, so
     free riders share what pushes leave of the uplink with honest
     viewers.
   - **Free riders at relays.** A relay admits and keeps a child whatever
     the child gives back, so a free rider takes a slot of a relay's
     interior stream like anyone.
   - **Colluders.** A verdict is a majority over a holder's reporters, so
     greedy viewers that praise a liar or defame an honest holder decide
     it where they outnumber its honest reporters: holders with few (far
     ones, and at 6 Mbit/s those whose children aren't the seed's
     neighbours). In the ppspp sim's `SIM_COLLUDE` rows they get honest
     holders barred and save no liar. Weigh a reporter by how long the
     seed has seen it announce, or require a quorum that grows with the
     reporters seen; decide whether colluders join the default greedy
     mix.
   - **ANNOUNCE's reach** (Wire changes, ppspp), then **holders nobody
     grafts onto.** The seed picks holders by its round trip to them and
     children pick parents by announcement lag, so a far holder can
     relay to nobody, and a near one with a weak uplink to few. No
     reporter votes on them, and they keep their copies. Decide whether a
     copy should go to a neighbour the reports show serving as a relay.
4. **Pull on lossy, sparse and mobile paths.** The worst scenarios after
   a constrained seed, where pull does the most repair.
   - **The silence after a timeout.** LEDBAT's 2 KiB minimum, under a
     4 KiB chunk, leaves a channel asking for nothing for
     `FLOOR_LOSS_HOLD` after a timeout. The ppspp sim's pull-heavy cells
     lose up to a fifth of p99 to that silence, and the real stack's busy
     stars lose as much without it (tried: TRIED.md). Find what it buys
     there, likely a congested peer left to its pushes, and keep the
     silence for that alone. It also stalls joins: a joiner pulls its
     GOP's backlog from the seed alone, the seed's keyframe pushes queue
     those answers past the congestion timeout, and the window then sits
     under a chunk for the hold. Two viewers joining in the same second
     take 3-5 s for their backlogs, against 1.1 s for one and 1.7-2.4 s
     without the hold (strims-sim's `collapse_join_backlog_hold`, on
     branch `video-target`).
   - **Deadlines from the player.** Readers read chunks as soon as they
     arrive and never skip, so a deadline taken from first sight has no
     slack to spend and only moves repairs earlier (tried: TRIED.md).
     Once players keep playout clocks (Video), a chunk's deadline is its
     publish time plus the player's target lag: wait longer for pushes
     while pull can still meet it, and skip chunks no peer can meet.
   - **Request decorrelation.** LEDBAT keeps viewers' requests apart
     with a cwnd floor and randomized 64-chunk request groups, which cost
     about 70 ms of p99 in well-provisioned meshes; scheduler-level
     decorrelation (rarest-first) could replace the groups. Plain oldest
     first tied (TRIED.md).
   - Convergence on sparse meshes.
5. **Push-tree refinements.** Smaller or narrower costs.
   - Slow parents are detected late, so `sim_push_slow_parent`'s p99 can
     still pass 1 s on some seeds. The upload meter's 2 s busy window
     takes about 1.5 s to reach `SHED_UTILIZATION`, a child's deadline
     stretches with its parent's lag up to `DEADLINE_MAX`, and
     `PRUNE_MIN_AGE` and `SWITCH_HOLD` shield a new parent.
   - A non-seed's interior stream is the first it accepts a subscriber
     for. Its extras are admitted at `EXTRA_UTILIZATION` of the measured
     capacity, so a datacenter viewer forwards every stream it holds.
   - Reciprocity versus tree children (children get the minimum weight).
   - Cycles longer than two; a hop count (Wire changes, ppspp) would
     show them.
   - Push bypassing congestion control. Judging it needs the WAN model's
     sender backpressure (Simulation coverage and fidelity).
   - **Copies by region.** Copies go to the nearest neighbours. Try a
     scenario whose nearest neighbours share one path or access network.
   - **Reciprocal topology.** Nodes keep their links whatever a peer
     reciprocates. Dropping links that don't reciprocate and dialing
     random new peers, with a budget for leechers, would move high-upload
     peers toward the centre of the swarm: the half of the topology a
     seed's allocation can't reach. The untraversable-NAT model
     (Simulation coverage and fidelity) would show what leechers cost.
   - **Allocations.** `TimeSet` allocates per node (one ring entry per
     munro would do); the data queue's `BTreeMap` index allocates;
     `channel_stats` sizes binmaps from their debug dump.
   - **Record swarms.** Chat's and the directory's event and asset swarms
     run pull only (`PushMode::Off`). Measure Off, EagerHave and Hybrid on
     a chat-like workload (sparse, small, signed records) before they take
     the default.
6. **The admitting seed** (`SeedAllocation::Admission`). Decide whether
   it stays; while it does:
   - a startup window: before its upload and store rate are measured it
     admits more than a constrained uplink carries, and at 6 Mbit/s its
     pushes go stale. The adaptive seed's window counts whole copies at
     holders; this one needs slots, with shedding down to it;
   - it keeps a greedy subscriber's slot for as long as it asks; the
     silent-holder rule is the adaptive seed's.

### strims-ppspp defects
- Require monotonic sign-all timestamps until live signatures bind the
  bin (see Wire changes).
- Remove the unused `add_stream_close`.

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

- **Adversarial-node simulation.** Whole-node hostile-peer scenarios need a
  test-only hook in `strims-node`. Waiting on the user.
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
- **WebRTC send buffering.** strims-webrtc's buffered amount includes
  unacked bytes, so the node's cap also limits throughput per RTT. It needs an
  unsent-only count, a configurable low threshold, per-connection RTT, cwnd
  and counters, a per-socket reflexive-candidate cache, and explicit
  offer/answer calls. A WebRTC link also holds up to `max_buffered_amount`
  (256 KiB) in FIFO order below QoS, about 420 ms at 5 Mbit/s: the queue
  that ppspp's frame cap keeps short on TCP links. Measure what it costs
  on the testbed with browser viewers.
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
  - WAN paths (`strims-wan`) turn a lost packet into a retransmission delay
    but model no TCP sender's congestion response, so a lossy connection
    keeps its full rate. Send buffers (`Scenario::send_buffer`) push back
    on a sender only through its uplink, not through one slow path (a slow
    receiver, a congestion window over a long round trip). Only the
    uncapped WAN cases run them: with them on (a 100 ms uplink queue)
    `wan_mobile_viewers_pull`'s LTE p99 passes its 3.5 s bound on one seed
    of 8. Nor are retransmission timeouts modelled: beside a competing
    upload the sim's worst second of lag is half a real link's.
  - Against the testbed (`hack/testbed`, `calibration_star` with
    `STRIMS_SIM_TCP`), below saturation the sim's uplink shows no queueing
    where a real link reads about 50 ms at p90 from bursts, behind 100 ms
    queues its queueing delay reads half the real link's, and the window
    model holds our data in its senders' queues far longer than real TCP
    (at 0.8 of the uplink, one-way delay about 120 ms over its base
    against 5 ms). Close these before trusting a ranking that turns on
    the delay's level.
  - Through a halving of the seed's uplink the testbed fell into the
    lock-in deeper than its twin, and its round trips there ran 1.4 times
    the sim's. Recheck with today's frames; if the gap holds, find the
    per-byte overhead on the seed's link.
  - Loss is independent per packet, and jitter changes every 20 ms per
    end. Bursty mobile loss and satellite handover outages need their own
    processes.
  - No WAN scenario runs WebRTC yet. Show that a WAN run over WebRTC
    replays over 16 seeds (DTLS handshake sizes vary between runs and are
    charged a fixed size), then add one. Browser seeds stream over WebRTC,
    so the upload meter's defaults need it measured before they cover
    them.
  - Every pair a sim links can reach each other. A model of peers that
    accept no inbound links (an untraversable NAT) would show what they
    cost the mesh and the seed's allocation, and whether either should
    treat them as leechers.
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
- Elide the padding of units signed early. `Writer::flush` pads a partial
  unit with zeros to a whole unit and sends them, so a stream signed early
  on a timer is sent at no less than unit size ÷ interval
  (`strims_video::units`'s signing). A live signature would cover how
  many of its unit's chunks hold data, and peers would neither request,
  push nor announce the rest. A receiver would check them as zero leaves
  against the signed root. This lands with the bin item above, since both
  change what the signature covers. Branch `video/ppspp-sign-gated` (on
  main a356f676) holds an implementation: zero-leaf verification,
  data-less chunks held as zeros and left out of HAVEs, requests for them
  refused or split, and the hybrid's miss deferral for munros a node
  hasn't seen, applied once a swarm shows a data-less chunk. Applied to
  every swarm, the deferral slows the pruning of starved parents and grows
  cycles. The gated branch hasn't been through the sim suite.
- Make HAVE claims verifiable.
- Stamp DATA at send time, not after the sender's queue.
- STREAM_REQUEST: add an expiry or lease plus a rate limit. STREAM_CLOSE:
  add a reason.
- Announce the interior stream.
- Mark pushed DATA and echo send times.
- Add a hop count or depth to STREAM_OPEN or HAVE.
- ANNOUNCE's reach: a report names the reporter's direct parent and
  reaches only the seed's own neighbours, so a holder whose children
  aren't among them goes unjudged (at 6 Mbit/s the seed links to 32 of
  100 viewers). Name the reporter's root holder, carried down the tree
  (with the hop count above), or relay reports up the tree, signed by
  viewer identities the seed can check.
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
