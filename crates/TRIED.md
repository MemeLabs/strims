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
  links deliver): thin-15-10 stayed collapsed on 8 of 8 seeds.

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

## Request control

**A BBR-like request controller.** (`BbrLike`, retired in 3c40ec37) It
halved tail lag behind capped seeds and recovered from round-trip steps in
about a second, but behind a seed with less upload than the bitrate it
delivered 1.5–2.5 points less (viewers' requests overlapped), its p99 was
about 9 % worse on sparse lossy meshes, pulling behind a cable seed on WAN
paths it lagged far behind LEDBAT, and on a shared parent it yielded to
LEDBAT.
