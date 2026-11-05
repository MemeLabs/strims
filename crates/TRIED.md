# Tried and dropped

Ideas that were built, measured and lost, so they aren't retried by
accident. Retrying one is fine when it's deliberate, because the code it
depended on changed or there's a new angle on why it lost; then replace its
entry with the new result, or delete it if it wins.

An entry gives what was built, the mechanism that sank it and the numbers
that decided it, in a few lines, with the commit it was measured against
(or that recorded it) and its branch if one is kept. Ideas never built,
work in progress, what landed, run logs and per-cell tables don't belong
here. Delete an entry once the code it measured is gone. Keep the file
short enough to read in one sitting.

## Push trees

**A cancelled push as a late sample of its parent's delay.** (`lag-miss`,
against 4a0cd7db) A child's miss counted in its parent's `LagEwma` as a
sample at its deadline (the first miss after a hit; mean and deviation, not
the peak), so the estimate would stop seeing only the pushes that beat the
cancels. Every counted miss lengthens the parent's deadlines (star-7-10,
seed 26010: the median deadline at a miss went from 130 to 540 ms), and
where other holders answer quickly a tight deadline and a pull beat waiting
for the push. Robust tier, 32 seeds: collapsed 245 → 259, healthy stars'
lag p99 ×1.06–1.20 (star-7-10 ×1.20 [1.12, 1.29]), no gain anywhere.

**Cancelling a replaced parent's expected chunks at it.**
(`replaced-cancel`, against 4a0cd7db) A subscription remembered the parent
a switch replaced and the range it was still expected to push, and
cancelled there the chunks it missed. That removes the duplicates it aims
at (pulled, then pushed by the old parent: 534 → 103 on the super-peer
varying case, seed 4), but they are few: duplicates fell 0.6 % over the
robust tier. Most of a switch's duplicates are the old parent's queued
pushes of chunks the new parent pushes too, which STREAM_CANCEL leaves
queued; also cancelling the announced chunks from the new parent's start
(`replaced-cancel-ahead`) didn't reach them.

**Barring or probing holders nobody reports on.** (`announce-drop`,
`announce-probe`, `follower-probe`; recorded in 25ea2ed1) A holder whose
children aren't the seed's neighbours goes unjudged. Barring one after
15 s without a reporter, and probing the holders reports leave unjudged,
both barred honest holders without improving p99.

**Shedding on push queue wait.** (recorded in d1552e65) It catches slow
parents sooner but hurts `sim_push_parent_dies` and sparse meshes.

**Copies to the fastest uplinks in alternating regions** (recorded in
298545b0) did no better than copies to the nearest neighbours on the
current paths.

**A first-wave copy waiting for a nearer recent asker.** (recorded in
d82c04df) It stopped the first wave's later copies from going to farther
neighbours without holders' shares, and cost 15–20 % of p99 with them.

**Dropping a cancelled stream's queued pushes at the old parent.**
(`queued-cancel-only`, `queued-cancel-from`, `queued-cancel`; against
bde51268) On STREAM_CANCEL the parent dropped at write time what it still
had queued of the stream for that child: all of it, only from a start
chunk the cancel named (the new parent's start after a switch, a wire
change), or every push of a stream the child no longer subscribes to. By
the time the cancel arrives the old parent's pushes are mostly below the
runner: in a 50-viewer cable-seed run the drop caught none of the ~620
pushes that came from former parents. Duplicates are about 1 % of chunks
received; the first two variants left them within ±2 % over the robust
and peers tiers, and the named start made wan-super-peer-varying's p99
×1.26 [1.01, 1.57] at 32 seeds. Dropping for every lost subscription cut
duplicates 3–6 %, but star-31-5 collapsed on 6 of 16 seeds against 2.

**Other prune backoffs.** (`prune-backoff5`, `prune-backoff-liar`, against
c72c399e) A parent that had pushed backed off 5 s at first instead of 2 s:
robust tier collapsed 192 against main's 186 at 32 seeds. Escalating only
a parent that pushed nothing, with a fixed 10 s for one that had pushed:
184 against 186, and the seed-starved cell's delivery 1.8 points lower.
Both kept most of the liar gain (honest p99 ×0.88 and ×0.87, against
×0.83 for 2 s).

