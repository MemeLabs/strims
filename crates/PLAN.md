# Plan

Forward-looking work only. What exists is described by the code. How to work
on it is in `CONVENTIONS.md`.

## Remaining work

### App layer
- A server for the devtools UI (`src/devtools`): `DevToolsService`, and
  `CapConnService`, which lists, watches and loads ppspp captures (the page
  reads the old `CapConnLog` format, not `.ppcap` dumps).
- The UI warns a user whose device clock is set far enough wrong to break
  links and records, judged against the server's or peers' time, and says
  how to fix it.

### Video
- **Services.**
  - The VideoIngressShare service isn't served; it needs a design
    (discovery under a key and salt, the caller's identity from its vpn
    certificate, quotas on the host's ingest and upload, the client and its
    UI). The ingress form hides its `service_network_keys` until then.
  - Egress: each Data message of the RPC path allocates; an RPC stream on
    the writer's own node replays the live window from its tail, and
    should start near the head.
  - `SwarmReader` and `SwarmWriter` close themselves on drop with
    `try_send`, so a full node command queue loses the close and leaks the
    slot; a cancelled `create_swarm` or `add_swarm` leaks the same way.
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
    good while video goes on ends its segments short; Safari doesn't start
    a variant whose segments carry video and no audio (seen with a feed
    whose audio ran 49 s behind its video in tag order), and HLS has no
    silence that follows the video as the player has.
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
- **Signing.** A unit signed early sends no padding, so its interval
  trades only signatures a second against sign waits: re-choose
  `UnitSigning`'s intervals and `SMALL_BELOW_BPS` (the `sign` tier with u1
  and u8 at t100-t300, on both sides of 750 kbit/s; only 1- and 8-chunk
  units).
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
  - A custom web receiver running the wasm node and `UnitsPlayer` would
    cast a stream no reachable node serves, if the receiver runtime has
    WebRTC data channels and WebAssembly and a Chromecast's CPU carries the
    node; build it only if that case matters.

