# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""A testbed run's packets (run.sh's PCAP=1): what each node offered its
egress queue (pcap-<i>-offered.pcap) against what left it (pcap-<i>.pcap),
matched by flow and the payload's first bytes (a data channel's DTLS 1.3
records encrypt their record numbers; the ciphertext is unique per packet).

    python3 hack/testbed/pcap.py <run dir> <node index>...
    python3 hack/testbed/pcap.py bytes <run dir> <node index>...
    python3 hack/testbed/pcap.py acks <run dir> <node index>... [--from <s>]

Per node, over 5-60 s of the stream: the packets and bytes it offered, those
its queue dropped and on how many flows, its offered peaks over 10, 50 and
200 ms, and the flows that lost most.

`bytes` says what the transport added, per node and direction over the
same span, from pcap-<i>.pcap (eth0 holds both): the packets, their frame
bytes (Ethernet's 14 included, as tbf shapes them: the sims' SCTP model
charges them, and the TCP model's packets per write come to about as much)
and payload bytes (TCP's or UDP's), TCP's pure ACKs,
the packets of SMALL bytes or less, and on the way out the frame bytes over
what the node's driver wrote (node-<i>.jsonl's bytes_written), as a sim
row's up_wire stands to what its hosts wrote. A data channel's payload is
DTLS: its SCTP framing counts as payload here.

`acks` sorts each node's TCP acknowledgements of the data it received by
why Linux sent them, the reasons the sims' rows count (`classify`, against
strims-sim's window model and a bench's `ev: drops` lines), per connection
and over them, from <s> seconds after each handshake (default 0), with the
delayed ones' waits from the first byte they acknowledge (ms). A bench run
(bench.sh) needs PCAP=1.
"""
import collections, json, os, struct, sys

def packets(path):
    """(t seconds, src, sport, dst, dport, proto, length, dtls key or None)."""
    with open(path, "rb") as f:
        hdr = f.read(24)
        magic = struct.unpack("<I", hdr[:4])[0]
        nano = magic == 0xa1b23c4d
        while True:
            rh = f.read(16)
            if len(rh) < 16: return
            ts, tu, incl, orig = struct.unpack("<IIII", rh)
            data = f.read(incl)
            t = ts + tu / (1e9 if nano else 1e6)
            if len(data) < 34 or data[12:14] != b"\x08\x00": continue
            ihl = (data[14] & 15) * 4
            proto = data[23]
            src = ".".join(map(str, data[26:30])); dst = ".".join(map(str, data[30:34]))
            l4 = 14 + ihl
            if len(data) < l4 + 8: continue
            sport, dport = struct.unpack(">HH", data[l4:l4 + 4])
            # The payload's first bytes: a record's ciphertext (DTLS 1.3
            # encrypts its record numbers), unique per packet.
            key = bytes(data[l4 + 8:l4 + 8 + 48]) if proto == 17 and len(data) >= l4 + 8 + 24 else None
            yield t, src, sport, dst, dport, proto, orig, key

# Packets of at most this many frame bytes count as small, as the sims
# count them (strims_wan::wire::SMALL_PACKET).
SMALL = 256


def headers(path):
    """(t seconds, src, dst, proto, frame bytes, payload bytes, pure ACK)."""
    with open(path, "rb") as f:
        hdr = f.read(24)
        nano = struct.unpack("<I", hdr[:4])[0] == 0xa1b23c4d
        while True:
            rh = f.read(16)
            if len(rh) < 16: return
            ts, tu, incl, orig = struct.unpack("<IIII", rh)
            data = f.read(incl)
            if len(data) < 34 or data[12:14] != b"\x08\x00": continue
            t = ts + tu / (1e9 if nano else 1e6)
            ihl = (data[14] & 15) * 4
            total = struct.unpack(">H", data[16:18])[0]
            proto = data[23]
            src = ".".join(map(str, data[26:30])); dst = ".".join(map(str, data[30:34]))
            l4 = 14 + ihl
            if proto == 6 and len(data) >= l4 + 14:
                off = (data[l4 + 12] >> 4) * 4
                payload = total - ihl - off
                flags = data[l4 + 13]
                pure = payload == 0 and flags & 0x10 and not flags & 0x07
            elif proto == 17:
                payload, pure = total - ihl - 8, False
            else:
                continue
            yield t, src, dst, proto, orig, payload, bool(pure)


def written(d, i, lo, hi):
    """What node `i`'s driver wrote from `lo` to `hi` s after its start."""
    rows = [json.loads(x) for x in open(os.path.join(d, f"node-{i}.jsonl")) if '"rel_ms"' in x]
    inside = [r for r in rows if lo <= r["rel_ms"] / 1000 <= hi and r.get("bytes_written") is not None]
    if len(inside) < 2:
        return None, None
    return inside[-1]["bytes_written"] - inside[0]["bytes_written"], (inside[-1]["rel_ms"] - inside[0]["rel_ms"]) / 1000


