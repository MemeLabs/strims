#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The calibration testbed: real strims nodes in Docker containers on one
# bridge, each shaping its own uplink, so the upload meter can be measured
# on a real shaped link and compared with strims-sim's model of one (its
# calibration scenarios). A node is native (strims-sim's testbed_node
# example) or a headless browser running strims-wasm's node
# (strims-testbed's testbed_browser example, in the node's container).
#
#   hack/testbed/run.sh <out dir>
#
# Every container is its own network namespace with a veth on the bridge.
# Each shapes its egress as a host behind an access modem. eth0's clsact
# hands every packet the node sends to an ifb (ifbd), whose netem holds it
# for the one-way delay. The ifb hands it back to eth0, whose root qdisc is
# the modem: the node's access link class (hack/testbed/links.sh's
# node_tree). The default class, tbf, is a tbf of the link's rate, bucket
# and queue:
#
#   tc qdisc replace dev eth0 root handle 10: tbf rate <up>mbit \
#       burst <bucket> limit <up x queue x 125 bytes> \
#       [peakrate <p>mbit mtu <two of the class's largest frames>]
#   tc qdisc add dev ifbd root netem delay <d>ms limit 100000
#   tc filter add dev eth0 egress matchall action mirred egress redirect dev ifbd
#
# The other classes change that root:
#
#   - cable's, dsl's and fiber's framing puts a size table on it (cake
#     takes the framing as its own overhead instead);
#   - cable's DOCSIS-PIE (aqm=pie) is a pie under it; a FIFO, such as a DSL
#     modem's, is the tbf's own byte limit;
#   - a home router (router=) takes its place: cake at the router's rate,
#     or a tbf at that rate over fq_codel;
#   - a trace's root is a prio whose band 1 is an fq, which holds each
#     packet to the departure tracelink's classifier sets.
#
# A class with slots (cable's grants, slot=) moves the modem, handle 10:,
# to a second ifb (ifbm). eth0's root then hands ifbm what the delay
# returns, and holds what comes back from ifbm in a netem of slots alone.
# The order is delay, shaper, slots.
#
# The bucket is what leaves at once (or at the peak rate) after the link
# has been idle. It is 3028 bytes (two full frames) unless SEED_BURST (the
# seed's) or VIEWER_BURST (the viewers') says otherwise.
#
# This order, the delay ahead of the shaper, acts as a modem's:
#
#   - The socket gets its memory back at the redirect, as a host's socket
#     does at its NIC once it has handed a packet to a modem. The egress
#     filter's redirect hands the ifb a clone of each packet (act_mirred
#     clones at egress), and eth0's clsact hook frees the socket's own
#     (sch_handle_egress consumes the stolen skb). So TCP Small Queues
#     counts nothing the delay or the modem holds, and doesn't keep the
#     modem's queue empty, at any delay (at `netem delay 0ms` a CUBIC
#     upload still filled a 2 Mbit/s root tbf to 120 KB).
#   - A fixed delay ahead of a FIFO shaper delays its departures as one
#     after it would, so a rate step applies the delay after its time.
#
# tbf, the root, is offered each packet as netem releases it, and dequeues
# it before the next, so a burst its tokens cover leaves as it comes, and
# its byte limit is met as the packets arrive. With netem at the root and
# the tbf its child, netem_dequeue would move every packet due into the
# tbf before the tbf sent any, so a burst the tokens covered would still
# meet the limit at once and lose its tail.
#
# Offloads are off on eth0 and on the ifb, so the ifb cuts TCP's GSO
# buffers (TCP always builds them) into the frames a modem would take, and
# tbf shapes real packets. A peak's bucket (tbf's mtu) holds two of the
# class's largest frames: 3028 bytes under tbf, 3036 under cable's and
# fiber's 18-byte framing. On this host tbf wakes some 25 us late for each
# frame its peak paces, which a one-frame bucket would lose (100 Mbit/s
# read 86). Two frames back to back, the most the bucket lets out at once,
# is what a line at the peak sends in 0.24 ms at 100 Mbit/s.
#
# One viewer can get a slow downlink (an ifb device with a tbf, on eth0's
# ingress).
#
# Browser nodes (KINDS) are shaped the same way. The harness in a browser
# container serves the page and drives it on the container's loopback, so
# the shaped eth0 is the browser's only other interface and carries every
# link. Browsers can't listen, so a run with a browser node also runs a
# relay (tb-relay, 10.77.0.5, unshaped; RELAY=0 leaves it out): a native
# node in the testbed network that runs no swarm. Every node dials it and
# joins the network, and browsers make their WebRTC links by peer exchange
# through it. With a log that reads a browser's fine clock on (TIMER_LOG,
# LINK_LOG),
# the harness serves the page cross-origin isolated (strims-testbed's
# Site::serve_isolated), so the worker's performance clock reads at the
# browser's isolated grain; otherwise as a plain page. Either way the
# browsers' prefs stay at their defaults. A node's deadlines go by
# Date.now(), whole ms in either browser either way. A Firefox node runs
# otherwise on the isolated page: its line timer and writes run less late
# and its seed's allocation checks slip less often than on a plain page,
# where a native or Chromium node's don't move
# (hack/sim-farm/matrices/testbed-timers-control.toml).
# Natives link to natives over TCP; a browser links to a native over a
# WebSocket (BROWSER_LINKS=ws) or WebRTC (webrtc), and to a browser over
# WebRTC. The topology file's header (strims-testbed's topology module)
# says who makes which link.
#
# Every container has a MAC of its address (02:77:00:00:00 and the
# address's last octet, in hex), and a permanent neighbour entry for every
# other container of the run, set before its node starts: a deployed node
# reaches its peers through a router whose neighbour it has long resolved,
# where a container's first packet to a peer would wait for ARP, a round
# trip through both containers' delays and queues. A container whose eth0
# lacks its MAC fails before its node starts. The relay, which needs
# NET_ADMIN for its entries alone, drops it before its node starts.
#
# Every node runs the swarm with its peers through strims-transfer's
# announce and answer as a deployed node does, but without the network
# membership run that precedes them on a deployed node (strims-testbed's
# peers module): each peer is on the swarm's network as it links, and
# each side announces it as its link to a peer comes up, and runs it on the
# peer's announce or on the answer to its own, whichever comes first, so
# the side whose link came up first runs it a trip after the other's came
# up. The relay serves the transfers and refuses each announce.
#
# Settings (environment):
#   VIEWERS=<n>            viewers (default 3)
#   SEED_UP_MBIT=<m>       the seed's uplink (default 10)
#   VIEWER_UP_MBIT=<m>     each viewer's uplink (default 20)
#   QUEUE_MS=<ms>          every uplink's queue (default 100)
#   SEED_BURST=<bytes>     the seed's tbf bucket (default 3028; at least its
#                          link's largest frame, what a shaper charges a
#                          full packet: 1514 under tbf, 1518 under fiber's
#                          framing)
#   VIEWER_BURST=<bytes>   each viewer's tbf bucket (default 3028; at least
#                          its link's largest frame)
#   SEED_PEAK_MBIT=<m>     the rate the seed's bucket sends at, its line's
#                          own (tbf's peakrate). It must be over the
#                          uplink's rate and every step's: run.sh refuses
#                          one that isn't. Unset, the bucket sends at once
#   VIEWER_PEAK_MBIT=<m>   each viewer's
#   SEED_LINK=<spec>       the seed's access link class,
#                          `<class>[,<key>=<value>...]`: tbf (the default,
#                          the tbf above), cable, dsl, fiber or trace.
#                          hack/testbed/links.sh documents each class, its
#                          keys and defaults. A class's rate is
#                          SEED_UP_MBIT and its queue QUEUE_MS (a FIFO's
#                          bytes, or cable's DOCSIS-PIE's full frames; a
#                          router's fq_codel and cake keep their own
#                          limits). A tbf's or fiber's bucket is SEED_BURST
#                          and its peak SEED_PEAK_MBIT. A trace's time 0 is
#                          the stream's start (its packets reach the
#                          shaper SEED_DELAY_MS later). Its files are
#                          mounted from the host, so a trace link runs only
#                          on a local testbed (a farm testbed matrix
#                          refuses one)
#   VIEWER_LINK=<spec>     each viewer's (default tbf)
#   LINKS="<i>:<spec> ..." node i's link, in place of SEED_LINK or
#                          VIEWER_LINK
#   SEED_DELAY_MS=<ms>     the seed's one-way delay (default 20)
#   VIEWER_DELAY_MS=<ms>   each viewer's (default 10)
#   SECS=<s>               the stream's length (default 180)
#   JOIN_AT=<s>            when viewers dial in, after the start (default 0).
#                          A viewer that dials in after the start arrives
#                          into the running stream: it reads from the live
#                          edge, as the app's player joins (strims-testbed's
#                          node module)
#   JOINS="<i>:<s> ..."    node i dials in <s> seconds after the start, in
#                          place of JOIN_AT. A viewer of JOINS arrives, and
#                          reads from the live edge, whatever its time (0
#                          too), as its twin's viewers do. Give <s> to the
#                          ms: joins a whole number of 100 ms apart (the
#                          runner's tick, which each node sets from its
#                          first channel) lock every node's tick phase to
#                          every later join, where a deployed viewer's
#                          join falls anywhere in it
#   SLOW_DOWN_KBIT=<k>     viewer 1's downlink, if set
#   SEED_UP_STEPS="<s>:<m> ..."  the seed's uplink becomes <m> Mbit/s <s>
#                          seconds after the stream starts, by a tc qdisc
#                          change SEED_DELAY_MS later; the queue keeps
#                          QUEUE_MS at the new rate. A step hands the seed a
#                          full bucket, SEED_BURST bytes at once: tbf's
#                          change fills its bucket (tbf_change sets its
#                          tokens to the buffer), which a link whose rate
#                          changes doesn't do, and no qdisc here steps a
#                          byte bucket's rate keeping its tokens (HTB keeps
#                          them as time at the rate, and passes a packet on
#                          any)
#   COMPETE="<s>:<d> ..."  a bulk TCP upload (iperf3, CUBIC) from the seed's
#                          namespace, through its shaped uplink, to a sink
#                          container (10 ms egress delay), from <s> seconds
#                          after the stream starts for <d> seconds
#   SEED_LOSS_PCT=<p>      the seed's site loses each packet it sends or
#                          receives with this probability, in percent, as
#                          the sim's site loss does: one in round(100/<p>)
#                          (tc's gact) as each packet reaches its receiver,
#                          the node's and COMPETE's upload's alike: a filter
#                          on the seed's ingress, and on each viewer's and
#                          the sink's for what the seed sends them. netem's
#                          loss on the seed's egress would drop the whole
#                          GSO buffer a packet leaves in
#   SS_EVERY=<s>           how often the seed's sockets are sampled into
#                          ss-0.log (default 1)
#   TOPOLOGY=<file>        links (`link a b` lines); default a star
#   NODE_BIN=<path>        the testbed_node binary (default
#                          target/release/examples/testbed_node), built by
#                          `cargo build --release -p strims-sim --example
#                          testbed_node && hack/testbed/stamp.py write
#                          target/release/examples/testbed_node`, which
#                          writes its stamp beside it
#   TCSAMPLE_BIN=<path>, SSSAMPLE_BIN=<path>
#                          the containers' samplers (hack/testbed/tcsample.c
#                          and sssample.c, static); unset, run.sh builds
#                          them with gcc into target/testbed, as it does
#                          tracelink (a farm host has no gcc: the farm
#                          ships them as artifacts)
#   LINK_BIN=<path>        the link specs' parser (strims-wan's testbed_link
#                          example). Unset, run.sh builds it from its own
#                          tree with cargo, into CARGO_TARGET_DIR (default
#                          target/), so NODE_BIN may be another commit's.
#                          That build waits for any other cargo build on
#                          the same dir (cargo says so); LINK_BIN avoids
#                          the wait
#   RATE=<bytes/s>         the stream's rate, at every node (default
#                          strims-testbed's RATE, 256 KiB/s)
#   MEDIA=<flv file>       the seed streams the file's H.264 and AAC in the
#                          unit format (strims-testbed's media module), in
#                          place of the self-checking frames at RATE; it
#                          must be native. Browser viewers play it through
#                          the baseline player, fed from their node as an
#                          app page's player is; a player that restarts
#                          takes the stream from the next whole record
#   UNITS_CPS=<n>          with MEDIA, the swarm's chunks per signature,
#                          the file's signing as ingest chooses it
#                          (default 8)
#   RECORD="<i> ..."       with MEDIA, these native viewers record the Data
#                          messages they read in record-<i>.bin
#                          (testbed_node's --record)
#   NODE_ARGS=<args>       more flags for every native node
#   FORECAST=1             every node keeps its scheduler's forecast log
#                          (testbed_node's and testbed_browser's
#                          --forecast), as strims-sim's STRIMS_SIM_FORECAST=1
#                          keeps a simulated node's, and its lines' records
#                          carry it: the run's tables' forecast and arrivals,
#                          and its twin's (its knob)
#   SPEC=1                 every node keeps its request and seed logs
#                          (--spec), as STRIMS_SIM_SPEC=1 does: the tables'
#                          arrivals, haves, asks, seed_decisions,
#                          seed_sends and path_probes. Both off (0) by
#                          default; a line's records hold no logs then
#   PARENT_CHECKS=1        every node keeps its parent checks
#                          (--parent-checks: its decision log's), as
#                          STRIMS_SIM_PARENT_CHECKS=1 does: the tables'
#                          parent_checks. Off (0) by default. It takes the
#                          tap a capture would, so run.sh refuses it with
#                          CAPTURE=1
#   TIMER_LOG=<nodes>      these nodes keep their timers as they fire
#                          (testbed_node's and testbed_browser's
#                          --timer-log): `all`, or node indices of the
#                          run's (`0 2`; 0 the seed), as strims-sim's
#                          STRIMS_SIM_TIMER_LOG keeps a simulated node's:
#                          the tables' timers. Unset, none
#   TIMER_PROBE=1          TIMER_LOG's nodes run the timer probe through
#                          the stream (--timer-probe: strims-testbed's
#                          node::probe, timers of fixed delays), as
#                          STRIMS_SIM_TIMER_PROBE=1 has the twin's run it.
#                          Off (0) by default
#   LINK_LOG=<nodes>       these nodes keep their links' I/O as it happens
#                          (testbed_node's and testbed_browser's
#                          --link-log): `all` (nodes 0 to VIEWERS, never
#                          the relay), or node indices of the run's (`0
#                          2`, written plainly), as strims-sim's
#                          STRIMS_SIM_LINK_LOG keeps a simulated node's:
#                          the tables' link_writes, link_reads and
#                          link_samples. A browser's page is then served
#                          cross-origin isolated, as with TIMER_LOG.
#                          Unset, none
#   TRACE_SCTP=<nodes>     these Chromium nodes' dcSCTP logs every packet
#                          its data channels send and receive, decrypted
#                          (testbed_browser's --trace-sctp), into
#                          sctp-<i>.jsonl: `all` (every Chromium node of
#                          0 to VIEWERS), or node indices of the run's, as
#                          strims-sim's STRIMS_SIM_TRACE_SCTP keeps a
#                          simulated node's associations: the tables'
#                          sctp_events. Its associations are named by the
#                          node's writes on each link, so LINK_LOG must
#                          name each of them; a bench node's are named by
#                          the trace's own lines, each by the link the
#                          bench harness made it for. The log costs the browser's
#                          network thread about 0.17 ms a full packet
#                          (strims-testbed's sctp module), which slows
#                          its sending: it moves the run's timing and its
#                          counts (the seed's queue drops among them), so
#                          take a bench's timing, drops and wire readings
#                          from runs with it off. Unset, none
#   CAPTURE=1              ppspp captures in <out dir>/capture, one dump
#                          per node (native and browser), named by host id
#   PCAP=1                 each node's packets on its eth0, both ways, in
#                          <out dir>/pcap-<i>.pcap (tcpdump, after the
#                          egress queue: what it drops isn't there), and
#                          what it offers its egress queue in
#                          pcap-<i>-offered.pcap (a clsact mirror to an
#                          ifb, ahead of the delay and the queue): the first
#                          PCAP_SNAPLEN bytes of each (default 128: the
#                          headers; a data channel's payload past its DTLS
#                          record header is encrypted)
#   KINDS="<k0> <k1> ..."  node i's kind: native (the default for nodes not
#                          listed), firefox or chromium
#   BROWSER_LINKS=ws|webrtc  how browsers link to native nodes (default ws)
#   BROWSER_BIN=<path>     the testbed_browser binary (default
#                          target/release/examples/testbed_browser), built by
#                          `cargo build --release -p strims-testbed --example
#                          testbed_browser && hack/testbed/stamp.py write
#                          target/release/examples/testbed_browser`, stamped
#                          from NODE_BIN's sources as WEB_BUNDLE is
#   BROWSER_ARGS=<args>    more flags for every browser node's harness
#   CPU_PROFILE=<at>:<s>   each Chromium node's page and workers sampled by
#                          V8's profiler from <at> seconds after the start
#                          for <s> (testbed_browser's --cpu-profile), into
#                          <out dir>/profile/cpu-<i>-<target>.cpuprofile;
#                          wasm frames have names with a bundle built with
#                          CARGO_PROFILE_RELEASE_WASM_STRIP=false
#   RELAY=0                no relay, for a harness that makes its nodes'
#                          links itself (the bare bench's, bench.sh)
#   WEB_BUNDLE=<dir>       the browsers' page (default dist/wasm-testbed),
#                          built by `pnpm exec webpack --mode production
#                          --env TARGET=wasm-testbed &&
#                          hack/testbed/stamp.py write dist/wasm-testbed`
#                          (which needs node_modules, the pinned
#                          wasm-bindgen-cli and the wasm32 target); the
#                          harness checks it holds its page. A run with
#                          browsers needs it, NODE_BIN and BROWSER_BIN
#                          stamped from one checkout's sources (stamp.py).
#                          A page from other sources may not speak the
#                          node's protocol (its links come up, but no stream
#                          flows), nor read the harness's commands as it
#                          sends them, so run.sh refuses it, and an artifact
#                          without a stamp or rebuilt since, saying how to
#                          rebuild
#
# Writes to <out dir>:
#   topology
#   cpu                    the host's CPU model (/proc/cpuinfo's model name):
#                          its Chromium nodes' cost of a packet on their
#                          network thread, where the run has no capture
#                          (strims-sim's testbed::thread_cost)
#   node-<i>.jsonl         a JSON line per second from each node
#                          (strims-testbed's node module); a browser's adds
#                          post_ms and host_t_ms (testbed_browser's)
#   node-<i>.err           the node's errors; a browser node's harness logs
#                          there each of its page's events with its clock
#                          (testbed_browser's), its signalling timings
#                          (`rtcTiming`) among them, and each peer
#                          exchange it asks for
#   cpu-<i>.log            the container's CPU time and memory once a
#                          second: ms since the epoch, the cgroup's
#                          usage_usec, and its memory.current in bytes,
#                          read with bash's builtins
#   netstat-<i>.log        each native node's TCP counters, its namespace's
#                          (/proc/net/netstat's TcpExt line), every 100 ms,
#                          read with bash's builtins: `# TcpExt: <names>`
#                          once, then `<ms since the epoch> TcpExt: <values>`.
#                          strims-sim's testbed module takes the network
#                          columns rtos and window_cuts from it
#   tc-<i>.log             each node's qdiscs on every device at 10 Hz
#                          (tcsample's): ms since the epoch, then `tc -s -j
#                          qdisc show`'s JSON, each qdisc with its dev.
#                          strims-sim's testbed module reads it (its
#                          uplink; testbed_tables --uplink prints it),
#                          finding the modem's 10: on eth0 or, with slots,
#                          on ifbm
#   ss-0.log               the seed's sockets every SS_EVERY (sssample's:
#                          `ss -tin state established`'s text)
#   tcsample-<i>.err, sssample-0.err
#                          the samplers' errors, each costing a sample
#   tcsample-<i>.end, sssample-0.end
#                          when a sampler exited while its node ran, and
#                          its exit status, which fails the run
#   pcap-<i>.end           with PCAP, when its tcpdump exited while its node
#                          ran, and its exit status: none when it ran to the
#                          end
#   qdisc-<i>.txt          each node's qdiscs on every device as it shaped
#                          them, before the stream (`tc -s -d qdisc show`):
#                          its modem with its size table, the delay's ifbd,
#                          the slots' ifbm, the slow downlink's ifbs and
#                          PCAP's ifbo
#   steps.log              each of SEED_UP_STEPS: when it applied (its tc's
#                          return, stamped in the seed's container), its
#                          rate, and when its docker exec began and
#                          returned here, ms since the epoch: `<applied>
#                          <mbit> <began> <returned>` (a run from before
#                          the stamp has `<returned> <mbit>`)
#   compete-<s>.json       each upload's iperf3 report
#   compete.log            when each upload started
#   links                  each node's link spec, rate, delay and queue, and
#                          the device whose root is its modem, as the
#                          parser gives it: `node <i> <spec> <up_mbit>
#                          <delay_ms> <queue_ms> <modem_dev>`
#   sctp-<i>.jsonl         with TRACE_SCTP, each packet node i's dcSCTP sent
#                          or received, a JSON line each (strims-testbed's
#                          sctp::Packet)
#   link-<i>.log           a trace link's tracelink log: its queue,
#                          packets, bytes and drops every 100 ms
#   link-<i>.err           a trace link's tracelink stderr
#   settings               the settings given, QDISC, the shaping's qdisc
#                          tree (SHAPER below), so runs of one tree pool
#                          apart from another's, and STAMPS, the clock its
#                          own logs' stamps and its waits are read from
#                          (`clock`: bash's $EPOCHREALTIME, as now_ms and
#                          every sampler read it; a run before run.sh named
#                          it, stamped by uutils' `date +%s%3N`, which reads
#                          up to 900 ms late in a second's first 100 ms,
#                          names none, and strims-sim's testbed module keys
#                          it apart), and JOIN_LINKS, how a node made its
#                          links at its join (`together`: all at once,
#                          strims-testbed's topology module; a run before
#                          run.sh named it made them one at a time, each
#                          awaited, seed first, names none, and the testbed
#                          module keys it apart), and NEIGHBOURS, how a
#                          container finds its peers' MACs (`static`: from
#                          its neighbour table, set before its node starts;
#                          a run before run.sh named it resolved each by
#                          ARP at its first packet, names none, and the
#                          testbed module keys it apart), and SWARMS, how
#                          a node ran its swarm with a peer (`transfers`:
#                          once their announce and answer bound its
#                          channels; a run before run.sh named it ran it as
#                          each peer's link came up, names none, and the
#                          testbed module keys it apart), and LINES, how a
#                          node wrote its line of a second (`apart`: its
#                          metrics asked for once the second's push
#                          period's writes had gone, its records taken on
#                          the node's thread and the line written on a
#                          thread or, a browser's, in a worker of its own,
#                          strims-testbed's node module; a run before
#                          run.sh named it took them in that period's own
#                          pass and wrote each line on the node's thread as
#                          they came, which held it, a browser's ahead of
#                          the period's first write, names none, and the
#                          testbed module keys it apart)
#   .ctl                   while the run lasts, its control files, which go
#                          with it
# A node's lines carry the records strims-sim's tables take (`rec`), which
# strims-sim's testbed_tables example writes as a run's tables. With a
# browser node also node-relay.err, cpu-relay.log and hosts/ (each node's
# host id). With MEDIA, each browser viewer's player-<i>.jsonl (its
# player's line a second, testbed_browser's --player), and RECORD's
# record-<i>.bin.
#
# The stream starts once every node's link is up: each container marks its
# link up when it has shaped it, a trace's link once it has loaded its
# trace. The run fails, removing its containers, if a node's link fails to
# build (a tc error, a missing module, a tracelink that fails), a node
# exits with an error, or a step fails to apply. However the run ends, it
# stops its steps and uploads, and each of those checks that the run is
# its own before it acts on a container.
#
# Images: strims-testbed (Dockerfile) for native nodes, the relay and the
# sink, and strims-testbed-browser (browser.Dockerfile) for browser nodes,
# built here. Container names (tb-<i>, tb-relay, tb-sink), the network
# (strims-testbed-sealed, internal: no route out, removed when the run
# ends) and its subnet (10.77.0.0/24) are fixed, so one testbed runs per
# Docker host at a time: run.sh stops if a tb-* container, or a testbed
# network with containers on it, is there.
#
# The containers send nothing off the testbed, but the host still talks
# to each run's new bridge: its avahi's mDNS and its IPv6 multicast (mDNS
# and listener reports from its link-local address). That is 16-19 mDNS
# and about 42 IPv6 packets into each container per run, most before the
# stream. They only come in, so no node's uplink carries them, and the
# metrics count node-to-node traffic alone.
#
# Needs Docker (the user's docker group) and the kernel modules the links
# are shaped with, which links.sh's link_modules lists: every run's netem,
# tbf, ifb, clsact and redirect, SEED_LOSS_PCT's filter, and the link
# classes' qdiscs, classifiers and marks. If tc can't load them, run this
# once (a farm host's boot loads them, and builds each class's tree as
# linkcheck.sh renders it):
#
#   sudo modprobe -a $(. hack/testbed/links.sh; link_modules)
#
# A trace link's container also gets CAP_BPF and CAP_PERFMON, and run.sh
# builds tracelink (hack/testbed/tracelink.c, into target/testbed) with
# gcc.