### Targets
- `strims-wasm` runs a profile's node in a browser worker, and its PSI
  sessions in a second worker. Remaining:
  - the browser suites on Safari (iOS 17.1+) and Chromium; only Firefox
    has run them;
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
  - on the HLS listener: keep-alive over HTTP/1.1, Range requests, CORS
    preflight, and a cap per address, so one client's slow or idle
    connections can't hold every slot.
  - invite shortlinks: a server for `InviteLink.GetInvitation` at
    `/api/invite` (the UI's `/invite/<code>` page already asks it), sign-up
    in the same step for a newcomer, and a way to mint codes;
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
- Captures from the app's page node (OPFS, or dumps streamed to the UI).
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
  - Video: the RTMP server's byte stream and the parsers it reaches; the
    unit-format reader over the chunked layout; the HLS routes;
    strims-player's remuxer and demuxer.
  - A short run of each target in `hack/check.sh --full`, and long
    campaigns on the sim farm (quoted with `farm.py estimate`). Each
    crash becomes a regression test beside its parser's property tests.

### Sim suite
- `suite.py farm --refs` runs every arm, the base included, though an
  earlier farm run may hold the base's rows at the same cells; only
  `score` reuses them, by hand. Reuse a matching base by itself: find a
  run whose arm builds the same code for the cell's sim at the same cell
  and env, run only the new arms at that run's seeds, and score them
  against its rows.
  - Match on code, not commits: a rebase changes the sha and keeps the
    code. `build_key` hashes everything under `crates/`, docs and crates
    the sim doesn't build included; key on the sources of the sim's
    dependency closure.
  - Where the code differs only in strims-sim's harness, which should
    leave the timeline alone, rerun the base at a couple of the run's
    seeds and reuse the rest only if their traces match.

### Swarm
Hybrid (`PushMode::Hybrid`) is the design: push trees carry the stream
and pull repairs what they miss. Its seed is the adaptive seed
(`SeedAllocation::Adaptive` with `SeedQos::ByAllocation`), whose reports
judge its holders; every hybrid node declines silent peers. The groups
are in priority order, and so are the items in each. Ideas already tried
and dropped are in `TRIED.md`.

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
   - Streams whose bitrate swings widely trail what they did with the
     early units' padding sent (`sign-sens` at u8,t300, 32 seeds:
     1500v:s1.2 lag p99 x1.05-1.14; overload x1.14 [1.02,1.27]). Relays'
     meters and written peaks read the quiet scenes, and over 8-chunk
     units the last streams of each half of 16 go 3-18 s without data at
     1500v. Try relay readings that count a unit's data-less chunks, as
     the store rate does.
   - Emit every stream's chunks at a steady cadence whatever the
     bitrate, padding what a quiet scene leaves (padding is elided, so a
     data-less chunk costs little), so a neighbour can predict when a
     chunk is held: speculative requests to peers that haven't announced
     a chunk, and declared lag summaries in place of HAVEs, assume it.
   - Without a known capacity the upload meter still trails a known
     uplink. Measure the gap on the robust tier by role
     (`STRIMS_SIM_KNOWN_UPLINK=seed` and `=viewers`) before choosing the
     next meter change.
   - A relay with few links reads a standing queue once each backs up a
     window, so its drained reading is what those links deliver, not its
     uplink: star-3-10's viewers (20 Mbit/s uplinks) read about 550 KB/s
     and declined extra streams at half that. Clamping on the least latest
     excess across its links lost a thin relay's own queue (`TRIED.md`).
   - Thin relays collapse on browsers. Behind a 10 Mbit/s seed, 15 viewers
     on 3 Mbit/s uplinks in a full mesh (256 KiB/s, 4 KiB chunks and 32 KiB
     munros: `collapse_thin_relays`'s cell, healthy on TCP): on the Docker
     testbed's 16 browsers every viewer falls about 16 s behind, and the
     browser twin collapses all 32 seeds, though with the egress pacer it
     delivers 99.6 % of the stream (73.5 % without). Each of a relay's
     peers costs its uplink as data alone doesn't: its HAVE frames and
     probes, and a SACK for nearly each packet sent (dcSCTP sets the I bit
     while a window is under ten packets), and a full mesh leaves each
     association a fifteenth of the uplink. Browsers held to nine peers
     were cured on both (`TRIED.md`), but a fixed budget fits one config:
     the cure is a pool that follows from what a node's links and hardware
     sustain (Network construction, in Open questions).
   - Rerun the Docker testbed's five-node mesh (`mesh-b`, `mesh-margin`),
     whose seed's push children filled its uplink, on today's seed budget,
     and reproduce what remains in the sim.
   - Pushes queue toward a peer whose connection is backed up past one
     window, to ppspp's cap, and are dropped as stale only when written;
     and a viewer that falls a window behind sends its neighbours messages
     they count as invalid (strims-sim's slow viewer).
2. **A residential seed at a real bitrate.** Behind a cable seed at
   6-8 Mbit/s honest viewers lag several times what they do behind fiber.
   - **Pull from a constrained seed.** Its neighbours pull the newest
     chunks from it several times over. Candidates, none a wire change:
     answer a fresh chunk to a bounded number of neighbours, by the seed's
     own count of what it sent; cancel the requests already queued at a
     peer whose answers lag; gate pull from the seed by deadline.
   - **Joins' backlogs.** A joiner's backlog delays the others' live
     arrivals: find whether the seed's uplink or the joiner's requests take
     the time. Backlog chunks are never announced, so viewers joining the
     same GOP can't serve each other (`collapse_join_backlog_hold`).
   - **Holders' shares under pull.** `SeedQos::ByAllocation` gives a lone
     holder 1000 / (1000 + 50 × the other neighbours) of the uplink,
     which with 32 neighbours is barely what a 6 Mbit/s copy needs. A
     holder's weight should cover its pushes whatever the neighbour
     count.
   - **Protocol overhead.** Score each candidate with `suite.py --rank
     overhead`. Request runs of chunks, not one at a time (no wire
     change). Ping per peer, not per channel: one ping loop per peer in
     the runner, riding the first channel payload written to it, would
     feed every channel's controller to that peer (no wire change); it
     changes what the controllers see, so gate it on the ppspp sim's A/B
     matrix and strims-sim's WAN cases.
3. **Greedy peers.** Liars and colluders, most of their cost on the
   child side.
   - **Pull from liars.** A liar still gets requests when the channels
     expected to answer sooner have no room, and LEDBAT's floor reopens
     its window after each timeout.
   - **HAVEs past the head.** A HAVE far past the stream's head moves a
     non-seed's newest bin time and prunes its bin times, so deadlines
     and HAVE lags lose their start until the stream catches up. It
     needs a bound on how far past the newest chunk a peer may announce.
   - **Liars after a bar.** Each time a bar lapses the liar takes a copy
     again until the reports bar it once more; reports judge only peers
     the seed feeds, so a barred peer can't show it serves first.
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
     grafts onto**: a far holder can relay to nobody, and keeps its copy
     while no reporter votes on it.
4. **Pull on lossy, sparse and mobile paths.** The worst scenarios after
   a constrained seed, where pull does the most repair.
   - **The silence after a timeout.** LEDBAT's minimum window, under a
     chunk, leaves a channel asking for nothing for `FLOOR_LOSS_HOLD` after
     a timeout, which costs pull-heavy cells tail lag and stalls joins
     (`collapse_join_backlog_hold`). Find what it buys there, likely a
     congested peer left to its pushes, and keep the silence for that
     alone.
   - **Deadlines from the player.** Once `UnitsPlayer`'s position reaches
     the scheduler (Video, Players), a chunk's deadline is its publish time
     plus the player's target lag: wait longer for pushes while pull can
     still meet it, and skip chunks no peer can meet.
5. **Push-tree refinements.** Smaller or narrower costs.
   - Slow parents are detected late, so `sim_push_slow_parent`'s p99 can
     still pass 1 s on some seeds. The upload meter's 2 s busy window
     takes about 1.5 s to reach `SHED_UTILIZATION`, a child's deadline
     stretches with its parent's lag up to `DEADLINE_MAX`, and
     `PRUNE_MIN_AGE` and `SWITCH_HOLD` shield a new parent.
   - Push loops fed by pull: a peer that pulled our newest chunk from
     elsewhere passes the graft rule while its parents pass through us,
     and the silent-parent prune breaks the loop late where parents lag.
   - Push bypassing congestion control: judge it on the window model
     (`STRIMS_SIM_TCP`).
   - **Copies by region.** Try a scenario whose nearest neighbours share
     one path or access network.
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
   - it keeps a liar's slot for as long as it asks (the silence rule
     declines free riders); the silent-holder rule is the adaptive seed's.

### strims-ppspp defects
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

- **Network construction.** Which peers a node shares and when, when it
  makes and drops links, and how many it keeps: a budget that is the
  node's equilibrium under its uplink, hardware and network, not a
  constant, with byzantine peers under an honest majority and no gain from
  defecting. strims-sim's organic mode (`STRIMS_SIM_DISCOVERY`, the `net`
  and `net-hostile` tiers) builds topologies from discovery, and its
  peer pool (`pool.rs`, experiment flags) holds the policies under trial.
  - The pool to port to the node's app layer: `pool-role` with eviction
    past the budget, trials (`pool-trial`), the starving rule and sampled
    index answers.
  - Cure the lag ramps that still collapse the `net` tier's runs (the
    verdict keeps counting them).
  - Strong relays belong beside the source from the first wave; neither a
    findable seed nor a seed allocation by reports gets one there in time
    (`TRIED.md`). Find how trees formed around residential holders absorb
    one losing its uplink.
  - Liars still gain: a relay seldom asks its children for anything, so
    the silence rule doesn't reach a liar leaf, and challenging children
    for what they announced failed (`TRIED.md`); what a child serves its
    own neighbours shows only in the reports.
  - Still missing from the sim: referrals on refusal and prune,
    squatters, sybil colluders and poisoners, and NAT, setup-latency and
    CPU models. Make sampled index answers the default, and port the
    winning pool to the node's app layer once it holds.
