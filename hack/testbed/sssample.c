// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
//
// sssample: a container's established TCP sockets at a fixed interval,
// from one persistent process: no fork a sample, where run.sh's loop of
// `ss -tin state established` forks ss each time (about 0.025 of a core
// at 1 Hz in the testbed's image, on the seed, whose uplink and timers the
// meter work studies).
//
//   sssample [-i <interval ms>] [-r <samples>]
//
// Each interval (1000 ms by default) it reads the IPv4 and IPv6 TCP
// sockets in state established over sock_diag (inet_diag, with each
// one's tcp_info and congestion control) and writes them as run.sh's
// ss-<i>.log holds `ss -tin state established`'s: a sample's `@ <unix
// ms>` line, ss's header line, then a socket's line (Recv-Q, Send-Q,
// local and peer address:port) and its info line (a tab, then ss's
// `-i` fields strims-sim's testbed module reads: the congestion control,
// rto, rtt/rttvar, cwnd, ssthresh, bytes_sent, bytes_acked,
// bytes_received, segs_out, segs_in, data_segs_out, data_segs_in,
// pacing_rate and delivery_rate in bits a second, retrans, notsent,
// minrtt; times in ms, as ss prints them). A dump of every socket walks
// the kernel's whole established hash, every network namespace's (about
// 10 ms of a core a family on the dev VM, which ss's dump pays too), so
// it dumps every `-r` samples (5 by default) and between asks for each
// socket the last dump found by its address and cookie, a hash lookup: a
// socket opened since shows from the next dump on, one closed goes. The
// stamp is tcsample's: the
// wall clock read once at the start, advanced by the monotonic clock as
// each dump is asked for. A sample is written whole, at once. Build it
// static (gcc -O2 -static), as run.sh builds tracelink, and mount it in.

#define _GNU_SOURCE
#include <arpa/inet.h>
#include <errno.h>
#include <linux/inet_diag.h>
#include <linux/netlink.h>
#include <linux/rtnetlink.h>
#include <linux/sock_diag.h>
#include <linux/tcp.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

#ifndef TCP_ESTABLISHED
#define TCP_ESTABLISHED 1
#endif

// A growing sample.
struct buf {
    char *p;
    size_t len, cap;
};

static void put(struct buf *b, const char *fmt, ...) {
    for (;;) {
        va_list ap;
        va_start(ap, fmt);
        int n = vsnprintf(b->p + b->len, b->cap - b->len, fmt, ap);
        va_end(ap);
        if (n < 0)
            return;
        if ((size_t)n < b->cap - b->len) {
            b->len += (size_t)n;
            return;
        }
        b->cap = (b->cap + (size_t)n + 1) * 2;
        b->p = realloc(b->p, b->cap);
        if (!b->p) {
            perror("sssample: realloc");
            exit(1);
        }
    }
}

// An address and port as ss prints them: `10.77.0.10:4000`, `[::1]:80`.
static void addr(struct buf *b, int family, const uint32_t *a, uint16_t port) {
    char s[INET6_ADDRSTRLEN] = "";
    inet_ntop(family, a, s, sizeof s);
    if (family == AF_INET6)
        put(b, "[%s]:%u", s, ntohs(port));
    else
        put(b, "%s:%u", s, ntohs(port));
}

// The sockets the last dump found, to ask for one by one between dumps.
struct known {
    uint8_t family;
    struct inet_diag_sockid id;
};
static struct known *known;
static size_t nknown, capknown;

static void remember(uint8_t family, const struct inet_diag_sockid *id) {
    if (nknown == capknown) {
        capknown = capknown ? capknown * 2 : 64;
        known = realloc(known, capknown * sizeof *known);
        if (!known) {
            perror("sssample: realloc");
            exit(1);
        }
    }
    known[nknown].family = family;
    known[nknown].id = *id;
    nknown++;
}