def bytes_(d, nodes, lo=5, hi=60):
    print(f"{'node':>4} {'dir':>3} {'pkts/s':>8} {'KB/s':>8} {'payload':>8} {'hdr%':>5} {'pure_acks':>9} {'ack%':>5}"
          f" {'small%':>6} {'l2/written':>10}")
    for i in nodes:
        ip = f"10.77.0.{10 + i}"
        r0 = next(json.loads(x) for x in open(os.path.join(d, f"node-{i}.jsonl")) if '"rel_ms"' in x)
        t0 = (r0["t_ms"] - r0["rel_ms"]) / 1000
        sides = {"out": [], "in": []}
        for p in headers(os.path.join(d, f"pcap-{i}.pcap")):
            if not lo <= p[0] - t0 < hi:
                continue
            if p[1] == ip:
                sides["out"].append(p)
            elif p[2] == ip:
                sides["in"].append(p)
        w, span = written(d, i, lo, hi)
        for side, ps in sides.items():
            if not ps:
                continue
            secs = max(ps[-1][0] - ps[0][0], 1e-3)
            total = sum(p[4] for p in ps)
            payload = sum(p[5] for p in ps)
            acks = [p for p in ps if p[6]]
            small = sum(1 for p in ps if p[4] <= SMALL)
            ratio = "-"
            if side == "out" and w:
                ratio = f"{total / secs / (w / span):10.3f}"
            print(f"{i:>4} {side:>3} {len(ps) / secs:8.0f} {total / secs / 1e3:8.1f} {payload / secs / 1e3:8.1f}"
                  f" {100 * (total - payload) / max(total, 1):5.1f} {len(acks) / secs:9.0f}"
                  f" {100 * sum(p[4] for p in acks) / max(total, 1):5.1f} {100 * small / len(ps):6.1f} {ratio:>10}")


def segments(path):
    """TCP packets: (t seconds, src, sport, dst, dport, seq, ack, flags,
    payload bytes, TCP header bytes)."""
    with open(path, "rb") as f:
        hdr = f.read(24)
        nano = struct.unpack("<I", hdr[:4])[0] == 0xa1b23c4d
        while True:
            rh = f.read(16)
            if len(rh) < 16: return
            ts, tu, incl, orig = struct.unpack("<IIII", rh)
            data = f.read(incl)
            if len(data) < 34 or data[12:14] != b"\x08\x00" or data[23] != 6: continue
            ihl = (data[14] & 15) * 4
            l4 = 14 + ihl
            if len(data) < l4 + 14: continue
            total = struct.unpack(">H", data[16:18])[0]
            sport, dport, seq, ack = struct.unpack(">HHII", data[l4:l4 + 12])
            off = (data[l4 + 12] >> 4) * 4
            src = ".".join(map(str, data[26:30])); dst = ".".join(map(str, data[30:34]))
            yield (ts + tu / (1e9 if nano else 1e6), src, sport, dst, dport, seq, ack, data[l4 + 13],
                   total - ihl - off, off)


# Linux's delayed acknowledgement constants (include/net/tcp.h, HZ 1000).
ATO_MIN, DELACK_MAX, QUICKACKS, MSS, RTO_MIN = 0.040, 0.200, 16, 1448, 0.200
FIN, SYN, PSH, ACK = 0x01, 0x02, 0x08, 0x10
WHYS = ("now", "quick", "read", "timer", "data", "other")


