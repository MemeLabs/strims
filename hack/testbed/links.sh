# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# run.sh's access link classes. A node's link is a spec,
# `<class>[,<key>=<value>...]` (run.sh's SEED_LINK and VIEWER_LINK), that
# builds the modem: eth0's root, behind run.sh's ifb whose netem holds the
# node's delay. The functions print shell for the node's container.
#
#   link_root SPEC UP_MBIT BURST LIMIT PEAK_MBIT ORIGIN_MS NODE
#       eth0's root (handle 10:): the access link
#   link_slot SPEC
#       netem's slot argument for the delay's netem, if the spec has one
#   link_change SPEC UP_MBIT BURST LIMIT PEAK_MBIT BASE_UP_MBIT
#       the command that steps the link's rate (SEED_UP_STEPS) from
#       BASE_UP_MBIT, the rate it started at, to UP_MBIT
#   link_files SPEC
#       the host files the link reads, mounted at /links
#   link_modules
#       the kernel modules run.sh and the classes shape with, for a host to
#       load (hack/sim-farm/user-data.sh does)
#
# The classes, each charging a packet what its link does past its IP bytes
# (tc's size table over the 14 bytes of Ethernet header a qdisc counts,
# strims-wan's Framing), their defaults the figures strims-wan's access
# profiles cite:
#
#   tbf     a tbf of BURST bytes (PEAK_MBIT its peakrate, if set): run.sh's
#           link, Ethernet's 14 bytes
#   cable   DOCSIS: a token bucket of burst= bytes (default 125 ms of the
#           rate, a CMTS's Maximum Traffic Burst at Cisco's MSTR/64) sent at
#           peak= Mbit/s (default 100, four bonded D3.0 channels), over the
#           modem's queue, aqm= pie (DOCSIS-PIE, RFC 8034's target 10 ms and
#           update 16 ms: D3.1's, the default) or fifo (LIMIT bytes, D3.0's
#           buffer); and the upstream's requests and grants as slots of
#           slot=<min>-<max> ms (default 2-8, request to grant: CableLabs'
#           LLD white paper), ahead of the bucket; DOCSIS's 18-byte frame,
#           at least 64
#   dsl     the line's sync rate, a burst of two frames, LIMIT bytes of
#           modem FIFO; line= adsl (ATM: PPPoE over LLC/SNAP's 40 bytes,
#           padded to 48-byte payloads of 53-byte cells) or vdsl (PTM:
#           PPPoE and a VLAN, 30 bytes, and a byte in 65 off the rate)
#   fiber   a tbf of BURST bytes at the provisioned rate, Ethernet's frame
#           with its FCS, 18 bytes
#   trace   tracelink's delivery opportunities: file= a trace (Mahimahi's
#           format; format=us for tracelink's), scale=, offset= s into it,
#           delay=<file>@<step ms> (a one-way delay trace added in FIFO
#           order), LIMIT bytes held at most (limit= to set them); for
#           cellular and satellite uplinks. The trace's time 0 is ORIGIN_MS.
#
# and on any class:
#
#   slot=<min>-<max>  slots on the delay's netem (netem `slot`): what is
#           ready leaves when the next slot opens, min to max ms after the
#           packet that draws it; a Wi-Fi station's aggregates, or cable's
#           grants
#   router=fq_codel|cake  the home router shapes the uplink at pct= % of
#           UP_MBIT (default 95), with that AQM, in place of the modem's
#           shaper and queue (SQM), the class's framing kept
#
# The slots come ahead of the shaper, where a cable modem's grants follow
# it: an idle link's first packet waits its slot, then the bucket's bytes
# go at the peak, but under load the shaper paces what the grants would
# send in batches (a stage after eth0's root would need a second device the
# packets can return through).

link_opt() {
  local spec=$1 key=$2 default=$3 kv kvs
  IFS=, read -r -a kvs <<< "$spec"
  for kv in "${kvs[@]:1}"; do
    if [ "${kv%%=*}" = "$key" ]; then echo "${kv#*=}"; return; fi
  done
  echo "$default"
}

link_class() { echo "${1%%,*}"; }

