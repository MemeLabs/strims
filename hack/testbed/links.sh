# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# run.sh's access link classes. A node's link is a spec,
# `<class>[,<key>=<value>...]` (run.sh's SEED_LINK, VIEWER_LINK and LINKS),
# that builds the modem behind run.sh's ifb whose netem holds the node's
# delay: eth0's root, or with slots the root of a second ifb (ifbm), which
# eth0's root hands every packet the delay's ifb returns, and whose
# packets eth0's root then holds for their slot (a netem with slots alone)
# before they leave: delay, shaper, then the medium's slots, where a cable
# modem's grants and a station's aggregates act on what its shaper
# passed. strims-wan's testbed module is the one parser of a spec, the
# sims' twins' as well (its docs give the classes, their keys and
# defaults, and what the twins refuse): `link_eval` runs its testbed_link
# example, which run.sh names in `link_bin` (built from run.sh's own tree,
# or LINK_BIN), which fails on a spec it wouldn't build, and sets the
# LINK_* variables it prints; the other functions print shell for the
# node's container from them.
#
#   link_eval SPEC UP_MBIT BURST LIMIT [PEAK_MBIT] [BASE_UP_MBIT] [TOP_MBIT]
#       parses SPEC for a node at UP_MBIT, its rate having started at
#       BASE_UP_MBIT (UP_MBIT without) and reaching TOP_MBIT at its
#       highest, its start's or a step's (the higher of the two without),
#       which a peak is over, with the run's bucket, queue limit and peak
#       (tbf's and fiber's): LINK_CLASS, LINK_STAB, LINK_SHAPER,
#       LINK_CHILD, LINK_SLOT, LINK_TRACE and LINK_FILES
#   link_root DELAY_MS NODE
#       the access link: the modem (handle 10: on eth0, or on ifbm with
#       slots) and the slots; a trace's link loads its trace and waits for
#       the stream's start (the time in /ctl/start, which run.sh writes
#       once every link has loaded), its time 0 DELAY_MS after it
#   link_attached NODE
#       once /ctl/start is there, the wait for a trace's classifier to be
#       in (nothing for another class)
#   link_dev
#       the modem's device: eth0, or ifbm
#   link_change
#       the command that changes the link's rate to link_eval's UP_MBIT
#       (SEED_UP_STEPS): its shaper, and the queue under it (cable's
#       DOCSIS-PIE holds QUEUE_MS at the new rate)
#   node_tree DELAY NODE PCAP LOSS_MATCH LOSS_EVERY SLOW QUEUE
#       a node's whole link as run.sh builds it on its eth0, a command a
#       line, from link_eval's LINK_* (linkcheck.sh renders each class's
#       for a farm host)
#   link_modules
#       the kernel modules run.sh and the classes shape with, for a host to
#       load (a farm host's boot loads them)
#
# The classes, each charging a packet what its link does past its IP bytes
# with tc's size table (strims-wan's Framing over the 14 bytes of Ethernet
# header a qdisc counts):
#
#   tbf     a tbf of BURST bytes (PEAK_MBIT its peakrate, if set): run.sh's
#           link
#   cable   DOCSIS: a token bucket sent at a peak (a line's 100 Mbit/s
#           unless peak= says, and none if the link's rate reaches it),
#           over DOCSIS-PIE (aqm=pie, the default) or a FIFO of LIMIT
#           bytes, and the upstream's requests and grants as slots after it
#   dsl     the line's sync rate, a burst of two frames, LIMIT bytes of
#           modem FIFO, in ADSL's ATM cells or VDSL2's PTM
#   fiber   a tbf of BURST bytes (PEAK_MBIT its peakrate, if set) at the
#           provisioned rate
#   trace   tracelink's delivery opportunities (hack/testbed/tracelink.c),
#           its files mounted at /links
#
# and on any class slot= (what the shaper passed leaves when the next slot
# opens) and, but a trace, router=fq_codel|cake (an SQM home router in
# place of the modem's shaper and queue). A queue's limit holds what waits
# for the shaper, not for a slot.

link_eval() {
  local out
  out=$("$link_bin" --link "$1" --up-mbit "$2" --burst "$3" --limit "$4" \
    ${5:+--peak-mbit "$5"} ${6:+--base-mbit "$6"} ${7:+--top-mbit "$7"}) || return 1
  eval "$out"
}

link_dev() {
  if [ -n "$LINK_SLOT" ]; then echo ifbm; else echo eth0; fi
}