set -euo pipefail

out=${1:?usage: run.sh <out dir>}
root=$(cd "$(dirname "$0")/../.." && pwd)
viewers=${VIEWERS:-3}
seed_up=${SEED_UP_MBIT:-10}
viewer_up=${VIEWER_UP_MBIT:-20}
queue=${QUEUE_MS:-100}
seed_burst=${SEED_BURST:-3028}
viewer_burst=${VIEWER_BURST:-3028}
seed_peak=${SEED_PEAK_MBIT:-}
seed_link=${SEED_LINK:-tbf}
viewer_link=${VIEWER_LINK:-tbf}
read -r -a link_specs <<< "${LINKS:-}"
viewer_peak=${VIEWER_PEAK_MBIT:-}
seed_delay=${SEED_DELAY_MS:-20}
viewer_delay=${VIEWER_DELAY_MS:-10}
secs=${SECS:-180}
join_at=${JOIN_AT:-0}
slow=${SLOW_DOWN_KBIT:-}
steps=${SEED_UP_STEPS:-}
compete=${COMPETE:-}
ss_every=${SS_EVERY:-1}
ss_ms=$(awk -v s="$ss_every" 'BEGIN { if (s <= 0) exit 1; printf "%d", s * 1000 + 0.5 }') \
  || { echo "SS_EVERY=$ss_every: seconds, more than 0" >&2; exit 1; }