# Fails on a spec run.sh can't build.
link_check() {
  local spec=$1 class kv kvs
  class=$(link_class "$spec")
  case $class in
    tbf|cable|dsl|fiber|trace) ;;
    *) echo "link $spec: the class is tbf, cable, dsl, fiber or trace" >&2; return 1 ;;
  esac
  IFS=, read -r -a kvs <<< "$spec"
  for kv in "${kvs[@]:1}"; do
    case ${kv%%=*} in
      slot|router|pct) ;;
      burst|peak|aqm) [ "$class" = cable ] || { echo "link $spec: ${kv%%=*} is cable's" >&2; return 1; } ;;
      line) [ "$class" = dsl ] || { echo "link $spec: line is dsl's" >&2; return 1; } ;;
      file|format|scale|offset|delay|limit) [ "$class" = trace ] || { echo "link $spec: ${kv%%=*} is trace's" >&2; return 1; } ;;
      *) echo "link $spec: unknown key ${kv%%=*}" >&2; return 1 ;;
    esac
  done
  case $(link_opt "$spec" router "") in ''|fq_codel|cake) ;; *) echo "link $spec: router is fq_codel or cake" >&2; return 1 ;; esac
  case $(link_opt "$spec" aqm pie) in pie|fifo) ;; *) echo "link $spec: aqm is pie or fifo" >&2; return 1 ;; esac
  case $(link_opt "$spec" line adsl) in adsl|vdsl) ;; *) echo "link $spec: line is adsl or vdsl" >&2; return 1 ;; esac
  if [ "$class" = trace ]; then
    [ -n "$(link_opt "$spec" file "")" ] || { echo "link $spec: trace needs file=" >&2; return 1; }
    [ -z "$(link_opt "$spec" router "")" ] || { echo "link $spec: a trace takes no router" >&2; return 1; }
  fi
}

# tc's size table: what the link charges past skb->len (which holds the
# 14-byte Ethernet header), a byte at a time (tsize 2048 over mtu 2048: the
# default table rounds a size up to 4 bytes, 1518 to 1520).
link_stab() {
  case $(link_class "$1") in
    cable) echo "stab overhead 4 mpu 64 mtu 2048 tsize 2048 " ;;
    dsl) if [ "$(link_opt "$1" line adsl)" = vdsl ]; then echo "stab overhead 16 mtu 2048 tsize 2048 "; else echo "stab linklayer atm overhead 26 mtu 2048 tsize 2048 "; fi ;;
    fiber) echo "stab overhead 4 mtu 2048 tsize 2048 " ;;
    *) echo "" ;;
  esac
}

# cake's overhead keywords for the class: cake takes them over the IP
# packet, so no size table goes with them.
link_cake_overhead() {
  case $(link_class "$1") in
    cable) echo "docsis" ;;
    dsl) if [ "$(link_opt "$1" line adsl)" = vdsl ]; then echo "pppoe-ptm"; else echo "pppoe-llcsnap"; fi ;;
    fiber) echo "overhead 18" ;;
    *) echo "overhead 14" ;;
  esac
}

link_slot() {
  local s
  s=$(link_opt "$1" slot "$([ "$(link_class "$1")" = cable ] && echo 2-8)")
  if [ -n "$s" ] && [ "$s" != off ]; then echo " slot ${s%-*}ms ${s#*-}ms"; fi
}

# The rate a class's tbf runs at for UP_MBIT, in kbit/s.
link_rate() {
  if [ "$(link_class "$1")" = dsl ] && [ "$(link_opt "$1" line adsl)" = vdsl ]; then
    awk -v u="$2" 'BEGIN { printf "%.0f", u * 1000 * 64 / 65 }'
  else
    awk -v u="$2" 'BEGIN { printf "%.0f", u * 1000 }'
  fi
}

# The shaper's qdisc (no handle): tbf, or the router's.
link_shaper() {
  local spec=$1 up=$2 burst=$3 limit=$4 peak=$5 base=${6:-$2} class router pct rate
  class=$(link_class "$spec")
  router=$(link_opt "$spec" router "")
  if [ -n "$router" ]; then
    pct=$(link_opt "$spec" pct 95)
    rate=$(awk -v u="$(link_rate "$spec" "$up")" -v p="$pct" 'BEGIN { printf "%.0f", u * p / 100 }')
    if [ "$router" = cake ]; then
      echo "cake bandwidth ${rate}kbit $(link_cake_overhead "$spec")"
    else
      # sqm-scripts' HTB takes a millisecond of the rate, at least 1749
      # bytes, as its burst.
      local b=$(( rate / 8 > 1749 ? rate / 8 : 1749 ))
      echo "tbf rate ${rate}kbit burst $b limit $limit"
    fi
    return
  fi
  case $class in
    cable)
      burst=$(link_opt "$spec" burst "$(awk -v u="$base" 'BEGIN { printf "%.0f", u * 125000 / 8 }')")
      peak=$(link_opt "$spec" peak 100)
      if awk -v p="$peak" -v u="$up" 'BEGIN { exit !(p <= u) }'; then peak=""; fi ;;
    dsl) burst=3500; peak="" ;;
  esac
  echo "tbf rate $(link_rate "$spec" "$up")kbit burst $burst limit $limit${peak:+ peakrate ${peak}mbit mtu 3028}"
}

