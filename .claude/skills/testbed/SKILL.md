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
    cargo build --release -p strims-testbed --example testbed_browser
    pnpm exec webpack --mode production --env TARGET=wasm-testbed \
      && hack/testbed/stamp.py write dist/wasm-testbed
    VIEWERS=3 SECS=90 hack/testbed/run.sh target/testbed/OUT
    KINDS="native chromium chromium firefox" BROWSER_LINKS=webrtc hack/testbed/run.sh target/testbed/OUT

The first line builds the native node, and the next two the browser
nodes' harness and their page (dist/wasm-testbed). `stamp.py write`
records beside each the hash of the sources the node and the page share
(hack/testbed/stamp.py's doc), and a run with browsers refuses a page
whose stamp isn't the node's, or an unstamped or since rebuilt artifact:
one from other sources may not speak the node's protocol (links come up,
no stream flows). A worktree whose shared crates differ from the page's
builds its own.

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
- `SEED_LINK`, `VIEWER_LINK` and `LINKS` give nodes access link classes
  (tbf, cable, dsl, fiber, trace; `hack/testbed/links.sh` documents
  each). A trace link replays a capacity trace with tracelink
  (`hack/testbed/tracelink.c`, an EDT classifier over fq that run.sh
  builds with gcc), its files mounted from the host, so it runs on a
  local testbed alone: a farm testbed matrix refuses one. run.sh parses
  every spec with strims-wan's testbed module (its `testbed_link`
  example, which run.sh builds from its own tree, or `LINK_BIN`), and
  calibration_star's `STRIMS_SIM_CAL_SEED_LINK` and the bench sim take
  the specs the WAN model can shape, refusing cable's default DOCSIS-PIE
  (give `aqm=fifo`), `router=`, `delay=` and `limit=`.
- `CPU_PROFILE` samples the Chromium nodes' V8 profiles.
  `hack/testbed/cpuprofile.py` summarises them (`--crate`, `--match`).
  Wasm frames have names only when the bundle is built with
  `CARGO_PROFILE_RELEASE_WASM_STRIP=false`.
- **Into the lake.** `simdb.py testbed OUT... --name NAME` writes each
  run's tables (strims-sim's `testbed_tables` example: `cargo build
  --release -p strims-sim --example testbed_tables`) and folds them into
  the cache as source testbed/NAME, each run a job of sim `testbed`.
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
  schedule (TESTBED_SCHEDULE). Today's run.sh writes STAMPS=date, so
  every run it makes keys apart and has no twin, until run.sh stamps from
  its own clock and writes STAMPS=clock (the sampler wiring). Don't spend
  farm money on testbed matrices, the tier's included, before then.
- **The twin.** `simdb.py twin 'testbed/NAME' --name NAME-twin --bin
  SCENARIOS --seeds 8` runs strims-sim's `calibration_star` with each
  cell's knobs (a scenarios test binary from `cargo test --release -p
  strims-sim --test scenarios --no-run`), its runs a side's from the
  catalog (index first; `farm/RUN/%` for a farm run's, `--kind` and
  `--side-arm` as testbed_side's). The macros `twin_runs_of` and
  `twin_fit_of` compare the two over the same tables (`simdb.py describe
  twin_fit_of`): a side is `testbed_side(SRC_LIKE, kind := ..., arm :=
  ...)`, and twin_fit_of gives each cell's medians and the testbed runs'
  place among the twin's seeds (b_quantile). `effects` scores one against
  the other, a testbed run paired with the twin seed of its number. The bare TCP bench is
  `hack/testbed/bench.sh` with `benchsuite.py run` and `benchsuite.py
  score`, over the cases in hack/testbed/bench/.
- **On the farm.** A `kind = "testbed"` matrix runs run.sh as a black box
  on hosts of exactly `host_cores` cores. Its arms are paired on one host
  for each repeat. The sim-farm skill covers running one, and farm.py's
  docstring the matrix keys. The rows land in
  target/sim-farm/MATRIX/rows.jsonl and the out dirs in runs/;
  `farm.py report MATRIX.toml` groups them. A matrix whose artifacts hold
  `TABLES_BIN` (strims-sim's testbed_tables) has each run's tables folded
  on its host and flushed to S3 as a sim job's are (the farm's S3 bucket
  and permissions, as sim matrices' flushes take), so `simdb.py pull
  --run MATRIX` brings a farm run into the cache, an experiment
  `farm/RUN`. `simdb.py testbed-tier DIR` writes the testbed tier's
  matrices (hack/sim-farm/matrices/testbed-tier: the tiers' cells the
  testbed hosts, one a matrix, each repeat its own joins). While run.sh
  writes STAMPS=date, each matrix's header says its runs key apart.

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
  280-290 MB.
- About 15 browsers in a full mesh fit the dev VM. They ran without
  disturbing timers, with the host about 55 % idle on average and 10 % at
  peaks. Sparser topologies fit more. Size a farm testbed host's
  `host_cores` from the per-node CPU at the run's peer count, leaving the
  host about half idle.
- run.sh keeps browser profiles on tmpfs. Profile writes to the host's
  disk stalled other containers by up to 160 ms.
- Judge load from cpu-<i>.log and the host's idle, never from lag. A
  swarm can lag for protocol reasons with CPU to spare.
- run.sh's samplers take their own share, in each container's cgroup (20 s
  alone in the image, an idle container's 0.003 core taken off). The tc
  loop (`tc -s qdisc show` at 10 Hz, a fork of tc and tr a sample) costs
  0.08 core, more than a native node; hack/testbed/tcsample.c (one process,
  rtnetlink, the same lines) 0.001. The cpu loop (date, sed and cat once a
  second) costs 0.012; read with bash's builtins and `read -t` it costs
  nothing measurable. The seed's ss loop costs 0.025 at 1 Hz, most of it
  the kernel walking its whole established hash (every network
  namespace's) for the dump, which ss and a sock_diag dump pay alike:
  hack/testbed/sssample.c dumps only when the namespace has opened a TCP
  connection since the last sample (/proc/net/snmp), and the sample
  after, and asks for each known socket by address otherwise: 0.0016
  with steady sockets, a new one in the first sample after its connect,
  as ss's loop has it. With every node sampled, count the tc loop in a host's
  `host_cores` until tcsample replaces it.

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