**Deferring a push miss until its unit shows its count.** (`elide` 3974cf79
as PPSPP_X_DEFER, against ac965fa1; first built on `video/ppspp-sign-gated`)
A deadline that passed while a child held none of the chunk's unit, and no
peer had announced it or a later chunk of the unit, released the chunk to
pull unjudged; its miss counted once a chunk of the unit showed whether it
held data. The hold already covers it: a unit's data-less chunks are held
once any of its chunks arrives, within `DEADLINE_MAX` of the unit's first
announcement. At 32 seeds and u8,t300 it matched counting at the deadline
in all but two cells of `sign-confirm` (churn@1500v 8 against 11 of 32
collapsed, n.s.).

**A store rate of data alone for a stream signed early.** (`elide-c`
48a006aa) The hybrid's store rate left data-less chunks out, so a VBR
stream's quiet scenes opened subscriptions its loud ones couldn't carry:
`sign-sens` at u8,t300, 32 seeds, against the padding sent: 1500v:s1.2 lag
p99 x1.19-1.29 in the residential, overload and churn cells (first waves
x1.33-1.39, honest holders x1.19-1.30, liar grafts x1.35-1.64). Counting
them as stored brought those cells to x1.05-1.14.

**Freshness against a quiet stream's latest data.** (`elide-d` 6be4456e,
on `elide-c`) A peer's announcement lag stayed fresh within
`CANDIDATE_FRESHNESS` of the stream's latest data here, and a seed's check
judged a holder's pushes only when its streams had published since the
last, for the streams an early unit leaves without data for seconds. No
effect: against `elide-c` at 32 seeds, lag p99 x0.999 (`sign-confirm`) and
x1.006 (`sign-sens`).

## Upload meter

Relay readings other than the bound in `UploadInfo::relay_capacity`,
measured against 91ded310 (branch `thin`, switches in `STRIMS_X_METER`):

- **2× the peak write rate:** heals thin relays (thin-15-10 collapsed
  16 → 2 of 16), but where busy time is accurate, as in the ppspp sim,
  relays then read 0.03–0.3 of their uplinks: lag p99 ×1.4–3.5 in ten of
  the eleven ppspp cells, and ×1.13–1.23 in five WAN cells.
- **A bound only once a queue shows:** thin-15-10 still collapsed on 4 of 8
  seeds, against 2 with the bound from the start; relays take children
  for twice their uplinks before any queue shows. **Time with a queue
  below the runner counted as busy:** 8 of 8.
- **Shedding on a 50 ms queue as well as on busy time:** thin-15-10
  collapsed 16 → 8 where the bound alone gave 2, and wan-mobile ×1.29.
- **Clamping on the least latest excess across a relay's links**, to ignore
  per-connection window backlog and peers' queues: it loses the thin
  relay's own queue, and `collapse_thin_relays`, `collapse_pull_lock_in`
  and the residential varying WAN case fail.
- **A floor under every relay reading while no queue forms** (4 × 32 KB/s):
  a viewer in the residential varying WAN case missed 4,662 frames; on the
  busy-time reading alone, star-3-10's lag p99 went ×1.49.
- **The delivered rate under a standing queue, taken more carefully**
  (only while the links' latest round trips show the queue too, the largest
  of the last 2 s, or only while the node writes at least 90 % of what its
  links deliver): thin-15-10 stayed collapsed on 8 of 8 seeds. The largest
  of the last 2 s, retried for a seed whose reading fell to a third of its
  halved uplink (branch `drain-max`, on 2bcca388), didn't reach that dip,
  which was the busy time's reading while stale pushes were dropped:
  halve-7-10's lag p99 ×1.06 and halve-15-10's ×1.07 at 16 local seeds.

**Twice the rate below saturation, and holding a saturated reading.**
(branch `upload-meter`, recorded in d16a7b92 and de76f727) In a screen of
five uncapped cells, reading twice the rate below saturation made p99
55–64 % worse, relays declining everything; capping the reading at twice
the rate while busy shows no saturation, and sizing no first wave without
a known capacity, made 60 s runs worse. A saturation detector on the
queue delay did nothing alone; holding what it measured for the stream's
life cut p99 16 %, but only because no uplink in the sim changed.

**Other queue-delay samples.** (branch `queue-delay-source`, recorded in
bdd133a7) HAVE round trips and ACK one-way delays measured the queue no
better than pings did on a real link.