- **TLS for `wss://` on native nodes.** Native nodes don't dial `wss://`
  yet. tokio-tungstenite can take the rustls `ClientConfig` that
  `NodeFactory` builds once for the node's HTTPS
  (`strims_node::fetch::tls_client_config`).
- **Link and peer housekeeping:**
  - cache host certificates until near expiry;
  - QoS: a per-link gate if per-session blocking shows up in profiles;
    per-link and per-class rate limits; MTU vs exact backlog hints; a
    neutral starting weight for new non-ppspp writers;
  - make vpn `VerifyMode::Enforce` the default; it is `Off`;
  - lossy transports report loss, so direct vpn links can get a backup copy;
  - peer-exchange topology hints that suggest links to shorten long paths;
  - decide whether any app needs to stop a broadcast from being relayed;
  - two nodes whose bootstrap clients dial each other at once need a
    tie-break, as crossed peer exchanges have (the lower host id wins);
  - `connect_peer` policy belongs to the app layer; the node never initiates
    it. Transfer discovery asks to link to every host a search finds, with
    no budget across transfers;
  - lowering the peer budget keeps the peers already connected; decide
    whether it should close the excess, and which peers go;
  - peers that mix invalid data or false HAVEs with valid data are only
    backed off; scoring or disconnecting them is app policy;
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
  - strims-sim's swarm counters and clean-input check read each channel's
    counters while it's open, so they're lower bounds: move them onto the
    runner's totals, which keep closed channels'.
  - Close the window model's gaps against the testbed's bare TCP bench
    (`hack/testbed/benchsuite.py`): the holders' losses beside each
    other's bursts, the holders' probe round-trip tails, a HyStart
    ack-train test, and the uplink cut's twin (`testbed-copy-floor`).
  - Through a halving of the seed's uplink the testbed fell into the
    lock-in deeper than its twin, and its round trips there ran 1.4 times
    the sim's. Recheck with today's frames; if the gap holds, find the
    per-byte overhead on the seed's link.
  - LTE's and Starlink's loss comes in bursts at assumed figures (fades
    of 50 ms every 4 s, the satellites' reassignment every 15 s; the
    profiles' docs), and jitter changes every 20 ms per end. Measure both
    where the app runs on those links.
  - Browser peers collapse more often than TCP peers (the `browsers` tier,
    `STRIMS_SIM_RTC`), most in thin relays and the WAN cells, where SCTP's
    Reno holds a long, lossy path's window low. Mixed audiences (a native
    seed reaching browsers over WebSocket) wait for tuning once swarms stop
    collapsing.
  - A data channel's message holds one frame, and sctp-proto (str0m) ends
    each in a short chunk alone in its packet; ending messages at the
    sender's chunk boundary past their frames would save those packets.
  - str0m over UDP under the WAN window model doesn't replay: an SCTP
    INIT's resend lands a millisecond apart between runs. Find the leak;
    meanwhile WAN cells run native WebRTC on the `str0m` model.
  - Fit the bare bench's WebRTC links (the window model's data channels,
    `wan::sctp`'s `SctpProfile`s) to the testbed: Chromium's first-second
    losses, `bufferedAmount` steps and chunk sizing; Firefox's shortfall
    at a holder's share of the uplink; and both browsers' resends to
    str0m's receiver. The testbed's runs of a case vary: score against
    repeats.
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
    unused.
  - A member whose certificate has expired can't renew: the CA verifies it
    in full. Decide how a lapsed member rejoins.
  - The CA's records live on the device holding the server config and
    don't replicate.
  - A network whose record gains a server config after its first write
    gets neither the owner's CA records nor its directory admin record.