seed_loss=${SEED_LOSS_PCT:-}
loss_every=""
if [ -n "$seed_loss" ]; then
  loss_every=$(awk -v p="$seed_loss" 'BEGIN { v = 100 / p; if (v < 1 || v > 10000) exit 1; printf "%.0f", v }') \
    || { echo "SEED_LOSS_PCT=$seed_loss: out of range" >&2; exit 1; }
fi
node_args=${NODE_ARGS:-}
browser_args=${BROWSER_ARGS:-}
rate_arg=${RATE:+--rate $RATE}
# The scheduler's logs every node keeps (FORECAST, SPEC, PARENT_CHECKS),
# as the sim's knobs of those names keep a simulated node's.
log_args=""
for k in FORECAST SPEC PARENT_CHECKS; do
  case "${!k:-0}" in
    0) ;;
    1) log_args="$log_args --$(echo "$k" | tr '[:upper:]_' '[:lower:]-')" ;;
    *) echo "$k=${!k}: 1 or 0" >&2; exit 1 ;;
  esac
done
if [ "${PARENT_CHECKS:-0}" = 1 ] && [ "${CAPTURE:-0}" = 1 ]; then
  echo "PARENT_CHECKS=1 takes the tap CAPTURE=1 would: not both" >&2; exit 1
