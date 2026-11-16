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

    cargo build --release -p strims-sim --example testbed_node
    cargo build --release -p strims-testbed --example testbed_browser
    pnpm exec webpack --mode production --env TARGET=wasm-testbed
    VIEWERS=3 SECS=90 hack/testbed/run.sh target/testbed/OUT
    KINDS="native chromium chromium firefox" BROWSER_LINKS=webrtc hack/testbed/run.sh target/testbed/OUT

The first line builds the native node, and the next two the browser
nodes' harness and their page (dist/wasm-testbed).

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
  container's CPU time and memory each second.
- `SEED_LINK`, `VIEWER_LINK` and `LINKS` give nodes access link classes
  (tbf, cable, dsl, fiber, trace; `hack/testbed/links.sh` documents
  each). A trace link replays a capacity trace with tracelink
  (`hack/testbed/tracelink.c`, an EDT classifier over fq that run.sh
  builds with gcc), its files mounted from the host; calibration_star's
  `STRIMS_SIM_CAL_SEED_LINK` and the bench sim take the same specs.
- `CPU_PROFILE` samples the Chromium nodes' V8 profiles.
  `hack/testbed/cpuprofile.py` summarises them (`--crate`, `--match`).
  Wasm frames have names only when the bundle is built with
  `CARGO_PROFILE_RELEASE_WASM_STRIP=false`.
- **The twin.** `twin.py run TAG --bin SCENARIOS` runs strims-sim's
  `calibration_star` the way a testbed matrix ran the testbed, and
  `twin.py lag MATRIX TAG --viewers N` (or `swarm`, `meter`, `peers`,
  `socks`) compares the two. The bare TCP bench is `hack/testbed/bench.sh`
  with `benchsuite.py run` and `benchsuite.py score`, over the cases in
  hack/testbed/bench/.
- **On the farm.** A `kind = "testbed"` matrix runs run.sh as a black box
  on hosts of exactly `host_cores` cores. Its arms are paired on one host
  for each repeat. The sim-farm skill covers running one, and farm.py's
  docstring the matrix keys. The rows land in
  target/sim-farm/MATRIX/rows.jsonl and the out dirs in runs/;
  `farm.py report MATRIX.toml` groups them.

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
