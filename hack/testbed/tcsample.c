// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
//
// tcsample: a container's qdisc statistics at a fixed interval, from one
// persistent process: no fork a sample, where a shell loop of `tc -s -j
// qdisc show` forks tc (and tr) each time, about 0.11 of a core at 10 Hz
// in the testbed's image.
//
//   tcsample [-i <interval ms>] [<dev> ...]
//
// Each interval (100 ms by default) it dumps the qdiscs of the devices
// named (every device's without one: eth0's and the ifbs') over rtnetlink
// and writes a line, `<unix ms> <json>`, as run.sh's tc-<i>.log holds
// them: the JSON is `tc -s -j qdisc show`'s array, with the fields
// hack/testbed/tcstats.py reads (kind, handle, dev, root or parent,
// options: a tbf's rate, a cake's bandwidth, atm, overhead, mpu and raw;
// bytes, packets, drops, overlimits, requeues, backlog, qlen), rates in
// bytes a second as tc's JSON has them. The stamp is the wall clock read
// once at the start, advanced by the monotonic clock as each dump is
// asked for, so a stamp neither jumps with the wall clock nor depends on a
// date command's formatting. A line is written whole, at once. Build it
// static (gcc -O2 -static), as run.sh builds tracelink, and mount it in.

#define _GNU_SOURCE
#include <errno.h>
#include <linux/gen_stats.h>
#include <linux/netlink.h>
#include <linux/pkt_sched.h>
#include <linux/rtnetlink.h>
#include <net/if.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

// A growing line.
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
            perror("tcsample: realloc");
            exit(1);
        }
    }
}

// A handle as tc prints it: `10:`, `1:1`, `ffff:fff1`.
static void handle(struct buf *b, const char *key, uint32_t h) {
    if (TC_H_MIN(h))
        put(b, ",\"%s\":\"%x:%x\"", key, TC_H_MAJ(h) >> 16, TC_H_MIN(h));
    else
        put(b, ",\"%s\":\"%x:\"", key, TC_H_MAJ(h) >> 16);
}

// The attributes in `len` bytes from `rta`, by type, up to `max`.
static void attrs(struct rtattr **tb, int max, struct rtattr *rta, int len) {
    memset(tb, 0, sizeof(struct rtattr *) * (size_t)(max + 1));
    for (; RTA_OK(rta, len); rta = RTA_NEXT(rta, len))
        if (rta->rta_type <= max && !tb[rta->rta_type])
            tb[rta->rta_type] = rta;
}

static uint64_t u64_of(struct rtattr *a) {
    uint64_t v = 0;
    memcpy(&v, RTA_DATA(a), RTA_PAYLOAD(a) < sizeof v ? RTA_PAYLOAD(a) : sizeof v);
    return v;
}

static uint32_t u32_of(struct rtattr *a) {
    uint32_t v = 0;
    memcpy(&v, RTA_DATA(a), RTA_PAYLOAD(a) < sizeof v ? RTA_PAYLOAD(a) : sizeof v);
    return v;
}