// One socket: its line and its info line.
static void sock(struct buf *b, struct nlmsghdr *h) {
    struct inet_diag_msg *m = NLMSG_DATA(h);
    int len = (int)h->nlmsg_len - (int)NLMSG_LENGTH(sizeof *m);
    if (len < 0)
        return;
    struct tcp_info ti;
    memset(&ti, 0, sizeof ti);
    int have_info = 0;
    const char *cong = NULL;
    for (struct rtattr *a = (struct rtattr *)(m + 1); RTA_OK(a, len); a = RTA_NEXT(a, len)) {
        if (a->rta_type == INET_DIAG_INFO) {
            size_t n = (size_t)RTA_PAYLOAD(a) < sizeof ti ? (size_t)RTA_PAYLOAD(a) : sizeof ti;
            memcpy(&ti, RTA_DATA(a), n);
            have_info = 1;
        } else if (a->rta_type == INET_DIAG_CONG) {
            cong = (const char *)RTA_DATA(a);
        }
    }
    put(b, "%-6u %-6u ", m->idiag_rqueue, m->idiag_wqueue);
    addr(b, m->idiag_family, m->id.idiag_src, m->id.idiag_sport);
    put(b, " ");
    addr(b, m->idiag_family, m->id.idiag_dst, m->id.idiag_dport);
    put(b, "\n\t");
    if (cong)
        put(b, " %s", cong);
    if (have_info) {
        if (ti.tcpi_rto)
            put(b, " rto:%g", ti.tcpi_rto / 1000.0);
        if (ti.tcpi_rtt)
            put(b, " rtt:%g/%g", ti.tcpi_rtt / 1000.0, ti.tcpi_rttvar / 1000.0);
        put(b, " mss:%u cwnd:%u", ti.tcpi_snd_mss, ti.tcpi_snd_cwnd);
        if (ti.tcpi_snd_ssthresh < 0x7fffffff)
            put(b, " ssthresh:%u", ti.tcpi_snd_ssthresh);
        if (ti.tcpi_bytes_sent)
            put(b, " bytes_sent:%llu", (unsigned long long)ti.tcpi_bytes_sent);
        if (ti.tcpi_bytes_retrans)
            put(b, " bytes_retrans:%llu", (unsigned long long)ti.tcpi_bytes_retrans);
        if (ti.tcpi_bytes_acked)
            put(b, " bytes_acked:%llu", (unsigned long long)ti.tcpi_bytes_acked);
        if (ti.tcpi_bytes_received)
            put(b, " bytes_received:%llu", (unsigned long long)ti.tcpi_bytes_received);
        if (ti.tcpi_segs_out)
            put(b, " segs_out:%u", ti.tcpi_segs_out);
        if (ti.tcpi_segs_in)
            put(b, " segs_in:%u", ti.tcpi_segs_in);
        if (ti.tcpi_data_segs_out)
            put(b, " data_segs_out:%u", ti.tcpi_data_segs_out);
        if (ti.tcpi_data_segs_in)
            put(b, " data_segs_in:%u", ti.tcpi_data_segs_in);
        if (ti.tcpi_pacing_rate && ti.tcpi_pacing_rate != ~0ULL)
            put(b, " pacing_rate %llubps", (unsigned long long)ti.tcpi_pacing_rate * 8);
        if (ti.tcpi_delivery_rate)
            put(b, " delivery_rate %llubps", (unsigned long long)ti.tcpi_delivery_rate * 8);
        if (ti.tcpi_retrans || ti.tcpi_total_retrans)
            put(b, " retrans:%u/%u", ti.tcpi_retrans, ti.tcpi_total_retrans);
        if (ti.tcpi_notsent_bytes)
            put(b, " notsent:%u", ti.tcpi_notsent_bytes);
        if (ti.tcpi_min_rtt)
            put(b, " minrtt:%g", ti.tcpi_min_rtt / 1000.0);
    }
    put(b, "\n");
}