fi
# The nodes that keep their timers (TIMER_LOG), as the sim's knob of that
# name keeps a simulated node's.
read -r -a timer_nodes <<< "${TIMER_LOG:-}"
for t in "${timer_nodes[@]}"; do
  if [ "$t" != all ] && ! { [[ $t =~ ^(0|[1-9][0-9]*)$ ]] && [ "$t" -le "$viewers" ]; }; then
    echo "TIMER_LOG=$TIMER_LOG: all, or node indices of the run's 0 to $viewers" >&2; exit 1
  fi
done
case "${TIMER_PROBE:-0}" in
  0) probe_arg="" ;;
  1) probe_arg=" --timer-probe"
     [ "${#timer_nodes[@]}" -gt 0 ] || { echo "TIMER_PROBE=1 needs TIMER_LOG" >&2; exit 1; } ;;
  *) echo "TIMER_PROBE=$TIMER_PROBE: 1 or 0" >&2; exit 1 ;;
esac
timer_args_of() {
  local t
  for t in "${timer_nodes[@]}"; do
    if [ "$t" = all ] || [ "$t" = "$1" ]; then echo "--timer-log$probe_arg"; return; fi
  done
}
# The nodes that keep their links' I/O (LINK_LOG), as the sim's knob of
# that name keeps a simulated node's.
read -r -a link_nodes <<< "${LINK_LOG:-}"
for t in "${link_nodes[@]}"; do
  if ! [[ $t =~ ^(all|0|[1-9][0-9]*)$ ]] || { [ "$t" != all ] && [ "$t" -gt "$viewers" ]; }; then
    echo "LINK_LOG=$LINK_LOG: all, or node indices of the run's 0 to $viewers" >&2; exit 1
  fi
done
link_args_of() {
  local t
  for t in "${link_nodes[@]}"; do
    if [ "$t" = all ] || [ "$t" = "$1" ]; then echo "--link-log"; return; fi
  done
}
# The Chromium nodes whose dcSCTP logs its packets (TRACE_SCTP), as the
# sim's knob of that name keeps a simulated node's.
read -r -a sctp_nodes <<< "${TRACE_SCTP:-}"
for t in "${sctp_nodes[@]}"; do
  if ! [[ $t =~ ^(all|0|[1-9][0-9]*)$ ]] || { [ "$t" != all ] && [ "$t" -gt "$viewers" ]; }; then
    echo "TRACE_SCTP=$TRACE_SCTP: all, or node indices of the run's 0 to $viewers" >&2; exit 1
  fi
done
sctp_args_of() {
  local t
  for t in "${sctp_nodes[@]}"; do
    if [ "$t" = "$1" ] || { [ "$t" = all ] && [ "$2" = chromium ]; }; then
      echo "--trace-sctp /out/sctp-$1.jsonl"; return
    fi
  done
}
bin=${NODE_BIN:-$root/target/release/examples/testbed_node}
browser_bin=${BROWSER_BIN:-$root/target/release/examples/testbed_browser}
web=${WEB_BUNDLE:-$root/dist/wasm-testbed}
read -r -a kinds <<< "${KINDS:-}"
read -r -a joins <<< "${JOINS:-}"
read -r -a records <<< "${RECORD:-}"
media=${MEDIA:-}
units_arg=""
if [ -n "$media" ]; then
  [ -f "$media" ] || { echo "no media file at $media" >&2; exit 1; }
  media=$(cd "$(dirname "$media")" && pwd)/$(basename "$media")
  units_arg="--units ${UNITS_CPS:-8}"
fi
browser_links=${BROWSER_LINKS:-ws}
relay_on=${RELAY:-1}
image=strims-testbed
browser_image=strims-testbed-browser
net=strims-testbed-sealed
old_net=strims-testbed
subnet=10.77.0
relay=$subnet.5
# An address on the subnet no container or host holds (the nodes are
# .10 up, the relay .5, the sink .250, the bridge .1).
unused=$subnet.254
# The sink's address (COMPETE's).
sink=$subnet.250
# A container's MAC: its address's last octet, locally administered.
mac_of() { printf '02:77:00:00:00:%02x' "${1##*.}"; }
# The packets the seed sends, which each viewer and the sink lose
# SEED_LOSS_PCT of (a tc match).
from_seed="protocol ip u32 match ip src $subnet.10/32"