- **Bootstrap.**
  - A published membership has no alias, and the CA keeps no record of
    it (no `NetworkPeers` entry), so its holder can't renew at the CA.
    The publisher could record it.
  - A peer is asked `GetPublishEnabled` once per link, so a change to
    its bootstrap config reaches linked peers when they relink.
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
- Bound trailer length and add a hop-limit field.
- Authenticate unencrypted senders, which prevents reflection.
- Don't trust the publisher's clock in hash-table conflicts.
- Add a signed link-state message with a capability bit, and cheap per-hop
  authentication.
- Use or remove `Rejection`.

### ppspp
- Make HAVE claims verifiable.
- Stamp DATA at send time, not after the sender's queue.
- STREAM_REQUEST: add an expiry or lease plus a rate limit. STREAM_CLOSE:
  add a reason.
- Announce the interior stream.
- Mark pushed DATA and echo send times.
- ANNOUNCE's reach: a report names the reporter's direct parent and
  reaches only the seed's own neighbours, so a holder whose children
  aren't among them goes unjudged (at 6 Mbit/s the seed links to 32 of
  100 viewers). Name the reporter's root holder, carried down the tree,
  or relay reports up the tree, signed by viewer identities the seed can
  check.
- Add an epoch counter: `Writer::reset` within one millisecond keeps the
  epoch, so peers don't reset.
- Bump the protocol version.

### network
- Give runs an id, and answer `Negotiate` with the key count, so a stale
  message can't join a later run.
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
