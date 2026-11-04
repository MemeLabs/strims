# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""A testbed run's packets (run.sh's PCAP=1): what each node offered its
egress queue (pcap-<i>-offered.pcap) against what left it (pcap-<i>.pcap),
matched by flow and the payload's first bytes (a data channel's DTLS 1.3
records encrypt their record numbers; the ciphertext is unique per packet).

    python3 hack/testbed/pcap.py <run dir> <node index>...

Per node, over 5-60 s of the stream: the packets and bytes it offered, those
its queue dropped and on how many flows, its offered peaks over 10, 50 and
200 ms, and the flows that lost most.
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