// One request: a family's established TCP sockets (`id` NULL, a dump,
// each one remembered) or the one socket `id` names, into `b`. 0, or 1
// for a socket that's gone; -1 on a socket error.
static int ask(int fd, uint32_t seq, int family, const struct inet_diag_sockid *id, struct buf *b) {
    struct {
        struct nlmsghdr n;
        struct inet_diag_req_v2 r;
    } req;
    memset(&req, 0, sizeof req);
    req.n.nlmsg_len = sizeof req;
    req.n.nlmsg_type = SOCK_DIAG_BY_FAMILY;
    req.n.nlmsg_flags = NLM_F_REQUEST | (id ? 0 : NLM_F_DUMP);
    req.n.nlmsg_seq = seq;
    req.r.sdiag_family = (uint8_t)family;
    req.r.sdiag_protocol = IPPROTO_TCP;
    req.r.idiag_states = 1U << TCP_ESTABLISHED;
    req.r.idiag_ext = (1U << (INET_DIAG_INFO - 1)) | (1U << (INET_DIAG_CONG - 1));
    if (id)
        req.r.id = *id;
    if (send(fd, &req, sizeof req, 0) < 0)
        return -1;
    static char rx[1 << 16];
    for (;;) {
        ssize_t n = recv(fd, rx, sizeof rx, 0);
        if (n < 0) {
            if (errno == EINTR)
                continue;
            return -1;
        }
        for (struct nlmsghdr *h = (struct nlmsghdr *)rx; NLMSG_OK(h, (size_t)n); h = NLMSG_NEXT(h, n)) {
            if (h->nlmsg_seq != seq)
                continue;
            if (h->nlmsg_type == NLMSG_DONE)
                return 0;
            if (h->nlmsg_type == NLMSG_ERROR) {
                // A family the kernel has no diag for has no sockets; a
                // socket asked for that isn't there any more is gone.
                struct nlmsgerr *e = NLMSG_DATA(h);
                if (e->error == -ENOENT)
                    return id ? 1 : 0;
                return e->error ? -1 : 0;
            }
            if (h->nlmsg_type == SOCK_DIAG_BY_FAMILY) {
                struct inet_diag_msg *m = NLMSG_DATA(h);
                // A socket asked for is in state established still.
                if (id && m->idiag_state != TCP_ESTABLISHED)
                    return 1;
                sock(b, h);
                if (!id)
                    remember((uint8_t)family, &m->id);
                else
                    return 0;
            }
        }
    }
}

static int64_t ns_of(clockid_t c) {
    struct timespec ts;
    clock_gettime(c, &ts);
    return (int64_t)ts.tv_sec * 1000000000 + ts.tv_nsec;
}

int main(int argc, char **argv) {
    int64_t every = 1000, refresh = 5;
    for (int i = 1; i < argc; i += 2) {
        if (i + 1 < argc && !strcmp(argv[i], "-i"))
            every = atoll(argv[i + 1]);
        else if (i + 1 < argc && !strcmp(argv[i], "-r"))
            refresh = atoll(argv[i + 1]);
        else
            every = 0;
    }
    if (every <= 0 || refresh <= 0) {
        fprintf(stderr, "usage: sssample [-i <interval ms>] [-r <samples>]\n");
        return 2;
    }
    int fd = socket(AF_NETLINK, SOCK_RAW | SOCK_CLOEXEC, NETLINK_SOCK_DIAG);
    struct sockaddr_nl sa = {.nl_family = AF_NETLINK};
    if (fd < 0 || bind(fd, (struct sockaddr *)&sa, sizeof sa) < 0) {
        perror("sssample: sock_diag");
        return 1;
    }
    int64_t wall0 = ns_of(CLOCK_REALTIME), mono0 = ns_of(CLOCK_MONOTONIC);
    struct buf b = {malloc(4096), 0, 4096};
    if (!b.p)
        return 1;
    struct timespec next;
    clock_gettime(CLOCK_MONOTONIC, &next);
    uint32_t seq = 0;
    for (int64_t sample = 0;; sample++) {
        int64_t mono = ns_of(CLOCK_MONOTONIC);
        b.len = 0;
        put(&b, "@ %lld\nRecv-Q Send-Q Local Address:Port Peer Address:Port\n",
            (long long)((wall0 + (mono - mono0)) / 1000000));
        int err = 0;
        if (sample % refresh == 0) {
            nknown = 0;
            err = ask(fd, ++seq, AF_INET, NULL, &b) < 0 || ask(fd, ++seq, AF_INET6, NULL, &b) < 0;
        } else {
            size_t kept = 0;
            for (size_t i = 0; i < nknown && !err; i++) {
                int r = ask(fd, ++seq, known[i].family, &known[i].id, &b);
                err = r < 0;
                if (r == 0)
                    known[kept++] = known[i];
            }
            nknown = kept;
        }
        if (err) {
            perror("sssample: sock_diag");
            return 1;
        }
        for (size_t off = 0; off < b.len;) {
            ssize_t w = write(1, b.p + off, b.len - off);
            if (w < 0) {
                if (errno == EINTR)
                    continue;
                return 1;
            }
            off += (size_t)w;
        }
        // The next sample on the interval's grid; one missed is skipped.
        next.tv_nsec += every * 1000000;
        while (next.tv_nsec >= 1000000000) {
            next.tv_nsec -= 1000000000;
            next.tv_sec++;
        }
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (now.tv_sec > next.tv_sec || (now.tv_sec == next.tv_sec && now.tv_nsec > next.tv_nsec))
            next = now;
        while (clock_nanosleep(CLOCK_MONOTONIC, TIMER_ABSTIME, &next, NULL) == EINTR) {
        }
    }
}
