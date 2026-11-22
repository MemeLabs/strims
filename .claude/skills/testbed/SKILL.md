---
name: testbed
description: "The Docker testbed (hack/testbed: real strims nodes, native or headless Firefox and Chromium, in shaped containers on one host), its sim twin and TCP bench, and the device lab (hack/lab: an iPhone and Windows browsers on the LAN). Use for Docker testbed runs, local or as farm testbed matrices; for sizing browser nodes (CPU, memory, how many fit); for real-browser or real-device checks; and for anything in hack/lab."
---

# The testbed and the device lab

The testbed checks two things on a real kernel, TCP and WebRTC stack:
that a mechanism the sims found exists, and that a fix does no harm
there. It is one configuration and a few runs. It never outranks
strims-sim's breadth on effect sizes or rankings, so quote its numbers as
that configuration's. A gap to its twin is a calibration discrepancy, to
be fixed in the sim's model. Keep sessions short: score only the window
that matters, and run few times.

## Running it

The header of `hack/testbed/run.sh` is the reference: every setting,
what lands in the out dir, and the images.

    cargo build --release -p strims-sim --example testbed_node \
      && hack/testbed/stamp.py write target/release/examples/testbed_node
    cargo build --release -p strims-testbed --example testbed_browser \
      && hack/testbed/stamp.py write target/release/examples/testbed_browser
    pnpm exec webpack --mode production --env TARGET=wasm-testbed \
      && hack/testbed/stamp.py write dist/wasm-testbed
    VIEWERS=3 SECS=90 hack/testbed/run.sh target/testbed/OUT
    KINDS="native chromium chromium firefox" BROWSER_LINKS=webrtc hack/testbed/run.sh target/testbed/OUT