n=$((viewers + 1))
kind_of() { echo "${kinds[$1]:-native}"; }
# shellcheck source=hack/testbed/links.sh
source "$root/hack/testbed/links.sh"
link_of() {
  local l
  for l in "${link_specs[@]}"; do
    if [ "${l%%:*}" = "$1" ]; then echo "${l#*:}"; return; fi
  done
  if [ "$1" -eq 0 ]; then echo "$seed_link"; else echo "$viewer_link"; fi
}
join_of() {
  local j
  for j in "${joins[@]}"; do
    if [ "${j%%:*}" = "$1" ]; then echo "${j#*:}"; return; fi
  done
  echo "$join_at"
}
# --arrive for a viewer that arrives into the running stream, as the twin
# has its viewers arrive (strims-sim's testbed and calibration modules):
# one of JOINS, whatever its time, or of a JOIN_AT after the start.
arrive_of() {
  local j
  [ "$1" -eq 0 ] && return
  for j in "${joins[@]}"; do
    if [ "${j%%:*}" = "$1" ]; then echo --arrive; return; fi
  done
  if awk -v s="$join_at" 'BEGIN { exit !(s > 0) }'; then echo --arrive; fi
}
browsers=0
nbrowsers=0
if [ -n "$media" ] && [ "$(kind_of 0)" != native ]; then
  echo "MEDIA needs a native seed" >&2; exit 1
fi
for i in $(seq 0 $((n - 1))); do
  case $(kind_of "$i") in
    native) ;;
    firefox|chromium) browsers=1; nbrowsers=$((nbrowsers + 1)) ;;
    *) echo "node $i: unknown kind $(kind_of "$i")" >&2; exit 1 ;;
  esac
  if [ -n "$(sctp_args_of "$i" "$(kind_of "$i")")" ]; then
    [ "$(kind_of "$i")" = chromium ] || { echo "TRACE_SCTP: node $i is $(kind_of "$i"), not chromium" >&2; exit 1; }
    [ -n "$(link_args_of "$i")" ] || { echo "TRACE_SCTP: node $i needs LINK_LOG, which names its links" >&2; exit 1; }
    sctp_any=1
  fi
done
if [ "${#sctp_nodes[@]}" -gt 0 ] && [ -z "${sctp_any:-}" ]; then
  echo "TRACE_SCTP=$TRACE_SCTP names no Chromium node" >&2; exit 1
fi
case $browser_links in ws|webrtc) ;; *) echo "BROWSER_LINKS: ws or webrtc" >&2; exit 1 ;; esac
[ -x "$bin" ] || { echo "no testbed_node at $bin" >&2; exit 1; }
# The seed's steps, and the highest rate its link runs at, which a peak is
# over.
for step in $steps; do
  [[ $step =~ ^[0-9]+(\.[0-9]+)?:[0-9]+(\.[0-9]+)?$ ]] \
    || { echo "SEED_UP_STEPS: $step isn't <s>:<mbit>" >&2; exit 1; }
done
seed_top=$(awk -v u="$seed_up" -v s="$steps" 'BEGIN {
  t = u; n = split(s, a, " ")
  for (i = 1; i <= n; i++) { split(a[i], p, ":"); if (p[2] + 0 > t) t = p[2] + 0 }
  print t }')
# Every node's link spec, parsed before any container starts by this
# tree's parser (strims-wan's testbed module, the twins' too), whatever
# commit NODE_BIN is, and the seed's at each of its steps.
link_bin=${LINK_BIN:-}
if [ -z "$link_bin" ]; then
  echo "run.sh: building testbed_link from this tree (LINK_BIN skips it)" >&2
  cargo build --release --locked --manifest-path "$root/Cargo.toml" \
    -p strims-wan --example testbed_link \
    || { echo "building testbed_link failed (or give LINK_BIN)" >&2; exit 1; }
  link_bin=${CARGO_TARGET_DIR:-$root/target}/release/examples/testbed_link