static void options(struct buf *b, const char *kind, struct rtattr *opt) {
    if (!opt)
        return;
    if (!strcmp(kind, "tbf")) {
        struct rtattr *tb[TCA_TBF_MAX + 1];
        attrs(tb, TCA_TBF_MAX, RTA_DATA(opt), (int)RTA_PAYLOAD(opt));
        uint64_t rate = 0;
        if (tb[TCA_TBF_PARMS] && RTA_PAYLOAD(tb[TCA_TBF_PARMS]) >= sizeof(struct tc_tbf_qopt)) {
            struct tc_tbf_qopt q;
            memcpy(&q, RTA_DATA(tb[TCA_TBF_PARMS]), sizeof q);
            rate = q.rate.rate;
        }
        if (tb[TCA_TBF_RATE64])
            rate = u64_of(tb[TCA_TBF_RATE64]);
        put(b, ",\"options\":{\"rate\":%llu}", (unsigned long long)rate);
    } else if (!strcmp(kind, "cake")) {
        struct rtattr *tb[TCA_CAKE_MAX + 1];
        attrs(tb, TCA_CAKE_MAX, RTA_DATA(opt), (int)RTA_PAYLOAD(opt));
        put(b, ",\"options\":{");
        const char *sep = "";
        if (tb[TCA_CAKE_BASE_RATE64]) {
            put(b, "\"bandwidth\":%llu", (unsigned long long)u64_of(tb[TCA_CAKE_BASE_RATE64]));
            sep = ",";
        }
        if (tb[TCA_CAKE_ATM]) {
            uint32_t a = u32_of(tb[TCA_CAKE_ATM]);
            put(b, "%s\"atm\":\"%s\"", sep, a == CAKE_ATM_ATM ? "atm" : a == CAKE_ATM_PTM ? "ptm" : "noatm");
            sep = ",";
        }
        if (tb[TCA_CAKE_OVERHEAD]) {
            put(b, "%s\"overhead\":%d", sep, (int32_t)u32_of(tb[TCA_CAKE_OVERHEAD]));
            sep = ",";
        }
        if (tb[TCA_CAKE_MPU]) {
            put(b, "%s\"mpu\":%u", sep, u32_of(tb[TCA_CAKE_MPU]));
            sep = ",";
        }
        if (tb[TCA_CAKE_RAW])
            put(b, "%s\"raw\":true", sep);
        put(b, "}");
    }
}

// One RTM_NEWQDISC message as tc's JSON object; 0 if it's not one of the
// devices `devs` names.
static int qdisc(struct buf *b, struct nlmsghdr *h, char **devs, int ndevs, int first) {
    struct tcmsg *t = NLMSG_DATA(h);
    int len = (int)h->nlmsg_len - (int)NLMSG_LENGTH(sizeof *t);
    if (len < 0)
        return 0;
    char dev[IF_NAMESIZE] = "";
    if (!if_indextoname((unsigned)t->tcm_ifindex, dev))
        snprintf(dev, sizeof dev, "if%d", t->tcm_ifindex);
    if (ndevs) {
        int want = 0;
        for (int i = 0; i < ndevs; i++)
            want |= !strcmp(devs[i], dev);
        if (!want)
            return 0;
    }
    struct rtattr *tb[TCA_MAX + 1];
    attrs(tb, TCA_MAX, (struct rtattr *)((char *)t + NLMSG_ALIGN(sizeof *t)), len);
    const char *kind = tb[TCA_KIND] ? (const char *)RTA_DATA(tb[TCA_KIND]) : "";
    put(b, "%s{\"kind\":\"%s\"", first ? "" : ",", kind);
    handle(b, "handle", t->tcm_handle);
    put(b, ",\"dev\":\"%s\"", dev);
    if (t->tcm_parent == TC_H_ROOT)
        put(b, ",\"root\":true");
    else if (t->tcm_parent)
        handle(b, "parent", t->tcm_parent);
    options(b, kind, tb[TCA_OPTIONS]);
    uint64_t bytes = 0;
    uint32_t packets = 0;
    struct gnet_stats_queue q = {0};
    if (tb[TCA_STATS2]) {
        struct rtattr *st[TCA_STATS_MAX + 1];
        attrs(st, TCA_STATS_MAX, RTA_DATA(tb[TCA_STATS2]), (int)RTA_PAYLOAD(tb[TCA_STATS2]));
        if (st[TCA_STATS_BASIC] && RTA_PAYLOAD(st[TCA_STATS_BASIC]) >= 12) {
            memcpy(&bytes, RTA_DATA(st[TCA_STATS_BASIC]), 8);
            memcpy(&packets, (char *)RTA_DATA(st[TCA_STATS_BASIC]) + 8, 4);
        }
        if (st[TCA_STATS_QUEUE])
            memcpy(&q, RTA_DATA(st[TCA_STATS_QUEUE]),
                   RTA_PAYLOAD(st[TCA_STATS_QUEUE]) < sizeof q ? RTA_PAYLOAD(st[TCA_STATS_QUEUE]) : sizeof q);
    } else if (tb[TCA_STATS] && RTA_PAYLOAD(tb[TCA_STATS]) >= sizeof(struct tc_stats)) {
        struct tc_stats s;
        memcpy(&s, RTA_DATA(tb[TCA_STATS]), sizeof s);
        bytes = s.bytes;
        packets = s.packets;
        q.drops = s.drops;
        q.overlimits = s.overlimits;
        q.backlog = s.backlog;
        q.qlen = s.qlen;
    }
    put(b, ",\"bytes\":%llu,\"packets\":%u,\"drops\":%u,\"overlimits\":%u,\"requeues\":%u,\"backlog\":%u,\"qlen\":%u}",
        (unsigned long long)bytes, packets, q.drops, q.overlimits, q.requeues, q.backlog, q.qlen);
    return 1;
}