**The seed's budget on the relay bound.** (branch `repair`: eaf110fc
against its parent 9755d210, on bde51268) The seed's budget took `relay_capacity` for the busy time's
twentyfold over-read, which leaves a 10 Mbit/s seed with 7 viewers seven
copies and no headroom when its uplink halves. Its own munro bursts show
its links a queue, so the bound held the reading near what it wrote and
the budget at one or two copies, and trees deepened: at 16 local seeds
star-3-10's lag p99 went ×2.06, halve-7-10 collapsed 12 times against 7
and halve-15-10 9 against 5; only halve-7-20 gained (×0.42).

**Admitting by count until the uplink saturates.** (`idle-count`,
`idle-count-all`, against 5d81aaf6) An idle relay reads twenty times its
control traffic, short of one subscriber. Until its meter had seen
saturation (busy half the time, a standing queue, or the links' latest
excess at 25 ms), a node also took subscribers while its committed push
stayed within two past its upload rate: of its interior stream, or of any.
Since the relay floor the case is rare (star-15-10: 2 of 180 interior
decisions), and taking those children didn't help. At 32 fresh seeds the
robust tier collapsed 176 → 182 (interior) and 178 (any), the peers tier
67 → 71 and 77, and lag p99 stayed ×1.00. star-3-10, the cell it aimed
at, went ×1.13 [1.00, 1.27] with the interior stream alone.