fi
[ -x "$link_bin" ] || { echo "no testbed_link at $link_bin" >&2; exit 1; }
traces=0
step_cmds=()
for i in $(seq 0 $((n - 1))); do
  if [ "$i" -eq 0 ]; then up=$seed_up; burst=$seed_burst; peak=$seed_peak; top=$seed_top
  else up=$viewer_up; burst=$viewer_burst; peak=$viewer_peak; top=""; fi
  link_eval "$(link_of "$i")" "$up" "$burst" $((up * queue * 125)) "$peak" "" "$top" \
    || { echo "node $i: link $(link_of "$i") refused" >&2; exit 1; }
  if [ "$LINK_CLASS" = trace ]; then
    traces=1
    for f in $LINK_FILES; do
      [ -f "${f%%:*}" ] || { echo "node $i: no link file at ${f%%:*}" >&2; exit 1; }
    done
    if [ "$i" -eq 0 ] && [ -n "$steps" ]; then
      echo "SEED_UP_STEPS: a trace link takes no steps" >&2; exit 1
    fi
  fi
  if [ "$i" -eq 0 ]; then
    for step in $steps; do
      rate=${step#*:}
      link_eval "$(link_of 0)" "$rate" "$seed_burst" \
        "$(awk "BEGIN { printf \"%d\", $rate * $queue * 125 }")" "$seed_peak" "$seed_up" "$seed_top" \
        || { echo "SEED_UP_STEPS: link $(link_of 0) refused at step $step" >&2; exit 1; }
      step_cmds+=("$(link_change)")
    done
  fi
done
# The containers' samplers, one process each where a shell loop forks tc
# or ss a sample (the testbed skill has their costs): static, mounted in.
tcsample=${TCSAMPLE_BIN:-$root/target/testbed/tcsample}
sssample=${SSSAMPLE_BIN:-$root/target/testbed/sssample}
# Sampler $1 built at $2 from this tree's hack/testbed/$1.c, where it's
# missing or older than its source.
build_sampler() {
  if [ ! -x "$2" ] || [ "$root/hack/testbed/$1.c" -nt "$2" ]; then
    mkdir -p "$(dirname "$2")"
    gcc -O2 -static -o "$2" "$root/hack/testbed/$1.c" \
      || { echo "building $1 failed (or give ${1^^}_BIN)" >&2; exit 1; }
  fi
}
[ -n "${TCSAMPLE_BIN:-}" ] || build_sampler tcsample "$tcsample"
[ -n "${SSSAMPLE_BIN:-}" ] || build_sampler sssample "$sssample"
for s in "$tcsample" "$sssample"; do
  [ -x "$s" ] || { echo "no sampler at $s" >&2; exit 1; }
done
sampler_mounts=(-v "$tcsample:/tcsample:ro" -v "$sssample:/sssample:ro")
tracelink=$root/target/testbed/tracelink
if [ "$traces" = 1 ] && { [ ! -x "$tracelink" ] || [ "$root/hack/testbed/tracelink.c" -nt "$tracelink" ]; }; then
  mkdir -p "$(dirname "$tracelink")"
  gcc -O2 -static -o "$tracelink" "$root/hack/testbed/tracelink.c"
fi

mkdir -p "$out"
out=$(cd "$out" && pwd)
docker build -q -t "$image" "$root/hack/testbed" >/dev/null
if [ "$browsers" = 1 ]; then
  [ -x "$browser_bin" ] || { echo "no testbed_browser at $browser_bin" >&2; exit 1; }
  [ -d "$web" ] || { echo "no web bundle at $web" >&2; exit 1; }
  web=$(cd "$web" && pwd)
  python3 "$root/hack/testbed/stamp.py" check "$bin" "$web" "$browser_bin" || exit 1
  docker build -q -t "$browser_image" -f "$root/hack/testbed/browser.Dockerfile" \
    "$root/hack/testbed" >/dev/null
  # Written by the nodes, whatever user each runs as.
  mkdir -p "$out/hosts"
  chmod 1777 "$out/hosts"
fi
if [ "${CAPTURE:-0}" = 1 ]; then
  mkdir -p "$out/capture"
  chmod 1777 "$out/capture"
fi
# The network has no route out (--internal), so the shaped uplinks carry
# the testbed's traffic alone: a browser's own fetches (component updates,
# safe browsing, first-run downloads) would cross them otherwise. Nothing
# in a run needs the internet: the binaries and the page are mounted, the
# images built beforehand. It has a name of its own, which no older run.sh
# reuses (they make strims-testbed, routable, whose browsers need its
# route), and goes when the run ends; the two share the subnet, so the
# old one goes first. One of the new name is kept only as this run.sh
# makes it.
#
# One testbed per Docker host: a container named tb-* is another run's,
# live or left by one killed before its cleanup, and this run's would
# take its name (each start removes a container of its name first), and
# a testbed network with containers on it is another run's. Either stops
# this run before it touches anything.
stale=$(docker ps -a --format '{{.Names}}' | grep '^tb-' | sort | tr '\n' ' ' || true)
if [ -n "$stale" ]; then
  echo "containers ${stale% } exist: another testbed run is live, or one killed before its cleanup" \
    "left them; if none is live, remove them: docker rm -f ${stale% }" >&2
  exit 1
fi
for nw in "$old_net" "$net"; do
  on=$(docker network inspect -f '{{len .Containers}}' "$nw" 2>/dev/null) || on=0
  if [ "$on" != 0 ]; then
    echo "network $nw has $on containers on it: another testbed run is live" >&2
    exit 1
  fi
done
names=()
# The run's id, in its control files (/ctl/run), which a step or an upload
# checks in tb-0 before it acts: one left from another run then acts on
# nothing.
run_id=$(od -An -N8 -tx8 /dev/urandom | tr -d ' ')
# The run's control files, in its out dir under its id (.ctl/<id>, which
# goes with the run; what a run killed before its cleanup left goes with
# the next run into the out dir, or with the out dir, and its own work
# left running finds its dir gone): each node's mark that its link is up
# (/ctl/ready-<i>), the stream's start (/ctl/start, which the containers
# wait for), each container's exit code, and what failed while the stream
# ran.
rm -rf "$out/.ctl"
ctl=$out/.ctl/$run_id
mkdir -p -m 755 "$ctl"
echo "$run_id" > "$ctl/run"
ours="[ \"\$(cat /ctl/run 2>/dev/null)\" = $run_id ] || exit 3"
# The run's background work (its steps, uploads and container waits),
# which ends with it, however it ends.
bg=()
kill_tree() {
  local kids c
  kids=$(pgrep -P "$1" 2>/dev/null || true)
  kill "$1" 2>/dev/null || true
  for c in $kids; do kill_tree "$c"; done
}
# The run's background work, its containers, its control files and its
# network, in that order.
cleanup() {
  local p
  for p in "${bg[@]}"; do kill_tree "$p"; done
  docker rm -f "${names[@]}" >/dev/null 2>&1 || true
  rm -rf "$ctl"
  rmdir "$out/.ctl" 2>/dev/null || true
  docker network rm "$net" >/dev/null 2>&1 || true
}
trap cleanup EXIT
for nw in "$old_net" "$net"; do
  have=$(docker network inspect -f '{{.Internal}} {{range .IPAM.Config}}{{.Subnet}}{{end}}' "$nw" 2>/dev/null) \
    || continue
  [ "$nw" = "$net" ] && [ "$have" = "true $subnet.0/24" ] && continue
  docker network rm "$nw" >/dev/null \
    || { echo "network $nw ($have) is in use: another testbed run?" >&2; exit 1; }
done
docker network inspect "$net" >/dev/null 2>&1 \
  || docker network create --internal --subnet "$subnet.0/24" "$net" >/dev/null
# Nor does a container speak IPv6, whose link-local chatter (router
# solicitations, neighbour discovery) would cross them too.
nov6=(--sysctl net.ipv6.conf.all.disable_ipv6=1 --sysctl net.ipv6.conf.default.disable_ipv6=1)

{
  for i in $(seq 0 $((n - 1))); do
    if [ "$browsers" = 1 ]; then
      echo "node $i $subnet.$((10 + i)):4000 $(kind_of "$i")"
    else
      echo "node $i $subnet.$((10 + i)):4000"
    fi
  done
  if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then
    echo "relay $relay:4000 $relay:4001"
  fi
  if [ "$browsers" = 1 ]; then
    echo "browser-links $browser_links"
  fi
  if [ -n "${TOPOLOGY:-}" ]; then
    command grep '^link ' "$TOPOLOGY"
  else
    for i in $(seq 1 $viewers); do echo "link 0 $i"; done
  fi
} > "$out/topology"
# The host's CPU model, which sets what each packet of a Chromium node's
# takes of its network thread: the twin's, where the run has no capture.
while IFS=: read -r key value; do
  if [[ $key == "model name"* ]]; then
    echo "${value# }" > "$out/cpu"
    break
  fi
done < /proc/cpuinfo
# The qdisc tree each node's shape (below) builds, by name: a change to
# the tree changes it, so runs of two trees don't pool. Its root is the
# node's access link class's (the `links` file names each node's).
SHAPER="clsact>ifb-netem>root-class"
env | command grep -E '^(VIEWERS|SEED_UP_MBIT|VIEWER_UP_MBIT|QUEUE_MS|SEED_BURST|VIEWER_BURST|SEED_PEAK_MBIT|VIEWER_PEAK_MBIT|SEED_LINK|VIEWER_LINK|LINKS|SEED_DELAY_MS|VIEWER_DELAY_MS|SECS|JOIN_AT|JOINS|SLOW_DOWN_KBIT|SEED_UP_STEPS|COMPETE|SS_EVERY|SEED_LOSS_PCT|RATE|NODE_ARGS|TOPOLOGY|CAPTURE|PCAP|PCAP_SNAPLEN|KINDS|BROWSER_LINKS|BROWSER_ARGS|RELAY|MEDIA|UNITS_CPS|RECORD|FORECAST|SPEC|PARENT_CHECKS|TIMER_LOG|TIMER_PROBE|LINK_LOG|TRACE_SCTP)=' \
  > "$out/settings" || true
echo "QDISC=$SHAPER" >> "$out/settings"
echo "STAMPS=clock" >> "$out/settings"
echo "JOIN_LINKS=together" >> "$out/settings"
echo "NEIGHBOURS=static" >> "$out/settings"
echo "SWARMS=transfers" >> "$out/settings"
echo "LINES=apart" >> "$out/settings"

# Milliseconds since the epoch, from bash's clock ($EPOCHREALTIME: seconds
# and six digits of microseconds, no fork); the containers' samplers stamp
# their lines the same way (`t=${EPOCHREALTIME/[.,]/}; t=${t%???}`). The
# images' and hosts' date (uutils coreutils 0.10) prints %3N from the
# nanoseconds without their leading zeros, so in a second's first 100 ms
# `date +%s%3N` read up to 900 ms late.
now_ms() {
  local t=${EPOCHREALTIME/[.,]/}
  echo "${t%???}"
}
# What a container waits for before its node starts: the stream's start,
# which run.sh writes once every node's link is up.
wait_start='until [ -s /ctl/start ]; do sleep 0.05; done
start=$(cat /ctl/start)'

# Sampler $1 (mounted at /$1) run with arguments $3 in node $2's
# container, in the background of its script, into $4-$2.log, and what it
# leaves when it exits early (above): its status taken with `||`, as the
# container's script runs under set -e, which would end the subshell
# first.
sampled() {
  echo "( e=0; /$1 $3 > /out/$4-$2.log 2> /out/$1-$2.err || e=\$?; t=\${EPOCHREALTIME/[.,]/}; echo \"\${t%???} \$e\" > /out/$1-$2.end; echo \$e > /ctl/sampler-$1-$2 ) &"
}
# The container's CPU time and memory, once a second, in the background of
# its script, with bash's builtins alone: a fork of sed, cat and sleep a
# second cost 0.012 of a core, this nothing measurable (`read -t` on a pipe
# no one writes waits the second).
cpu_log() {
  echo "( exec 3<> <(:); while :; do t=\${EPOCHREALTIME/[.,]/}; t=\${t%???}; { read -r _ u _; } < /sys/fs/cgroup/cpu.stat; read -r m < /sys/fs/cgroup/memory.current; echo \"\$t \$u \$m\"; read -t 1 -u 3 || true; done > /out/cpu-$1.log ) &"
}
# The container's TCP counters (its network namespace's /proc/net/netstat
# TcpExt line: its names once, `# TcpExt: <names>`, then `<unix ms>
# TcpExt: <values>`) every 100 ms, with bash's builtins, in the background
# of its script.
netstat_log() {
  echo "( exec 3<> <(:); { read -r h; } < /proc/net/netstat; { echo \"# \$h\"; while :; do t=\${EPOCHREALTIME/[.,]/}; t=\${t%???}; { read -r _; read -r v; } < /proc/net/netstat; echo \"\$t \$v\"; read -t 0.1 -u 3 || true; done; } > /out/netstat-$1.log ) &"
}
hosts=""
if [ "$browsers" = 1 ]; then
  hosts="--hosts /out/hosts"
fi
# The run's containers' addresses, and container $1's neighbour entries for
# the others (the header's), as commands of its script, after a check that
# Docker gave its eth0 the MAC the others' entries name.
addrs=()
for i in $(seq 0 $((n - 1))); do addrs+=("$subnet.$((10 + i))"); done
if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then addrs+=("$relay"); fi
if [ -n "$compete" ]; then addrs+=("$sink"); fi
neighbours() {
  local a mac
  mac=$(mac_of "$1")
  printf '%s\n' 'read -r mac < /sys/class/net/eth0/address' \
    "[ \"\$mac\" = $mac ] || { echo \"eth0 is at \$mac, not $mac\" >&2; exit 1; }"
  for a in "${addrs[@]}"; do
    if [ "$a" != "$1" ]; then
      echo "ip neigh replace $a lladdr $(mac_of "$a") dev eth0 nud permanent"
    fi
  done
}
if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then
  names+=(tb-relay)
  docker rm -f tb-relay >/dev/null 2>&1 || true
  docker run -d --name tb-relay --network "$net" --ip "$relay" --mac-address "$(mac_of "$relay")" \
    "${nov6[@]}" --cap-add NET_ADMIN -v "$bin:/testbed_node:ro" -v "$out:/out" -v "$ctl:/ctl" "$image" \
    bash -c "set -e
$(neighbours "$relay")
$(cpu_log relay)
$wait_start
exec setpriv --inh-caps=-net_admin --bounding-set=-net_admin -- /testbed_node --relay-only --topology /out/topology --start \$start --secs $secs 2> /out/node-relay.err" >/dev/null
fi

: > "$out/links"
for i in $(seq 0 $((n - 1))); do
  if [ "$i" -eq 0 ]; then up=$seed_up; delay=$seed_delay; burst=$seed_burst; peak=$seed_peak
  else up=$viewer_up; delay=$viewer_delay; burst=$viewer_burst; peak=$viewer_peak; fi
  limit=$((up * queue * 125))
  link=$(link_of "$i")
  top=""
  if [ "$i" -eq 0 ]; then top=$seed_top; fi
  link_eval "$link" "$up" "$burst" "$limit" "$peak" "" "$top"
  echo "node $i $link $up $delay $queue $LINK_DEV" >> "$out/links"
  # A previous run's link log would pass for this one's.
  rm -f "$out/link-$i.log" "$out/link-$i.err"
  # A trace link's files, and the capabilities its classifier needs.
  link_mounts=()
  if [ "$LINK_CLASS" = trace ]; then
    link_mounts=(--cap-add BPF --cap-add PERFMON -v "$tracelink:/tracelink:ro")
    for f in $LINK_FILES; do
      h=${f%%:*}
      link_mounts+=(-v "$(cd "$(dirname "$h")" && pwd)/$(basename "$h"):${f#*:}:ro")
    done
  fi
  # The node's link (links.sh's node_tree, which linkcheck.sh checks on
  # a farm host): the seed loses what it receives, a viewer what the seed
  # sends it (as the sink does), and the first viewer's downlink is
  # SLOW_DOWN_KBIT's.
  if [ "$i" -eq 0 ]; then match=matchall; else match=$from_seed; fi
  node_slow=""
  if [ "$i" -eq 1 ]; then node_slow=$slow; fi
  shape=$(node_tree "$delay" "$i" "${PCAP:-0}" "$match" "$loss_every" "$node_slow" "$queue")
  # A sampler that exits while its node runs says when and how
  # (<sampler>-<i>.end: strims-sim's reader then takes its log's early end
  # as known, the log's columns ending at its last sample, where it
  # refuses a log that stops short without one) and tells run.sh (.ctl's
  # sampler-<sampler>-<i>), which fails the run: its node's container
  # ending kills it, writing nothing.
  sample="$(sampled tcsample "$i" "-i 100" tc)"
  if [ "$i" -eq 0 ]; then
    sample="$sample
$(sampled sssample "$i" "-i $ss_ms" ss)"
  fi
  if [ "${PCAP:-0}" = 1 ]; then
    # A tcpdump that exits while its node runs says when (its node's
    # container ending kills both, writing nothing), so the capture's
    # columns end there, not at 0.
    sample="$sample
( e=0; tcpdump -i eth0 -n -U -s ${PCAP_SNAPLEN:-128} -w /out/pcap-$i.pcap 2> /out/tcpdump-$i.err || e=\$?; t=\${EPOCHREALTIME/[.,]/}; echo \"\${t%???} \$e\" > /out/pcap-$i.end ) &
( tcpdump -i ifbo -n -U -s ${PCAP_SNAPLEN:-128} -w /out/pcap-$i-offered.pcap 2> /out/tcpdump-$i-offered.err ) &"
  fi
  capture=""
  if [ "${CAPTURE:-0}" = 1 ]; then capture="--capture /out/capture"; fi
  join=$(join_of "$i")
  arrive_arg=$(arrive_of "$i")
  timer_args=$(timer_args_of "$i")
  link_args=$(link_args_of "$i")
  sctp_args=$(sctp_args_of "$i" "$(kind_of "$i")")
  media_mount=()
  media_arg=""
  if [ -n "$media" ] && [ "$i" -eq 0 ]; then
    media_mount=(-v "$media:/media.flv:ro")
    media_arg="--media /media.flv"
  fi
  record_arg=""
  for r in "${records[@]}"; do
    if [ "$r" = "$i" ] && [ -n "$media" ]; then record_arg="--record /out/record-$i.bin"; fi
  done
  profile_arg=""
  if [ -n "${CPU_PROFILE:-}" ] && [ "$(kind_of "$i")" = chromium ]; then
    # Written by the browser's unprivileged user.
    mkdir -p "$out/profile"
    chmod 777 "$out/profile"
    profile_arg="--cpu-profile /out/profile --cpu-profile-at ${CPU_PROFILE%%:*} --cpu-profile-secs ${CPU_PROFILE#*:}"
  fi
  player_arg=""
  if [ -n "$media" ] && [ "$(kind_of "$i")" != native ]; then
    # Written by the browser's unprivileged user.
    : > "$out/player-$i.jsonl"
    chmod 666 "$out/player-$i.jsonl"
    player_arg="--player /out/player-$i.jsonl"
  fi
  name=tb-$i
  names+=("$name")
  addr=$subnet.$((10 + i))
  docker rm -f "$name" >/dev/null 2>&1 || true
  if [ "$(kind_of "$i")" = native ]; then
    docker run -d --name "$name" --network "$net" --ip "$addr" --mac-address "$(mac_of "$addr")" "${nov6[@]}" \
      --cap-add NET_ADMIN "${link_mounts[@]}" "${sampler_mounts[@]}" -v "$bin:/testbed_node:ro" -v "$out:/out" -v "$ctl:/ctl" "${media_mount[@]}" "$image" \
      bash -c "set -e
$(neighbours "$addr")
$shape
tc -s -d qdisc show > /out/qdisc-$i.txt
: > /ctl/ready-$i
$sample
$(cpu_log "$i")
$(netstat_log "$i")
$wait_start
$(link_attached "$i")
exec /testbed_node --index $i --topology /out/topology --start \$start --secs $secs --join-at $join $arrive_arg $rate_arg $units_arg $media_arg $record_arg $hosts $capture $log_args $timer_args $link_args $node_args > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
  else
    # The browser runs as the image's unprivileged user, after root has
    # shaped the link. Its profile, in /tmp, stays in memory, so its
    # writes don't stall the other containers' on the host's disk. The
    # internal network has no default route, and Firefox gathers its ICE
    # candidates from the address a route lookup gives (none, and its
    # links fail at once): a default via an address on the subnet nothing
    # holds ($unused) gives it eth0's, and reaches nothing. (One on-link,
    # `default dev eth0`, would reach the host's own addresses off the
    # subnet, which it answers ARP for.)
    docker run -d --name "$name" --network "$net" --ip "$addr" --mac-address "$(mac_of "$addr")" "${nov6[@]}" \
      --cap-add NET_ADMIN "${link_mounts[@]}" "${sampler_mounts[@]}" --shm-size 1g --tmpfs /tmp:size=1g \
      -v "$browser_bin:/testbed_browser:ro" \
      -v "$web:/web:ro" -v "$out:/out" -v "$ctl:/ctl" "$browser_image" \
      bash -c "set -e
$(neighbours "$addr")
$shape
ip route add default via $unused dev eth0
tc -s -d qdisc show > /out/qdisc-$i.txt
: > /ctl/ready-$i
$sample
$(cpu_log "$i")
$wait_start
$(link_attached "$i")
exec setpriv --reuid=ubuntu --regid=ubuntu --init-groups --reset-env /testbed_browser --index $i --topology /out/topology --web /web --start \$start --secs $secs --join-at $join $arrive_arg $rate_arg $units_arg $hosts $capture $player_arg $profile_arg $log_args $timer_args $link_args $sctp_args $browser_args > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
  fi
done

# A container whose link failed to build (a tc error, a missing module,
# a tracelink that failed) ends before its node starts: the run fails.
for i in $(seq 0 $((n - 1))); do
  until [ -e "$ctl/ready-$i" ]; do
    if [ "$(docker inspect -f '{{.State.Running}}' "tb-$i" 2>/dev/null)" != true ]; then
      # It may have ended since, its node having started.
      [ -e "$ctl/ready-$i" ] && break
      echo "node $i: its link failed:" >&2
      cat "$out/link-$i.err" >&2 2>/dev/null || true
      docker logs "tb-$i" >&2 2>&1 || true
      exit 1
    fi
    sleep 0.2
  done
done
# The stream starts once every link is up (a trace's having loaded its
# trace, however long), its classifier going in at the start: the nodes
# start then, and browsers load the page and compile its wasm.
lead=$(( 3000 + 100 * (n > 4 ? n - 4 : 0) ))
if [ "$browsers" = 1 ]; then
  lead=$(( lead + 20000 + 1000 * nbrowsers ))
fi
start=$(( $(now_ms) + lead ))
echo "$start" > "$ctl/start.tmp"
mv "$ctl/start.tmp" "$ctl/start"

if [ -n "$steps" ]; then
  (
    k=0
    for step in $steps; do
      at=${step%%:*}
      rate=${step#*:}
      # The delay is ahead of the shaper: what the seed sends from the
      # step's time on reaches it that much later.
      ms=$(awk "BEGIN { printf \"%d\", $start + $at * 1000 + $seed_delay }")
      wait=$(( ms - $(now_ms) ))
      if [ "$wait" -gt 0 ]; then sleep "$(awk "BEGIN { print $wait / 1000 }")"; fi
      [ -e "$ctl/run" ] || exit 0
      # The step's time is when its tc returned in the container (the
      # kernel changes the qdisc before it acks tc's request), the last
      # line out: docker exec's own return can trail it by 0.7 s. Errors
      # go apart, docker exec keeping no order between the two.
      began=$(now_ms)
      got=$(docker exec tb-0 bash -c "$ours; ${step_cmds[k]} || exit; t=\${EPOCHREALTIME/[.,]/}; echo \${t%???}" \
        2>"$ctl/step.err") && rc=0 || rc=$?
      returned=$(now_ms)
      [ "$rc" = 3 ] && exit 0
      applied=${got##*$'\n'}
      if [ "$rc" != 0 ] || ! [[ $applied =~ ^[0-9]+$ ]]; then
        echo "SEED_UP_STEPS: the step $step failed: $got$(cat "$ctl/step.err")" > "$ctl/failed"
        exit 1
      fi
      echo "$applied $rate $began $returned" >> "$out/steps.log"
      k=$((k + 1))
    done
  ) &
  bg+=($!)
fi

if [ -n "$compete" ]; then
  # The sink's 10 ms, and, as a viewer does, its loss of what the seed
  # sends it (the upload's data; the seed's own filter takes its acks),
  # each frame alone (no GRO).
  sink_shape="tc qdisc replace dev eth0 root netem delay 10ms limit 100000"
  if [ -n "$loss_every" ]; then
    sink_shape="$sink_shape
ethtool -K eth0 gro off >/dev/null 2>&1 || true
tc qdisc add dev eth0 clsact
$(loss_filter "$from_seed" "$loss_every")"
  fi
  names+=(tb-sink)
  docker rm -f tb-sink >/dev/null 2>&1 || true
  docker run -d --name tb-sink --network "$net" --ip "$sink" --mac-address "$(mac_of "$sink")" "${nov6[@]}" \
    --cap-add NET_ADMIN "$image" \
    bash -c "set -e
$(neighbours "$sink")
$sink_shape
exec iperf3 -s" >/dev/null
  (
    for flow in $compete; do
      at=${flow%%:*}
      dur=${flow#*:}
      ms=$(awk "BEGIN { printf \"%d\", $start + $at * 1000 }")
      wait=$(( ms - $(now_ms) ))
      if [ "$wait" -gt 0 ]; then sleep "$(awk "BEGIN { print $wait / 1000 }")"; fi
      [ -e "$ctl/run" ] || exit 0
      echo "$(now_ms) $at $dur" >> "$out/compete.log"
      docker exec tb-0 bash -c "$ours; exec iperf3 -c $sink -t $dur -i 1 -C cubic -J" \
        > "$out/compete-$at.json" 2>&1 || true
    done
  ) &
  bg+=($!)
fi

end_ms=$(( start + (secs + 15) * 1000 ))
echo "testbed: $n nodes, stream of $secs s; results in $out"
for name in "${names[@]}"; do
  ( docker wait "$name" > "$ctl/exit-$name" 2>/dev/null || true ) &
  bg+=($!)
done
# Until the stream has ended and the nodes have had 15 s to write their
# last lines, a step fails, or a node exits with an error.
failing() {
  local f
  [ -s "$ctl/failed" ] && return 0
  for f in "$ctl"/exit-*; do
    if [ -s "$f" ] && [ "$(cat "$f")" != 0 ]; then return 0; fi
  done
  for f in "$ctl"/sampler-*; do
    [ -e "$f" ] && return 0
  done
  return 1
}
while [ "$(now_ms)" -lt "$end_ms" ] && ! failing; do
  sleep 1
done
for i in $(seq 0 $((n - 1))); do
  docker logs "tb-$i" > "$out/docker-$i.log" 2>&1 || true
done
if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then
  docker logs tb-relay > "$out/docker-relay.log" 2>&1 || true
fi
if [ -s "$ctl/failed" ]; then
  cat "$ctl/failed" >&2
  exit 1
fi
# A node that exited with an error (one that never started, or failed
# while the stream ran) fails the run.
failed=0
for name in "${names[@]}"; do
  code=$(cat "$ctl/exit-$name" 2>/dev/null || true)
  if [ -n "$code" ] && [ "$code" != 0 ]; then
    echo "$name exited with $code (its docker-<i>.log and node-<i>.err)" >&2
    failed=1
  fi
done
# A sampler that exited while its node ran (its log would end there)
# fails it too.
for f in "$ctl"/sampler-*; do
  [ -e "$f" ] || continue
  s=${f##*/sampler-}
  echo "node ${s##*-}'s ${s%-*} exited with $(cat "$f") while it ran (its $s.err)" >&2
  failed=1
done
[ "$failed" = 0 ] || exit 1