// One dump of every qdisc into `b`; -1 on a socket error.
static int dump(int fd, uint32_t seq, struct buf *b, char **devs, int ndevs) {
    struct {
        struct nlmsghdr n;
        struct tcmsg t;
    } req;
    memset(&req, 0, sizeof req);
    req.n.nlmsg_len = NLMSG_LENGTH(sizeof req.t);
    req.n.nlmsg_type = RTM_GETQDISC;
    req.n.nlmsg_flags = NLM_F_REQUEST | NLM_F_DUMP;
    req.n.nlmsg_seq = seq;
    req.t.tcm_family = AF_UNSPEC;
    if (send(fd, &req, req.n.nlmsg_len, 0) < 0)
        return -1;
    static char rx[1 << 16];
    int first = 1;
    put(b, "[");
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
            if (h->nlmsg_type == NLMSG_DONE) {
                put(b, "]");
                return 0;
            }
            if (h->nlmsg_type == NLMSG_ERROR)
                return -1;
            if (h->nlmsg_type == RTM_NEWQDISC && qdisc(b, h, devs, ndevs, first))
                first = 0;
        }
    }
}

static int64_t ns_of(clockid_t c) {
    struct timespec ts;
    clock_gettime(c, &ts);
    return (int64_t)ts.tv_sec * 1000000000 + ts.tv_nsec;
}

int main(int argc, char **argv) {
    int64_t every = 100;
    int at = 1;
    if (at + 1 < argc && !strcmp(argv[at], "-i")) {
        every = atoll(argv[at + 1]);
        at += 2;
    }
    if (every <= 0 || (at < argc && argv[at][0] == '-')) {
        fprintf(stderr, "usage: tcsample [-i <interval ms>] [<dev> ...]\n");
        return 2;
    }
    int fd = socket(AF_NETLINK, SOCK_RAW | SOCK_CLOEXEC, NETLINK_ROUTE);
    struct sockaddr_nl sa = {.nl_family = AF_NETLINK};
    if (fd < 0 || bind(fd, (struct sockaddr *)&sa, sizeof sa) < 0) {
        perror("tcsample: netlink");
        return 1;
    }
    int64_t wall0 = ns_of(CLOCK_REALTIME), mono0 = ns_of(CLOCK_MONOTONIC);
    struct buf b = {malloc(4096), 0, 4096};
    if (!b.p)
        return 1;
    struct timespec next;
    clock_gettime(CLOCK_MONOTONIC, &next);
    for (uint32_t seq = 1;; seq++) {
        int64_t mono = ns_of(CLOCK_MONOTONIC);
        b.len = 0;
        put(&b, "%lld ", (long long)((wall0 + (mono - mono0)) / 1000000));
        if (dump(fd, seq, &b, argv + at, argc - at) < 0) {
            perror("tcsample: dump");
            return 1;
        }
        put(&b, "\n");
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