**A slow link's data left out of the busy time by its round trips.**
(`own-queue` 29c6001c, `own-queue-v1` 1ec374e7 without its gate; against
e5962e82) A peer's data in the runner wasn't busy time while its link's
latest round trip stood 50 ms above the node's latest excess and the
links' recent round trips showed no queue on the uplink (under 25 ms). It
cured `collapse_slow_viewer` (the healthy viewers' p99 1.06–3.01 s →
0.25–0.33 s at seeds 1–8), but a link's round trip shows what it carries
as well as its path: each link that carries much waits behind its own
buffer at its share of the uplink, so under a full uplink those links
stand out from the ones that carry little, and TCP's sawtooth drains the
shared queue often enough to open the gate. It changed every run of the
scarce-seed stars, where no link is slow; at 32 seeds the robust tier
collapsed 162 → 195 (star-31-5 2 → 12, p=0.01), halve-7-20's lag p99 went
×1.26 and wan-super-peer's ×1.19, and the peers tier's ppspp cells tied
(×0.997). Without the gate, the seed behind a competing upload read busy
0.03–0.3 against 0.9–1.0 (`wan_residential_seed_adaptive_uncapped_varying`
lost 623 of v0's frames at seed 6); left out of the push-busy time too,
the holders' waits let `collapse_first_wave_overshoot`'s ramp open 6
copies where the uplink pays for 2; a gate on the latest excess closed at
every munro burst.

**A slow link's data left out of the busy time by the runner's counts.**
(`own-bytes` ae307728, against 7c37a448) A peer's data wasn't busy time
once it had been queued without a break for 1 s while a peer with nothing
queued took at least as many bytes over the meter's window. It cured
`collapse_slow_viewer` at seeds 1–7 (250–314 ms at p99; seed 8 1006 ms).
Without the 1 s, a burst's children drain one by one, each having taken as
much as the next, and 63 of 80 runs of the robust cells v4 lost changed.
With it, near a full uplink TCP holds far links back while near ones
drain, which reads the same: wan-super-peer's seed, sending 1.8–2.1 MB/s
of its 2.5 MB/s, left out far links' backlogs of 1–2.5 s at 90–220 KB/s
while a drained peer took 340 KB/s. At 32 seeds wan-super-peer's lag p99
went ×1.15 [1.10, 1.21] (parent changes ×1.15, startup p99 ×1.10) and
wan-residential-seed's ×1.07 [1.03, 1.12].

**Other ways to keep a slow link from reading as a busy uplink.**
(`slow-fix-exp` 9458d27b and the knobs before it; against b0ab696c) Fed to
each reader of the busy time alone, a busy time without the slow link's
data cured `collapse_slow_viewer` only through the seed's budget: through
shedding alone the healthy viewers lagged more (1.0–3.9 s at p99), and
relay admission and the window changed nothing. Barring a holder whose
pushes were overloaded, or whose link read behind and short of its
streams, while most holders kept up, didn't cure it alone (the budget
still fell on the slow viewer's answers), and beside the budget's rule it
raised collapses (32 local seeds: star-15-5 5 → 9, wan-super-peer-varying
4 → 8; with raw delivery rates the farm's robust tier collapsed
162 → 176). Shedding the children whose data waited longest first changed
nothing. A budget on the time pushes waited, not answers, cured nothing
and lost frames under a competing upload
(`wan_super_peer_adaptive_uncapped_varying` seed 4). As the test of a slow
link, its recent excess alone (≥ 50 ms) took healthy links at munro
bursts (wan-super-peer ×1.06), and raw delivery rates (under half the best
other link's) took far links that TCP holds back under a competing upload
(seed 4 again), which the window's base round trip corrects.

## Request control

**A BBR-like request controller.** (`BbrLike`, retired in 3c40ec37) It
halved tail lag behind capped seeds and recovered from round-trip steps in
about a second, but behind a seed with less upload than the bitrate it
delivered 1.5–2.5 points less (viewers' requests overlapped), its p99 was
about 9 % worse on sparse lossy meshes, pulling behind a cable seed on WAN
paths it lagged far behind LEDBAT, and on a shared parent it yielded to
LEDBAT.

**A deadline-aware request scheduler.** (`deadline`, against a356f676)
Switches over pull: a timed out request sampled its channel's answer time
at twice its wait, so liars ranked slow; a request past its channel's mean
answer time plus four deviations was cancelled there and left to the
others; the reader's next missing chunk left its parent's push at the
parent's horizon, not its deadline; and request rounds asked oldest first,
not in the node's order within a group. Readers read chunks as soon as
they arrive and never skip, so a deadline from first sight has no slack to
spend and can only move repairs earlier. Moving them lost (ppspp sim,
16 seeds: wan-mobile ×1.08–1.22, wan-global ×1.08–1.32): overdue pushes
mostly still arrive, and pulling them adds load. The rest tied once an
empty window carried a request (48 seeds: ×0.92–1.03 on capped-seed,
sparse-lossy, wan-mobile, wan-global and residential).

**One request through any empty window.** (`one-request`, against
a356f676) A window with nothing in flight always asked for a chunk, which
also ended LEDBAT's silence after a timeout (its 2 KiB minimum is under a
4 KiB chunk for `FLOOR_LOSS_HOLD`). The ppspp sim's pull gained (peers
tier, 32 seeds: large-4mbit and overload ×0.81, wan-mobile ×0.93), the
real stack's busy stars lost (robust tier: star-31-10 and mesh6-31-10
×1.20, star-15-5 and thin-15-10 ×1.15 with timeouts ×1.12–1.28; lock-in
×1.16): there a peer that timed out is better left alone for the hold.
A minimum of two chunks (`deadline`'s `min2`) ends the silence too, and
scored the same in the ppspp sim (48 seeds).

**Thin relays' own uplink queue, by its round trips.** (against 75d9ca91
on the calibrated window model, thin-15-10 at 16 seeds: 11 collapses)
A relay's uploads hold the acknowledgements of what it receives in its
uplink's queue. Pacing all a relay sends (`uplink-pace`: the answer pace's
rule over every DATA write of a node that seeds nothing, 25 ms target,
the links' excess alone) collapsed 16 of 16 with lag p50 5.3 s: in a mesh
of busy thin relays each link's round trip carries the far relay's queue
too, as its echoes wait there, so the relay paces for queues it doesn't
hold. Pacing it on the model's true uplink queue instead (an oracle,
`own-queue-oracle`) collapsed 7 of 16 at 25 ms and 13 at 10 ms: no better
read of the queue cures the cell, as a pace holds a mean while the
acknowledgements wait behind each window's burst. A 15 ms answer target:
16 of 16 (repairs starve).
Uplink queues of 20 or 50 ms in place of 100: 16 of 16 (losses).

**A paced answer rate grown only to twice what answers sent.**
(`answer-validation`, against 1a97c9ad on the calibrated window model)
Thin relays' paced rate swings from 32 KB/s to 1 MB/s, three times their
uplinks, as answers wait for a burst's tokens; capping its growth at twice
the answers' rate over the last quarter span left thin-15-10 collapsing 25
of 32 times (24 without), lag p99 ×1.03 [0.93,1.13]. The cell collapses on
the acknowledgements that wait behind a relay's uploads, not on the
answers' bursts.