def classify(events, rtt):
    """Replays a Linux receiver's delayed acknowledgement state
    (tcp_event_data_recv, tcp_measure_rcv_mss, __tcp_ack_snd_check,
    tcp_send_delayed_ack, __tcp_cleanup_rbuf, tcp_event_data_sent and the
    delayed ACK timer) over one direction of a connection: the data packets
    it received ("in": t, seq, len, flags, header bytes) and every packet it
    sent ("out": t, ack, len, flags), sequence numbers relative to the
    data's first byte. Each packet sent that acknowledges new data gets a
    why (WHYS): data if it carries data, else at once when more than
    rcv_mss was unacknowledged (or out of order), quickack while the
    replayed quick count lasts outside pingpong mode, on read when it left
    within READ of a short packet the reader drained, by the timer when it
    left at least three quarters of the timer after the first byte it
    acknowledges, else other. Returns [(t, why, wait from the first byte
    it acknowledges)]."""
    rto = rtt + max(2 * rtt, RTO_MIN)
    nxt = wup = 0
    ahead = {}
    rcv_mss, lss, pushed, quick, pingpong, ato, last, since = 536, 0, 0, 0, False, 0.0, None, None
    last_in = None
    out = []
    for e in events:
        if e[0] == "in":
            _, t, seq, ln, flags, hlen = e
            if ln == 0:
                continue
            if seq != nxt:
                if seq > nxt:
                    ahead[seq] = ln
                    if since is None:
                        since = t
                continue
            # tcp_measure_rcv_mss
            l = ln
            prev, lss = lss, 0
            if l >= rcv_mss:
                rcv_mss = min(l, MSS)
                if flags & PSH:
                    pushed = max(pushed, 1)
            else:
                l += hlen
                skip = False
                if l >= 536 + 20 or (l >= 88 + 20 and not flags & (FIN | PSH)):
                    l -= hlen
                    lss = l
                    if l == prev:
                        rcv_mss = l
                        skip = True
                if not skip:
                    pushed = 2 if pushed else 1
            # tcp_event_data_recv's ato
            if ato == 0:
                quick, ato = QUICKACKS, ATO_MIN
            else:
                m = t - last
                if m <= ATO_MIN / 2:
                    ato = ato / 2 + ATO_MIN / 2
                elif m < ato:
                    ato = min(ato / 2 + m, rto)
                elif m > rto:
                    quick = max(quick, QUICKACKS)
            last = last_in = t
            if since is None:
                since = t
            nxt = seq + ln
            while nxt in ahead:
                nxt += ahead.pop(nxt)
            continue
        _, t, ack, ln, flags = e
        if not flags & ACK or flags & SYN:
            continue
        newly = ack - wup
        if newly <= 0 and not (ln == 0 and ahead):
            if ln and last is not None and t - last < ato:
                pingpong = True
            continue
        if ln:
            why = "data"
        elif newly > rcv_mss or ahead or newly <= 0:
            why = "now"
        elif quick and not pingpong:
            why = "quick"
        elif last_in is not None and t - last_in < READ and (pushed == 2 or (pushed and not pingpong)):
            why = "read"
        elif since is not None and t - since >= 0.75 * min(ato, max(rtt, ATO_MIN), DELACK_MAX):
            why = "timer"
            if pingpong:
                pingpong, ato = False, ATO_MIN
            else:
                ato = min(2 * ato, rto)
        else:
            why = "other"
        out.append((t, why, t - since if since is not None else 0.0))
        # tcp_event_ack_sent, then tcp_event_data_sent's pingpong count.
        if quick:
            if quick <= 1:
                quick, ato = 0, ATO_MIN
            else:
                quick -= 1
        pushed, since, wup = 0, None, max(wup, ack)
        if ln and last is not None and t - last < ato:
            pingpong = True
    return out


# How soon after the packet a read's acknowledgement leaves.
READ = 0.005