link_root() {
  local spec=$1 up=$2 burst=$3 limit=$4 peak=$5 origin=$6 node=$7 class
  class=$(link_class "$spec")
  if [ "$class" = trace ]; then
    local file fmt scale offset dspec tlimit args
    file=$(link_opt "$spec" file "")
    fmt=$(link_opt "$spec" format mm)
    scale=$(link_opt "$spec" scale 1)
    offset=$(link_opt "$spec" offset 0)
    dspec=$(link_opt "$spec" delay "")
    tlimit=$(link_opt "$spec" limit "$limit")
    echo "tc qdisc replace dev eth0 root handle 10: prio bands 2 priomap 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0"
    echo "tc qdisc add dev eth0 parent 10:1 handle 20: fq nopacing limit 100000 flow_limit 100000 horizon 100s"
    args="--dev eth0 --parent 10: --trace /links/$(basename "$file") --format $fmt --scale $scale --offset-s $offset --start-ms $origin --limit $tlimit"
    if [ -n "$dspec" ]; then
      args="$args --delay /links/$(basename "${dspec%@*}") --delay-step-ms ${dspec##*@}"
    fi
    echo "( exec /tracelink $args --log /out/link-$node.log 2> /out/link-$node.err ) &"
    echo "for _ in \$(seq 100); do grep -q '^# base' /out/link-$node.log 2>/dev/null && break; sleep 0.05; done"
    echo "grep -q '^# base' /out/link-$node.log"
    return
  fi
  local stab
  stab=$(link_stab "$spec")
  if [ "$(link_opt "$spec" router "")" = cake ]; then stab=""; fi
  echo "tc qdisc replace dev eth0 root handle 10: $stab$(link_shaper "$spec" "$up" "$burst" "$limit" "$peak" "$up")"
  if [ -z "$(link_opt "$spec" router "")" ]; then
    case $class in
      cable) [ "$(link_opt "$spec" aqm pie)" = pie ] && echo "tc qdisc add dev eth0 parent 10:1 handle 20: pie limit 10000 target 10ms tupdate 16ms" ;;
    esac
  elif [ "$(link_opt "$spec" router "")" = fq_codel ]; then
    echo "tc qdisc add dev eth0 parent 10:1 handle 20: fq_codel"
  fi
  return 0
}

link_change() {
  local spec=$1 up=$2 burst=$3 limit=$4 peak=$5 base=$6
  if [ "$(link_class "$spec")" = trace ]; then
    echo "echo 'a trace link takes no steps' >&2; false"
    return
  fi
  local stab
  stab=$(link_stab "$spec")
  if [ "$(link_opt "$spec" router "")" = cake ]; then stab=""; fi
  echo "tc qdisc change dev eth0 root handle 10: $stab$(link_shaper "$spec" "$up" "$burst" "$limit" "$peak" "$base")"
}

# A node's link as run.sh builds it on the node's eth0, a command a line:
#
#   node_tree SPEC UP BURST LIMIT PEAK ORIGIN NODE DELAY PCAP LOSS_MATCH LOSS_EVERY SLOW QUEUE
#
# eth0's offloads off, the class's modem (link_root with SPEC to NODE),
# and the delay's netem (DELAY ms, the class's slots) on an ifb behind
# eth0's clsact redirect; with PCAP 1, a mirror of what the node offers to
# an ifb (ifbo) ahead of it; with LOSS_EVERY, the ingress filter that drops
# one packet in LOSS_EVERY of those LOSS_MATCH (a tc match) takes; with
# SLOW (kbit/s), the downlink's tbf on an ifb behind an ingress redirect,
# QUEUE ms of it. linkcheck.sh builds each class's on a farm host.
node_tree() {
  local spec=$1 up=$2 burst=$3 limit=$4 peak=$5 origin=$6 node=$7 delay=$8
  local pcap=$9 loss_match=${10} loss_every=${11} slow=${12} queue=${13} offered=""
  if [ "$pcap" = 1 ]; then
    echo "ip link add ifbo type ifb && ip link set ifbo up"
    offered="action mirred egress mirror dev ifbo "
  fi
  echo "ethtool -K eth0 tso off gso off gro off >/dev/null 2>&1 || true"
  link_root "$spec" "$up" "$burst" "$limit" "$peak" "$origin" "$node"
  echo "ip link add ifbd type ifb && ip link set ifbd up"
  echo "ethtool -K ifbd tso off gso off >/dev/null 2>&1 || true"
  echo "tc qdisc add dev ifbd root netem delay ${delay}ms$(link_slot "$spec") limit 100000"
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

link_files() {
  local spec=$1 f d
  [ "$(link_class "$spec")" = trace ] || return 0
  f=$(link_opt "$spec" file "")
  [ -z "$f" ] || echo "$f"
  d=$(link_opt "$spec" delay "")
  [ -z "$d" ] || echo "${d%@*}"
}

# Every run's: the delay's netem on an ifb behind eth0's clsact and its
# matchall redirect, tbf, and SEED_LOSS_PCT's u32 match and gact. The
# classes': cable's PIE, the routers' fq_codel and cake, and the trace's
# prio, fq and tracelink's cls_bpf classifier.
link_modules() {
  echo "sch_netem sch_tbf ifb sch_ingress cls_matchall act_mirred cls_u32 act_gact" \
    "sch_pie sch_fq_codel sch_cake sch_prio sch_fq cls_bpf"
}