## Sim model

**Ethernet framing on the access links.** (on `tcp-bench`, against
d416f5c4) Counting 14 bytes more a packet on the window model's access
links, as if the testbed's tbf charged Ethernet's header, doubled the bare
TCP bench's drops where two holders share a 5 Mbit/s uplink (152 -> 313,
the testbed 178-192; the halving 217 -> 292, the testbed 194-238) and
left the 25-viewer twin of the testbed's uplink cut where it was (39.5
cut seconds over 1 s against 37.5).

## WebRTC links

**A smaller data channel cap.** (`browsers-exp`, against 226a5d43)
`WebRtcConfig::max_buffered_amount` at 32 or 64 KiB in place of 256 KiB,
on the sim-suite's browsers tier. The buffered amount counts the flight
too, so a cap holds back throughput where it would hold back the queue:
halve-15-10 at seeds 2001-2008 collapsed 6 of 8 at 64 KiB against 8 of 8
(lag p99 4.3 -> 2.4 s), and before 8 KiB messages the tier went 191 -> 184
collapsed of 512.

**16 KiB messages.** (`browsers-exp`, against 7a90d302) Collapsed fewer
than 8 KiB on the browsers tier (309 against 319 of 1024 at 32 seeds) but
broke star-3-10's lag p99 (x2.46) and star-7-20's (x1.77) at both seed
sets; 8 KiB landed.

**A larger channel pacer slack.** (`pacer-exp`, against the channel
pacer) `SLACK` at 24, 32 and 48 KiB in place of 16 on the browsers tier
at seeds 3001-3032: 251, 258 and 273 collapsed of 1024 against 250. 32
KiB cured the pacer's star-3-5 lag (p99 x0.79), which the host pacer
cured too.

**Coalescing a WebRTC link's small writes.** (`channel-pacer`,
`WebRtcConfig::coalesce`) Holding writes under 1 KiB for 10, 25 or 50 ms
after a link's last: at 10 ms 112 collapsed of 512 against 118 on the
browsers tier, and with HAVEs to a peer that announced the chunk at the
tick 101 but star-15-10's lag p99 x1.37. Deferring those HAVEs without
coalescing: 233 collapsed of 1024 against the pacer's 250, and 197 with
the host pacer.

**A host pacer over the channels' summed buffered amounts.**
(`pacer-exp`'s `agg`, `agg2`) Counting every channel's buffered amount
whole let a neighbour behind a slow downlink take its parents' caps
(wan-slow-viewer 14 -> 25 of 32); counting each at most its bandwidth over
the host's horizon undercounted channels whose rate went stale before a
burst (star-31-5 lag p99 x1.36). The host's own round trip (its summed
amounts') is its nearest neighbour's: a cap over it starved an LTE viewer
of `wan_webrtc_channels` (seed 13: lag p50 2.4 -> 14.6 s).


**HAVEs that wait at every node.** (`thin-rtc`, `pace250-ride`, against
e20aac07, with the pacers' 250 ms rate window) HAVEs that can wait went
only with a frame that carried something else to the peer, or at the
tick, at seeds and at relays without a queue too. With the eager windows
kept, the browsers tier collapsed 131 -> 110 of 1024 but robust's TCP
cells 139 -> 153 of 1248, star-3-5's lag p99 359 -> 878 ms and its
parent changes 0.67 -> 8.67 a minute: a 5 Mbit/s seed's HAVEs past its
queue are what its three viewers pull by and choose parents by. With no
eager HAVEs after a channel opens or a request, browsers collapsed 83
(thin-15-10 32 -> 14, halve-31-10 28 -> 17), but robust 145, lag p99
x1.71 on star-3-5, x1.62 on star-7-10 and x1.49 on limited-15-10, and
the peers tier's grabbers and no-greedy lost delivery; keeping the eager
HAVEs on new channels, star-3-5 went x1.91. A 100 ms rate window in place
of 250 left relays' hosts reading 1.2 times their uplinks (browsers 103
against 83; star-15-5's lag p99 x1.58).

**A browser host's egress pacer on other rules.** (`rtc-pace`,
`rtc-pace-land`, `rtc-pace-v11`, `tmp-*`, against 33ba7038 with a
queued relay's HAVEs at the tick and admission by committed push; browsers
tier at seeds 3001-3032, base 168 of 1024 collapsed) At 1.25 times the
most the links carried (bytes in flight over their round trips) over
10 s, with a 50 ms bucket: on the testbed's browsers it ran at 2.8 times
the seed's uplink and twice a relay's, and held nothing back; 168 -> 149.
At 1.25 times the most they delivered, it held hosts under their demand
(thin-15-10 6 of 8 local seeds against 2); with 0.9 of it while a queue
stood, every host of a swarm of queued relays read its peers' queues as
its own (the excess takes the echoes' way back) and drained to a
collapse (star-31-5, wan-super-peer 8 of 8). At 2.5 times with a 20 ms
bucket, without the growth: 168 -> 108, but every host paced at the first
wave under a 20 Mbit/s uplink while its readings were young, and healthy
stars' request timeouts went x3, their lag p99 x1.2-1.6 (seven
guardrails). Pacing only once a queue had stood 0.5-1 s left the stars
so; 2 s cured them but thin-15-10 collapsed again (its first wave's
losses start it). Going unpaced 2 s after the last queue, not 10, left
timeouts x2.5. Growing 10 % an update: 113, but star-15-10's lag p99
x1.29 and star-15-5's x1.24. Under the browser twin (dcSCTP's SCTP,
serial links), with the growth and browsers held to nine peers, in place of the
400 ms hold (local, star-7-5 at seeds 3001-3016, thin-15-10 and star-31-5
at 3001-3006): a target of 50 or 100 ms took star-7-5's lag p99 x0.90 and
x0.74 of the pacer's without a hold (x0.75 with it), but thin-15-10's lag
p50 x1.26 and x1.53 and star-31-5's x1.22; headroom 4 left star-7-5
(x1.01). Headroom 1.2 and 1.1 there, without the budget: 145 -> 157 and
163 of 1024 collapsed, the WAN cells' lag p99 up.