def acks(d, nodes, lo=0.0):
    """Each node's acknowledgements of the data it received, by why
    (classify), over its connections, from `lo` seconds after each
    connection's handshake: what it offered its egress queue
    (pcap-<i>-offered.pcap) against the data that reached it
    (pcap-<i>.pcap)."""
    print(f"{'node':>4} {'peer':>12} " + " ".join(f"{w:>6}" for w in WHYS) + f" {'timer ms mean':>13} {'p90':>5}")
    for i in nodes:
        ip = f"10.77.0.{10 + i}"
        inc = [p for p in segments(os.path.join(d, f"pcap-{i}.pcap")) if p[3] == ip]
        off = [p for p in segments(os.path.join(d, f"pcap-{i}-offered.pcap")) if p[1] == ip]
        flows = collections.defaultdict(lambda: {"in": [], "out": []})
        for p in inc:
            flows[(p[1], p[2], p[4])]["in"].append(p)
        for p in off:
            flows[(p[3], p[4], p[2])]["out"].append(p)
        total = collections.Counter()
        waits = []
        for (peer, pport, port), f in sorted(flows.items()):
            syn_in = next((p for p in f["in"] if p[7] & SYN), None)
            syn_out = next((p for p in f["out"] if p[7] & SYN), None)
            if not syn_in or not syn_out or not any(p[8] for p in f["in"]):
                continue
            isn = syn_in[5]
            rtt = abs(syn_out[0] - syn_in[0]) if syn_in[7] & ACK or syn_out[7] & ACK else 0.0
            # A server's round trip is its SYN-ACK to the handshake's ACK.
            if not syn_in[7] & ACK:
                done = next((p for p in f["in"] if p[0] > syn_out[0] and p[7] & ACK), None)
                rtt = done[0] - syn_out[0] if done else 0.0
            rel = lambda s: (s - isn - 1) % (1 << 32)
            evs = [("in", p[0], rel(p[5]), p[8], p[7], p[9]) for p in f["in"] if not p[7] & SYN]
            evs += [("out", p[0], rel(p[6]), p[8], p[7]) for p in f["out"]]
            evs.sort(key=lambda e: (e[1], e[0] == "out"))
            start = syn_in[0] + lo
            got = [x for x in classify(evs, rtt) if x[0] >= start]
            c = collections.Counter(w for _, w, _ in got)
            tw = sorted(1000 * w for _, why, w in got if why == "timer")
            waits += tw
            total += c
            print(f"{i:>4} {peer:>12} " + " ".join(f"{c[w]:>6}" for w in WHYS)
                  + f" {sum(tw) / max(len(tw), 1):13.1f} {tw[int(0.9 * (len(tw) - 1))] if tw else 0:5.0f}")
        waits.sort()
        n = max(sum(total.values()), 1)
        print(f"{i:>4} {'all':>12} " + " ".join(f"{total[w]:>6}" for w in WHYS)
              + f" {sum(waits) / max(len(waits), 1):13.1f} {waits[int(0.9 * (len(waits) - 1))] if waits else 0:5.0f}")
        print(f"{'':>4} {'share %':>12} " + " ".join(f"{100 * total[w] / n:6.1f}" for w in WHYS))


def node(d, i):
    ip = f"10.77.0.{10 + i}"
    r0 = next(json.loads(x) for x in open(os.path.join(d, f"node-{i}.jsonl")) if '"rel_ms"' in x)
    t0 = (r0["t_ms"] - r0["rel_ms"]) / 1000
    off = [p for p in packets(os.path.join(d, f"pcap-{i}-offered.pcap")) if p[1] == ip]
    out = [p for p in packets(os.path.join(d, f"pcap-{i}.pcap")) if p[1] == ip]
    return ip, t0, off, out

def drops(off, out):
    sent = {(p[3], p[4], p[2], p[7]) for p in out if p[7]}
    return [p for p in off if p[7] and (p[3], p[4], p[2], p[7]) not in sent]

if __name__ == "__main__":
    if sys.argv[1] == "bytes":
        bytes_(sys.argv[2], [int(x) for x in sys.argv[3:]])
        sys.exit()
    if sys.argv[1] == "acks":
        args = sys.argv[2:]
        lo = 0.0
        if "--from" in args:
            k = args.index("--from")
            lo = float(args[k + 1])
            del args[k:k + 2]
        acks(args[0], [int(x) for x in args[1:]], lo)
        sys.exit()
    d = sys.argv[1]; nodes = [int(x) for x in sys.argv[2:]]
    for i in nodes:
        ip, t0, off, out = node(d, i)
        lost = drops(off, out)
        w = [p for p in off if 5 <= p[0] - t0 < 60]
        lw = [p for p in lost if 5 <= p[0] - t0 < 60]
        flows = collections.Counter((p[3], p[4]) for p in w if p[7])
        lflows = collections.Counter((p[3], p[4]) for p in lw)
        # offered bytes in sliding windows
        def peak(win):
            best = 0; j = 0; s = 0
            for k, p in enumerate(w):
                s += p[6]
                while w[j][0] < p[0] - win: s -= w[j][6]; j += 1
                best = max(best, s)
            return best / win
        span = (w[-1][0] - w[0][0]) if w else 1
        print(f"node {i} ({ip}): offered {len(w)} pkts {sum(p[6] for p in w)/span/1000:.0f} KB/s, dropped {len(lw)} ({len(lw)/max(len(w),1)*100:.1f} %), flows {len(flows)}, flows with drops {len(lflows)}")
        print(f"   offered peak over 10 ms {peak(0.01)/1000:.0f} KB/s, 50 ms {peak(0.05)/1000:.0f}, 200 ms {peak(0.2)/1000:.0f}")
        big = sorted(lflows.items(), key=lambda x: -x[1])[:5]
        print("   most dropped flows:", [(f"{a}:{b}", n, flows[(a, b)]) for (a, b), n in big])