link_root() {
  local delay=$1 node=$2 dev
  dev=$(link_dev)
  if [ -n "$LINK_SLOT" ]; then
    # eth0's root: an unmarked packet (back from the delay's ifb) is
    # marked and handed to ifbm's shaper; a marked one (back from ifbm)
    # waits for its slot.
    echo "ip link add ifbm type ifb && ip link set ifbm up"
    echo "ethtool -K ifbm tso off gso off >/dev/null 2>&1 || true"
    echo "tc qdisc replace dev eth0 root handle 1: prio bands 2 priomap 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0"
    echo "tc qdisc add dev eth0 parent 1:2 handle 30: netem $LINK_SLOT limit 100000"
    echo "tc filter add dev eth0 parent 1: prio 1 handle 1 fw classid 1:2"
    echo "tc filter add dev eth0 parent 1: prio 2 matchall action skbedit mark 1 pipe action mirred egress redirect dev ifbm"
  fi
  if [ "$LINK_CLASS" = trace ]; then
    echo "tc qdisc replace dev $dev root handle 10: prio bands 2 priomap 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0"
    echo "tc qdisc add dev $dev parent 10:1 handle 20: fq nopacing limit 100000 flow_limit 100000 horizon 100s"
    # tracelink fills its maps (about 1 s a million opportunities) and
    # logs `# loaded`, then waits for the start; it fails the container
    # if it exits first.
    echo "( exec /tracelink --dev $dev --parent 10: $LINK_TRACE --start-file /ctl/start --start-ms $delay --log /out/link-$node.log 2> /out/link-$node.err ) &"
    echo "tracelink=\$!"
    link_wait "$node" loaded
    return
  fi
  echo "tc qdisc replace dev $dev root handle 10: ${LINK_STAB:+$LINK_STAB }$LINK_SHAPER"
  if [ -n "$LINK_CHILD" ]; then
    echo "tc qdisc add dev $dev parent 10:1 handle 20: $LINK_CHILD"
  fi
}

# The wait for tracelink's `# <line>` in NODE's log, which fails the
# container if tracelink exits first.
link_wait() {
  echo "until grep -q '^# $2' /out/link-$1.log 2>/dev/null; do kill -0 \$tracelink 2>/dev/null || { echo \"tracelink: \$(cat /out/link-$1.err)\" >&2; exit 1; }; sleep 0.05; done"
}

link_attached() {
  # The classifier is in once tracelink logs its base.
  if [ "$LINK_CLASS" = trace ]; then link_wait "$1" base; fi
}

link_change() {
  if [ -z "$LINK_SHAPER" ]; then
    echo "echo 'a $LINK_CLASS link takes no steps' >&2; false"
    return
  fi
  local cmd
  cmd="tc qdisc change dev $(link_dev) root handle 10: ${LINK_STAB:+$LINK_STAB }$LINK_SHAPER"
  if [ -n "$LINK_CHILD" ]; then
    cmd="$cmd && tc qdisc change dev $(link_dev) parent 10:1 handle 20: $LINK_CHILD"
  fi
  echo "$cmd"
}

# A node's link as run.sh builds it on the node's eth0, from link_eval's
# LINK_*, a command a line:
#
#   node_tree DELAY NODE PCAP LOSS_MATCH LOSS_EVERY SLOW QUEUE
#
# eth0's offloads off, the class's modem (link_root, NODE's), and the
# delay's netem (DELAY ms) on an ifb behind eth0's clsact redirect; with
# PCAP 1, a mirror of what the node offers to an ifb (ifbo) ahead of it;
# with LOSS_EVERY, the ingress filter that drops one packet in LOSS_EVERY
# of those LOSS_MATCH (a tc match) takes; with SLOW (kbit/s), the
# downlink's tbf on an ifb behind an ingress redirect, QUEUE ms of it.
# linkcheck.sh builds each class's on a farm host.
node_tree() {
  local delay=$1 node=$2 pcap=$3 loss_match=$4 loss_every=$5 slow=$6 queue=$7 offered=""
  if [ "$pcap" = 1 ]; then
    echo "ip link add ifbo type ifb && ip link set ifbo up"
    offered="action mirred egress mirror dev ifbo "
  fi
  echo "ethtool -K eth0 tso off gso off gro off >/dev/null 2>&1 || true"
  link_root "$delay" "$node"
  echo "ip link add ifbd type ifb && ip link set ifbd up"
  echo "ethtool -K ifbd tso off gso off >/dev/null 2>&1 || true"
  echo "tc qdisc add dev ifbd root netem delay ${delay}ms limit 100000"
  echo "tc qdisc add dev eth0 clsact"
  echo "tc filter add dev eth0 egress matchall ${offered}action mirred egress redirect dev ifbd"
  if [ -n "$loss_every" ]; then
    echo "tc filter add dev eth0 ingress prio 1 $loss_match action gact continue random netrand drop $loss_every"
  fi
  if [ -n "$slow" ]; then
    echo "ip link add ifbs type ifb && ip link set ifbs up"
    echo "tc filter add dev eth0 ingress prio 2 matchall action mirred egress redirect dev ifbs"
    echo "tc qdisc add dev ifbs root tbf rate ${slow}kbit burst 3028 limit $((slow * queue / 8))"
  fi
}

# Every run's: the delay's netem on an ifb behind eth0's clsact and its
# matchall redirect, tbf, and SEED_LOSS_PCT's u32 match and gact. The
# classes': cable's PIE, the routers' fq_codel and cake, the trace's prio,
# fq and tracelink's cls_bpf classifier, and the slots' prio, whose fw
# filter takes what skbedit marked coming back from ifbm.
link_modules() {
  echo "sch_netem sch_tbf ifb sch_ingress cls_matchall act_mirred cls_u32 act_gact" \
    "sch_pie sch_fq_codel sch_cake sch_prio sch_fq cls_bpf cls_fw act_skbedit"
}