**Data channel write buffers for 16 KiB.** (`msg-bufs`) With one-frame
4.5 KiB messages a link's two write buffers hold half what two 8 KiB
writes did, and star-7-20's seed read its first wave busy and shed 64
subscriptions at its worst seeds (startup p99 189 -> 415 ms at seed 3009).
Four buffers stopped those sheds (244 ms), but on the browsers tier at
seeds 3001-3032 collapses went 142 -> 168 of 1024 and halve-7-20's lag p99
x1.27: what waits below the runner hides the queue from the upload meter
everywhere else too.

**The first wave's ramp ending after three steps of queue.** (`first-wave`,
`RAMP_OVER` = 3) Two steps of a queue below the runner can be an uplink
that keeps up but queues each burst of the stream, where both steps'
probes met bursts: star-7-20's seed (browsers) kept its 7 copies at its
three shedding seeds (startup p99 320-388 -> 224-230 ms). At seeds
3001-3032 the browsers tier collapsed 130 -> 140 of 1024 (star-31-5 3 ->
10) and robust's TCP cells 143 -> 160 of 1248, lag p99 x1.043 (halve-7-10
7 -> 18, Fisher p 0.01; star-7-10's lag p99 x1.35): where the uplink is
short, the ramp's quarter second more of overshoot costs more than the
bursts' sheds do.

**A host pacer cap over a round trip and 20 ms.** (local, star-7-20 on the
browsers tier at seeds 3001-3032) In place of twice the round trip and 50
ms per channel, so that a burst at a near-full uplink waits in the runner
as it does over TCP: no run shed its first wave (3 before) and lag p99
went 220 -> 185 ms at the median, but the busy time then read the seed's
capacity at its uplink, and the ramp, bounded by the copies that pays
for, took 2 s to its 7 copies: request timeouts 4-21 -> 20-35 a run,
startup p99 to 645 ms. The host pacer's cap had followed the delivered
rate from the first round trips, so a slow start's delay exit would have
held nothing the cap didn't.

**The ramp ending only once its least link shows the queue too.** (offline,
over the ramps' logged readings at seeds 3001-3032) It would have kept
star-7-20's copies, but star-7-10's least links (browsers), where 4 copies
are 105 % of the uplink, read 7-22 ms at 7 of its 13 ends, and TCP's at 3
of 23: under a bursty stream a step's probes can't tell 92 % of an uplink
from 105 %.