The first line builds the native node, and the next two the browser
nodes' harness and their page (dist/wasm-testbed). `stamp.py write`
records beside each artifact the hash of the sources the node, the
harness and the page share (hack/testbed/stamp.py's doc). A run with
browsers refuses a page or a harness whose stamp differs from the
node's, and an artifact that has no stamp or was rebuilt since it was
stamped. A page built from other sources may not speak the node's
protocol (its links come up, but no stream flows), nor read the
harness's commands as the harness sends them. A worktree whose shared
crates differ from those the page was built from builds its own page.

- **One testbed per Docker host at a time.** The container names
  (tb-*), the network and its subnet are fixed. Check
  `docker ps --filter name=tb-` before starting. run.sh needs the docker
  group and the qdisc modules its header names.
- `KINDS` gives each node's kind: native (the default), firefox or
  chromium. A run with a browser also runs a relay. Browsers reach native
  nodes over a WebSocket or over WebRTC (`BROWSER_LINKS`), and each other
  over WebRTC.
- `TOPOLOGY` takes a file of `link a b` lines
  (`hack/sim-farm/matrices/mesh3`, hack/sim-farm/matrices/topologies/).
  The default is a star.
- Each node writes node-<i>.jsonl, a line a second, and cpu-<i>.log, its
  container's CPU time and memory each second. Each line's `rec` holds
  the records strims-sim's tables take, made by the sim's own code
  (strims-testbed's `record`). PCAP=1 gives the tables' wire columns,
  which a comparison wants.
- `FORECAST=1` and `SPEC=1` keep the nodes' scheduler logs, as the sim's
  knobs of those names keep a simulated node's, and the run's tables and
  its twin's fill forecast, arrivals, haves, asks, seed_decisions and
  seed_sends. The lines grow large (a 3-viewer mesh's 40 s: about 5 MB a
  node), and folding a run holds every line in memory (testbed_tables:
  229 MB for that run's 19 MB): measure a run's lines and the fold's RSS
  before a matrix turns the logs on. `PARENT_CHECKS=1` keeps each node's
  parent checks (the tables' parent_checks, as the twin's knob of that
  name); it takes the tap a capture would, so run.sh refuses it with
  `CAPTURE=1`.
- `TIMER_LOG=all` (or node indices) keeps those nodes' timers as they
  fire, as the sim's `STRIMS_SIM_TIMER_LOG` keeps a simulated node's:
  the tables' `timers`, its twin's for the same nodes. A browser's page
  is then served cross-origin isolated, so its worker's clock reads at
  the browser's isolated grain (Chromium's 5 µs, Firefox's 20 µs); the
  profiles' prefs stay at their defaults, as a user's are. Firefox's
  `Date.now()`, the node's own clock, still returns whole ms, but a
  Firefox node runs otherwise with the log on: its line timer and
  writes run less late and its seed's allocation checks slip less often
  than on a plain page, so its timer shares are an isolated page's (the
  farm's testbed-timers-control.toml; a native or Chromium node's runs
  don't move).
- `LINK_LOG=all` (or node indices) keeps those nodes' links' I/O as it
  happens, as the sim's `STRIMS_SIM_LINK_LOG` does: the tables'
  `link_writes`, `link_reads` and `link_samples`, its twin's for the same
  nodes, a browser's page served cross-origin isolated as with the timer
  log. A 20 Mbit/s browser seed's line grows by about 60 kB a second.
- `TRACE_SCTP=all` (or node indices; Chromium nodes, which `LINK_LOG`
  must name too) turns on dcSCTP's packet observer in those browsers:
  every SCTP packet their data channels send and receive, decrypted, in
  sctp-<i>.jsonl (strims-testbed's `sctp` module), the tables'
  `sctp_events`, its twin's for the same nodes (`STRIMS_SIM_TRACE_SCTP`).
  It costs: the observer formats each packet's bytes on the browser's
  network thread, about 0.17 ms a full packet, and a 20 Mbit/s seed's
  container took 0.45 core against 0.18 without it. It slows the stack's
  sending, so it moves the run's timing and its counts alike: the seed's
  unheld writes' buffered amount, its queue's drops, its packets' trains
  on the wire. Take a bench's timing, drops and wire readings from runs
  with it off (`sctp_fit_of`'s timing side), and from runs with it on
  only what the log alone holds (its SACKs, resends per drop, resend
  spacing), knowing the log moved those too.
  Firefox's MOZ_LOG names packets but not their chunks, and neither
  browser's getStats reports SCTP's window or flight.
- `SEED_LINK`, `VIEWER_LINK` and `LINKS` give nodes access link
  classes: tbf, cable, dsl, fiber or trace (`hack/testbed/links.sh`
  documents each).
  - A trace link replays a capacity trace with tracelink
    (`hack/testbed/tracelink.c`, which run.sh builds with gcc): a
    classifier that sets each packet's earliest departure time, over fq.
    The trace's files are mounted from the host, so a trace link runs
    only on a local testbed, and a farm testbed matrix refuses one.
  - run.sh parses every spec with strims-wan's testbed module, through
    its `testbed_link` example, which run.sh builds from its own tree
    unless `LINK_BIN` names one.
  - calibration_star's `STRIMS_SIM_CAL_SEED_LINK` and the bench sim take
    only the specs the WAN model can shape. They refuse cable's default
    DOCSIS-PIE (give `aqm=fifo`), `router=`, `delay=` and `limit=`.
- `CPU_PROFILE` samples the Chromium nodes' V8 profiles.
  `hack/testbed/cpuprofile.py` summarises them (`--crate`, `--match`).
  Wasm frames have names only when the bundle is built with
  `CARGO_PROFILE_RELEASE_WASM_STRIP=false`.
- **Into the lake.** `simdb.py testbed OUT... --name NAME` writes each
  run's tables (strims-sim's `testbed_tables` example: `cargo build
  --release -p strims-sim --example testbed_tables`) in a work dir of its
  own and folds them into the cache as source testbed/NAME, each run a
  job of sim `testbed`. It and `simdb.py replay` only read OUT, which
  stays its maker's (another session's, a farm run's runs/ copy).
  `simdb.py index` then takes them in, naming each run's cell by its
  knobs (`<tier>:<cell>@<qdisc tree>`, else `tb-<hash>`), and the
  tables, the verdicts, the scoring macros and the views read a testbed
  run as they read a sim run. A run with a node that never ran (its
  harness's error, or no lines or records) has no tables, and the index
  takes no testbed run without its own seconds rows; a run whose viewers
  ran and read nothing is a result, kept. A run whose settings don't say
  STAMPS=clock (run.sh's clock stamped its steps and samples) keys apart
  (TESTBED_STAMPS=date: uutils' `date` stamps late), and so do a sampler's
  stamps that go back (back) and a step or competing upload off its
  schedule (TESTBED_SCHEDULE). run.sh writes STAMPS=clock; a run from
  before it did has no twin. Likewise a run whose settings don't say
  JOIN_LINKS=together (its nodes made their links at their joins all at
  once, as a deployed viewer does; before, one at a time, each awaited,
  seed first) keys apart (TESTBED_JOIN_LINKS=serial), and has no twin,
  whose viewers link as the harness now does. So does a run whose
  settings don't say NEIGHBOURS=static (TESTBED_NEIGHBOURS=arp): run.sh
  gives every container a MAC of its address and a permanent neighbour
  entry for every other before its node starts, as a deployed node
  reaches its peers through a router it has long resolved; before, each
  container's first packet to a peer waited for ARP, a round trip through
  both delays and queues ahead of a link's first check or SYN. So does a
  run whose settings don't say SWARMS=transfers (TESTBED_SWARMS=direct):
  its nodes now run their swarm with each peer as the twin's hosts do,
  once strims-transfer's announce and answer bind its channels
  (strims-testbed's peers module), as a deployed node does but without
  the network membership run that precedes them on a deployed node (a
  PLAN item), the relay serving the
  transfers and refusing each announce; before, each node ran it as the
  peer's link came up, where the side whose link comes up first now runs
  it a trip after the other's came up. So does a run whose settings don't
  say LINES=apart (TESTBED_LINES=whole): a node now asks for its line's
  metrics once the second's push period's writes have gone, so the node
  takes them in its next pass (a browser seed's about 7 ms after the
  second at the median, a native node's tens of ms; the meter sample's
  `t_ms` says when), takes the line's records then on its own thread, and
  writes the line on a thread of its own (native) or in the page's lines
  worker (a browser: the records cross as bytes, strims-testbed's
  `record::wire`), as a deployed node, which writes none, loses no time
  to it; before, each took its metrics in that period's own pass (a
  browser seed's 1.6 ms after the second) and wrote its line on its
  thread once they came: a browser seed's 100 kB line (SPEC and its link
  log on) put every whole second's push period's first write 2-5 ms
  late, a native node's took 0.5 ms later in the period. What stays on
  the node's thread is the metrics' collection in its pass (node-core's,
  some 0.4 ms on a browser seed, mostly its runner's) and the take. So
  does a run whose JOINS draw
  the arrivals profile its matrix's knobs name and sit on the runner's
  100 ms tick grid, each a whole number of ticks from the start
  (TESTBED_JOINS=tick), as a draw to the tenth of a second or in whole
  seconds does: each node's tick, set from its first channel, then
  lands at one place in every later joiner's link-up, where the twin's
  draws and a deployed viewer's joins fall anywhere in it
  (`testbed-tier` draws them to the ms). A run folded before the testbed
  module knew a marker keeps its key until it is folded again.
- **The twin.** `simdb.py twin 'testbed/NAME' --name NAME-twin --bin
  SCENARIOS --seeds 8` runs strims-sim's `calibration_star` with each
  cell's knobs (a scenarios test binary from `cargo test --release -p
  strims-sim --test scenarios --no-run`), its runs a side's from the
  catalog (index first; `farm/RUN/%` for a farm run's, `--kind` and
  `--side-arm` as testbed_side's). A twin of one run (a side of one, as
  a joins-paired twin's is) sets its clocks' epoch phase to the run's
  (`STRIMS_SIM_CAL_EPOCH_MS`, the ms of its second the stream started
  at, which the fold records), so its upload meter's 100 ms grid sits
  against the stream's writes as the run's did; a twin of runs that
  started at different ms of their second leaves it out, each of its
  seeds drawing its own. A run's Chromium nodes' cost of a packet on
  their network thread, set by the host's CPU model (EPYC 9R45 5.4 µs,
  9R14 12.3, 7R13 18.1, the dev VM 15.4), is its twin's
  (`STRIMS_SIM_CAL_PACKET_COST_US`, which the fold reads from the long
  trains of a Chromium node's offered capture, else, and always in a
  TRACE_SCTP run, from the CPU model run.sh writes to the out dir's
  `cpu`: strims-sim's `testbed::thread_cost`); the bench sim takes its
  run's too. The fold records that model beside it
  (`STRIMS_SIM_CAL_CPU`, no twin knob). A cell's twin takes its runs'
  median where they all ran on one CPU model, else each twin seed its
  paired run's (`testbed.twin_cost`), so a farm cell whose repeats ran on
  two instance types is twinned on both; a cell with a run of no recorded
  model (its run dir from before `cpu`) pairs too, as one model's runs'
  costs spread wider than two models' lie apart. So are its browser
  pages' offer times, their host's too: a Firefox page's gathering end
  after its node's connect and its offer's post after that, and a
  Chromium page's first setting (`STRIMS_SIM_CAL_FIREFOX_GATHER_MS`,
  `STRIMS_SIM_CAL_FIREFOX_POST_MS`, `STRIMS_SIM_CAL_CHROMIUM_SET_MS`,
  a comma list, a node of its kind each in node order, from
  node-<i>.err's `rtcTiming` and the offered captures, else one value by
  CPU model: strims-sim's `testbed::offer_timing`); each twin page takes
  its node's, by its rank among its kind's, and a cell of one model's
  runs each node's median (`testbed.twin_offer`). A run's seed steps
  where steps.log stamps them, less the seed's egress delay, as its own
  scenario does (`STRIMS_SIM_CAL_SEED_UP_STEPS_MS`, ms of stream: tens of
  ms past their schedule), and its twin with them: a cell's takes their
  median where its runs' times of each step lie within 2 ms
  (`testbed.twin_steps`), else each twin seed its paired run's. A log
  from before run.sh stamped when the tc applied (two columns) gives the
  exec's return, upper bounds (`STRIMS_SIM_CAL_SEED_UP_STAMP=exec`, no
  twin knob). A cell's
  twin logs the seed's packets
  where its runs hold them in the catalog (a PCAP=1 run's every node:
  `STRIMS_SIM_PACKET_LOG`, which `testbed.cells_in` adds), which the wire
  fits read at their `at_node`; `--packet-hosts all` (or host numbers)
  logs more of the hosts the runs hold, a log held to the run's end. A
  fit none of whose model runs holds a row of a table a reading takes (a
  twin without that log) fails the query, naming the table and its knob,
  rather than score the reading a fail. A table no run of the catalog
  holds (a tier's lake, its runs PCAP=1 without TRACE_SCTP or LINK_LOG)
  leaves its readings unscored, the rest scoring: a row each of pass
  NULL whose `caveat` names the table and its settings (`bench_logs`).
  sctp_fit_of's model side is the twin of the testbed's logged cell
  (TRACE_SCTP), whose wire is the log-off cell's twin's (the sim's SCTP
  log moves nothing).
  The macros `twin_runs_of` and
  `twin_fit_of` compare the two over the same tables (`simdb.py describe
  twin_fit_of`): a side is `testbed_side(SRC_LIKE, kind := ..., arm :=
  ...)`, and twin_fit_of gives each cell's medians and the testbed runs'
  place among the twin's seeds (b_quantile). Both score each run's stream
  by default, from 10 s to its own SECS (`twin_window_of`): a twin records
  10 s past its stream and a testbed run about 5, where the meter reads
  nothing and the lag is carried. `effects` scores one against
  the other, a testbed run paired with the twin seed of its number.
  `simdb.py criterion TB_LIKE TW_LIKE --kind K` scores crates/PLAN.md's
  meter-search resume criterion (the macro `twin_criterion_of`): each
  group's and measure's readings and bars, and each cell's runs a side;
  `--paired` takes each testbed run against its own joins-paired twins
  (arm `pair-r<n>`, of its seed n) where its cell has them; otherwise the
  cell's twins. The bare TCP bench is
  `hack/testbed/bench.sh` with `benchsuite.py run` and `benchsuite.py
  score`, over the cases in hack/testbed/bench/. Its sim (`benchsuite.py
  sim`, testbed_bench `--sim`) gives each uplink the run's tbf bucket, as
  the tier's twins' `tbf` class does; `--no-bucket` drains it at its rate
  alone. Its comparison
  (`hack/testbed/bench.py`) reads a run's uplink queue and qdisc tree
  through a release testbed_tables (`--uplink`; `TABLES_BIN`, else `cargo
  build --release -p strims-sim --example testbed_tables`). A case with
  browser KINDS passes run.sh's stamp check only with testbed_bench
  stamped (`hack/testbed/stamp.py write
  target/release/examples/testbed_bench` after the build); bench.sh
  stamps the bench page itself before such a run, its stamp untracked
  beside it (hack/testbed/bench/page.stamp, gitignored).
- **The shaper bench.** A PCAP=1 run's tables hold each node's `packets`:
  what it offered its link (its offered capture, GSO's buffers cut) and
  each one's fate past the queue. `simdb.py replay OUT... --name NAME`
  (the runs in their ingest's order) replays those offers through each
  node's link in the model (strims-sim's `shaper_replay` example, `cargo
  build --release -p strims-sim --example shaper_replay`) twice, every
  offer (arm `replay`, the drops') and what the link carried
  (`replay:carried`, the departures'), each a job of its run's cell with
  env `SOURCE_K` its source's key, into the cache as source testbed/NAME;
  `shaper_fit_of((SELECT list({'cell': stored_cell, 'seed': seed, 'k': k,
  'src': src}) FROM jobs WHERE src = 'testbed/NAME'),
  testbed_side(SRC_LIKE))` joins them packet by packet and applies the
  shaper bench's pass rule per node (`simdb.py describe shaper_fit_of`;
  it has no backlog rule, and a trace link no replay). A source of the
  testbed side that holds packets and has no replay among the first
  argument's jobs (a run left out of the replay, one whose links have
  none) fails the fit, so name the replayed runs alone:
  `(SELECT list(s) FROM (SELECT unnest(testbed_side(SRC_LIKE)) AS s) WHERE
  s.k IN (SELECT env['SOURCE_K'] FROM jobs WHERE src = 'testbed/NAME'))`. `shaper_replay
  --bench OUT TABLES` reads a bench run's captures alone and writes the
  bench's `packets` and both replays'.
- **On the farm.** A `kind = "testbed"` matrix runs run.sh as a black box
  on hosts of exactly `host_cores` cores. Its arms are paired on one host
  for each repeat. The sim-farm skill covers running one, and farm.py's
  docstring the matrix keys. A matrix whose artifacts hold `TABLES_BIN`
  (strims-sim's testbed_tables) has each run's tables folded on its host
  and flushed to S3 as a sim job's are (the farm's S3 bucket and
  permissions, as sim matrices' flushes take), so `simdb.py pull --run
  MATRIX --commit SHA` brings a farm run into the cache (`--commit` takes
  that run's flushes alone, where S3 holds a rerun's under its name too:
  sim-lab), an experiment `farm/RUN`,
  its rows with the host's steal, load and node line counts
  (`_steal_share`, `_load_max`, ...). Its results are the lake's alone:
  target/sim-farm/MATRIX/rows.jsonl is the farm's bookkeeping and runs/
  the out dirs, and `farm.py report MATRIX.toml` names the farm runs
  that hold its runs' tables. `simdb.py testbed-tier DIR` writes the
  testbed tier's matrices (hack/sim-farm/matrices/testbed-tier: the
  tiers' cells the testbed hosts, one a matrix, each repeat its own
  joins, to the ms). Were run.sh to write STAMPS=date, each matrix's
  header would say its runs key apart. Each arm runs its own bundle's
  run.sh and TABLES_BIN, so in an A/B whose base predates a marker
  (LINES=apart, SWARMS=transfers, NEIGHBOURS=static, JOIN_LINKS=together)
  the base arm's runs write their lines on the node's thread, run their
  swarms at each link, resolve their peers by ARP or link one at a time,
  and its TABLES_BIN, which
  doesn't know the marker, keys them in the other arm's cells: such an
  A/B compares harnesses as well as its change.

## Browser node costs

Measured on the dev VM with run.sh. Measure again from cpu-<i>.log when
a sizing decision rests on them.

- A browser node's CPU grows with its peers. It is about 0.1 core for
  Firefox and 0.07 for Chromium with 3 peers, and 0.55 and 0.43 with 15
  (a 16-node full mesh). A native node takes 0.01-0.07.
- In a 16-node Chromium full mesh at a 6 Mbit/s stream (`RATE=750000`), a
  container took 0.35-0.53 core. The node's worker took about 0.28 of
  that (the seed's, 0.12). The rest is Chromium's own SCTP, DTLS and ICE
  threads, which V8 profiles don't show.
- CPU follows bytes moved, at about 0.2 core-seconds per MB in a WebRTC
  mesh, not frames.
- Memory: Firefox about 650-720 MB per container, Chromium about
  280-300 MB (peaks of 295 MB a viewer, 312 MB a seed with SPEC and its
  logs on, the page's lines worker among them).
- About 15 browsers in a full mesh fit the dev VM. They ran without
  disturbing timers, with the host about 55 % idle on average and 10 % at
  peaks. Sparser topologies fit more. Size a farm testbed host's
  `host_cores` from the per-node CPU at the run's peer count, leaving the
  host about half idle.
- PCAP=1's captures come back in each farm run's out dir, at about
  0.16 MB per viewer-second (a 31-viewer mesh6 run of 60 s, 300 MB):
  size a matrix's transfer and target/ disk by its viewers, seconds and
  runs.
- run.sh keeps browser profiles on tmpfs. Profile writes to the host's
  disk stalled other containers by up to 160 ms.
- Judge load from cpu-<i>.log and the host's idle, never from lag. A
  swarm can lag for protocol reasons with CPU to spare.
- run.sh's samplers take their own share, in each container's cgroup (20 s
  alone in the image, an idle container's 0.003 core taken off).
  hack/testbed/tcsample.c samples every node's qdiscs at 10 Hz (one
  process, rtnetlink, `tc -s -j qdisc show`'s JSON lines) for 0.004 core,
  where a shell loop of tc and tr cost 0.08, more than a native node. The
  cpu loop reads with bash's builtins and `read -t`, nothing measurable,
  where date, sed and cat once a second cost 0.012; the netstat loop
  (netstat-<i>.log, each native node's TCP counters every 100 ms) reads
  the same way for 0.0035 core. hack/testbed/sssample.c
  samples the seed's sockets every SS_EVERY for 0.004 at 1 Hz, where ss's
  loop cost 0.028: a dump walks the kernel's whole established hash
  (every network namespace's), which ss and sock_diag pay alike, so it
  dumps only when the namespace's ActiveOpens, PassiveOpens or CurrEstab
  moved (/proc/net/snmp), on the sample after, and after a known socket
  went, and asks for each known socket by address and cookie otherwise; a
  new socket shows in the first sample after it established. run.sh
  builds both with gcc into target/testbed (TCSAMPLE_BIN, SSSAMPLE_BIN
  skip it); the farm ships them as artifacts, a host having no gcc.

## The device lab

hack/lab is two parts. An agent page stays open on real devices on the
LAN: an iPhone, and Chrome and Firefox on a Windows machine. A loopback
controller runs code on those pages. The usage is in
`hack/lab/README.md`: the ports, the controller's /clients, /eval and
/log, and a strims-svc the devices link to.

- **Run it from the main checkout,** with the dev machine's LAN address,
  the one address the lab CA's name constraint permits (no repo file
  names it): `node hack/lab/server.mjs ADDR`. Its CA and
  server keys (hack/lab/tls/, gitignored, made per machine) exist only
  there. A worktree has none, and running `hack/lab/mkcert.sh` there
  would make a new CA that no device trusts.
- **Never delete hack/lab/tls/ or regenerate the CA.** The devices trust
  it, and installing it again is manual work on each device.
  `hack/lab/mkcert.sh` keeps an existing CA and makes only a new server
  certificate.
- **Never reload the agent pages.** A reload loses the unlock tap, and
  only the user can tap again.
- **One job drives the iPhone at a time.** Jobs that share it each run
  under `flock` on one lock file, target/lab/iphone.lock in the main
  checkout.
- **The server is a session's background task.** In a new session, first
  check whether one answers (`curl -s http://127.0.0.1:8090/clients`),
  and start it only if none does.
- A device's links are outside the testbed's shaped network.
