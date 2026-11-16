// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
//
// tracelink: replays a link's delivery opportunities on a container's
// egress, for run.sh's trace-driven uplinks (cellular, satellite).
//
//   tracelink --dev eth0 --parent 10: --trace FILE [--format mm|us]
//             [--opp-bytes N] [--scale X] [--limit BYTES] [--start-ms MS]
//             [--offset-s S] [--delay FILE --delay-step-ms MS]
//             [--log FILE] [--samples FILE]
//
// A trace is a list of delivery opportunities: times within a period and
// the bytes each may carry. `mm` is Mahimahi's format (Winstein et al.,
// NSDI 2013): a millisecond a line, each line one opportunity of
// --opp-bytes (default 1514, a 1500-byte IP packet's Ethernet frame), the
// period the last line's time, whose opportunities come at each period's
// time 0. `us` is `<t_us> <bytes>` a line, the
// period the last line's time unless a `# period_us <p>` line gives it.
// The trace repeats from its start every period. --scale multiplies every
// opportunity's bytes.
//
// It loads a cls_bpf classifier, assembled here, into the classful qdisc
// --parent (a prio whose band 1 is an fq with `nopacing`) and stays in the
// foreground, writing a line every 100 ms to --log (default stdout): ms
// since the epoch, the bytes queued, packets and bytes passed, packets
// dropped, and the most bytes it held. With --samples it writes a line a
// packet there: when it reached the classifier, its departure (0 if
// dropped) and its bytes, in ns since the epoch.
//
// The program keeps a byte cursor, the capacity its packets have taken. A
// packet of `len` bytes (the qdisc's length: a GSO buffer counts each
// segment's headers) at `now`:
//
//   - capacity that passed unused while nothing waited is lost: the cursor
//     moves up to the bytes of every opportunity before `now`;
//   - the bytes between those and the cursor are what the link holds; a
//     packet that would take them past --limit bytes is dropped, as a
//     byte FIFO's tail;
//   - otherwise the cursor takes `len` more, and the packet's departure
//     (skb->tstamp, which fq holds it to) is the time of the opportunity
//     where the cursor now ends, never earlier than the previous packet's,
//     plus the --delay trace's delay at that time, if given (`<ms>` a
//     line, one every --delay-step-ms, repeating).
//
// This is Mahimahi's link queue (mm-link) at a byte's resolution: an
// opportunity's bytes serve the packets that wait when it comes, a packet
// longer than what is left of one finishes in the next, and none carry
// over idle time. A GSO buffer leaves whole at its last byte's
// opportunity, so a device whose packets should each take their own sets
// `gso_max_segs 1`. Departures are fq's, to its timer's slack (10 us).
//
// The classifier sits under a netem at the root so that netem's delay
// orphans each packet's socket memory first: TCP Small Queues then lets a
// sender queue as much as the link holds, as at a modem. Its enqueue runs
// under the root qdisc's lock, so the program's state needs no atomics.
// --start-ms puts the trace's time 0 (or --offset-s into it) at that time
// since the epoch, repeating it back to before the program loads.
//
// Build: gcc -O2 -static -o target/testbed/tracelink hack/testbed/tracelink.c.
// Needs CAP_BPF, CAP_PERFMON and CAP_NET_ADMIN.

#define _GNU_SOURCE
#include <errno.h>
#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/netlink.h>
#include <linux/pkt_cls.h>
#include <linux/rtnetlink.h>
#include <arpa/inet.h>
#include <net/if.h>
#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define MAX_INSNS 4096
#define MAX_LABELS 256
#define SEARCH_STEPS 26 // a trace of up to 2^26 opportunities
#define SAMPLES 65536   // the samples ring, a power of 2

// The state map's single value: u64 fields at these byte offsets.
enum {
  S_BASE = 0,     // monotonic ns of the trace's time 0
  S_PERIOD = 8,   // ns
  S_TOTAL = 16,   // bytes a period
  S_N = 24,       // opportunities
  S_LIMIT = 32,   // bytes the link holds at most
  S_CURSOR = 40,  // bytes of capacity taken, since the base
  S_PKTS = 48,
  S_BYTES = 56,
  S_DROPS = 64,
  S_MAXQ = 72,
  S_LAST = 80,    // the last departure
  S_DSTEP = 88,   // the delay trace's step, ns (0: none)
  S_DN = 96,      // its entries
  S_SEQ = 104,    // packets classified, the samples' next slot
  S_SIZE = 112,
};

// A trace entry: the opportunity's time in its period (ns), and the
// bytes of every opportunity up to it in the period.
struct opp {
  uint64_t t;
  uint64_t cum;
};

static struct bpf_insn prog[MAX_INSNS];
static int np;
static int label_at[MAX_LABELS];
static int nlabels;
static struct {
  int insn;
  int label;
} fixes[MAX_INSNS];
static int nfixes;

static void die(const char *what) {
  fprintf(stderr, "tracelink: %s: %s\n", what, strerror(errno));
  exit(1);
}

static void fail(const char *what) {
  fprintf(stderr, "tracelink: %s\n", what);
  exit(1);
}

static void emit(uint8_t code, uint8_t dst, uint8_t src, int16_t off, int32_t imm) {
  if (np >= MAX_INSNS) fail("program too long");
  prog[np].code = code;
  prog[np].dst_reg = dst;
  prog[np].src_reg = src;
  prog[np].off = off;
  prog[np].imm = imm;
  np++;
}

static int label(void) {
  if (nlabels >= MAX_LABELS) fail("too many labels");
  label_at[nlabels] = -1;
  return nlabels++;
}

static void mark(int l) { label_at[l] = np; }

static void jump(uint8_t code, uint8_t dst, uint8_t src, int32_t imm, int l) {
  fixes[nfixes].insn = np;
  fixes[nfixes].label = l;
  nfixes++;
  emit(code, dst, src, 0, imm);
}

// Instructions, as the kernel's filter.h names them.
static void mov_reg(int d, int s) { emit(BPF_ALU64 | BPF_MOV | BPF_X, d, s, 0, 0); }
static void mov_imm(int d, int32_t i) { emit(BPF_ALU64 | BPF_MOV | BPF_K, d, 0, 0, i); }
static void alu_reg(int op, int d, int s) { emit(BPF_ALU64 | op | BPF_X, d, s, 0, 0); }
static void alu_imm(int op, int d, int32_t i) { emit(BPF_ALU64 | op | BPF_K, d, 0, 0, i); }
static void ldx(int size, int d, int s, int16_t off) { emit(BPF_LDX | BPF_MEM | size, d, s, off, 0); }
static void stx(int size, int d, int s, int16_t off) { emit(BPF_STX | BPF_MEM | size, d, s, off, 0); }
static void ld_map(int d, int fd) {
  emit(BPF_LD | BPF_DW | BPF_IMM, d, BPF_PSEUDO_MAP_FD, 0, fd);
  emit(0, 0, 0, 0, 0);
}
static void call(int fn) { emit(BPF_JMP | BPF_CALL, 0, 0, 0, fn); }
static void jmp_reg(int op, int d, int s, int l) { jump(BPF_JMP | op | BPF_X, d, s, 0, l); }
static void jmp_imm(int op, int d, int32_t i, int l) { jump(BPF_JMP | op | BPF_K, d, 0, i, l); }
static void ja(int l) { jump(BPF_JMP | BPF_JA, 0, 0, 0, l); }
static void exit_insn(void) { emit(BPF_JMP | BPF_EXIT, 0, 0, 0, 0); }

// Stack slots below r10.
enum {
  K_KEY = -8,     // u32 map key
  K_TARGET = -16, // the value a search looks for
  K_M = -24,      // periods
  K_NOW = -32,
  K_LEN = -40,
  K_AVAIL = -48,
  K_MID = -56,
};

// r1 = &map[key] or 0, key in register `key` (u32); clobbers r1-r5.
static void lookup(int map_fd, int key) {
  stx(BPF_W, BPF_REG_10, key, K_KEY);
  ld_map(BPF_REG_1, map_fd);
  mov_reg(BPF_REG_2, BPF_REG_10);
  alu_imm(BPF_ADD, BPF_REG_2, K_KEY);
  call(BPF_FUNC_map_lookup_elem);
}

// r8 = the first index in [r8, r9) whose entry's u64 at `field` is at
// least the target, r9 if none: a binary search, unrolled.
static void search(int trace_fd, int field, int missing) {
  for (int k = 0; k < SEARCH_STEPS; k++) {
    int skip = label(), ge = label();
    jmp_reg(BPF_JGE, BPF_REG_8, BPF_REG_9, skip);
    mov_reg(BPF_REG_1, BPF_REG_8);
    alu_reg(BPF_ADD, BPF_REG_1, BPF_REG_9);
    alu_imm(BPF_RSH, BPF_REG_1, 1);
    stx(BPF_DW, BPF_REG_10, BPF_REG_1, K_MID);
    lookup(trace_fd, BPF_REG_1);
    jmp_imm(BPF_JEQ, BPF_REG_0, 0, missing);
    ldx(BPF_DW, BPF_REG_1, BPF_REG_0, field);
    ldx(BPF_DW, BPF_REG_2, BPF_REG_10, K_TARGET);
    ldx(BPF_DW, BPF_REG_3, BPF_REG_10, K_MID);
    jmp_reg(BPF_JGE, BPF_REG_1, BPF_REG_2, ge);
    mov_reg(BPF_REG_8, BPF_REG_3);
    alu_imm(BPF_ADD, BPF_REG_8, 1);
    ja(skip);
    mark(ge);
    mov_reg(BPF_REG_9, BPF_REG_3);
    mark(skip);
  }
}

static void incr(int field, int by_reg) {
  ldx(BPF_DW, BPF_REG_1, BPF_REG_7, field);
  if (by_reg < 0) {
    alu_imm(BPF_ADD, BPF_REG_1, 1);
  } else {
    alu_reg(BPF_ADD, BPF_REG_1, by_reg);
  }
  stx(BPF_DW, BPF_REG_7, BPF_REG_1, field);
}

// r6 the skb, r7 the state, r8 and r9 a search's bounds.
// Records the packet in the samples ring: when it came (K_NOW), its
// departure (r8, 0 for a drop) and its bytes (K_LEN).
static void sample(int samples_fd, uint32_t mask) {
  int after = label();
  ldx(BPF_DW, BPF_REG_1, BPF_REG_7, S_SEQ);
  mov_reg(BPF_REG_2, BPF_REG_1);
  alu_imm(BPF_ADD, BPF_REG_2, 1);
  stx(BPF_DW, BPF_REG_7, BPF_REG_2, S_SEQ);
  alu_imm(BPF_AND, BPF_REG_1, (int32_t)mask);
  lookup(samples_fd, BPF_REG_1);
  jmp_imm(BPF_JEQ, BPF_REG_0, 0, after);
  ldx(BPF_DW, BPF_REG_1, BPF_REG_10, K_NOW);
  stx(BPF_DW, BPF_REG_0, BPF_REG_1, 0);
  stx(BPF_DW, BPF_REG_0, BPF_REG_8, 8);
  ldx(BPF_DW, BPF_REG_1, BPF_REG_10, K_LEN);
  stx(BPF_DW, BPF_REG_0, BPF_REG_1, 16);
  mark(after);
}

static void assemble(int state_fd, int trace_fd, int delay_fd, int samples_fd, uint32_t mask) {
  int pass = label(), drop = label(), noprev = label(), havecur = label();
  int nomax = label(), nodelay = label(), late = label(), inorder = label();
  mov_reg(BPF_REG_6, BPF_REG_1);
  mov_imm(BPF_REG_1, 0);
  lookup(state_fd, BPF_REG_1);
  jmp_imm(BPF_JEQ, BPF_REG_0, 0, pass);
  mov_reg(BPF_REG_7, BPF_REG_0);
  call(BPF_FUNC_ktime_get_ns);
  stx(BPF_DW, BPF_REG_10, BPF_REG_0, K_NOW);
  ldx(BPF_DW, BPF_REG_1, BPF_REG_7, S_BASE);
  jmp_reg(BPF_JLT, BPF_REG_0, BPF_REG_1, pass);
  alu_reg(BPF_SUB, BPF_REG_0, BPF_REG_1);
  ldx(BPF_DW, BPF_REG_2, BPF_REG_7, S_PERIOD);
  jmp_imm(BPF_JEQ, BPF_REG_2, 0, pass);
  mov_reg(BPF_REG_3, BPF_REG_0);
  alu_reg(BPF_DIV, BPF_REG_3, BPF_REG_2);
  stx(BPF_DW, BPF_REG_10, BPF_REG_3, K_M);
  alu_reg(BPF_MUL, BPF_REG_3, BPF_REG_2);
  alu_reg(BPF_SUB, BPF_REG_0, BPF_REG_3);
  stx(BPF_DW, BPF_REG_10, BPF_REG_0, K_TARGET);
  // The opportunities before now: the first whose time is now or later.
  mov_imm(BPF_REG_8, 0);
  ldx(BPF_DW, BPF_REG_9, BPF_REG_7, S_N);
  search(trace_fd, offsetof(struct opp, t), pass);
  // What they carry, since the base.
  ldx(BPF_DW, BPF_REG_1, BPF_REG_10, K_M);
  ldx(BPF_DW, BPF_REG_2, BPF_REG_7, S_TOTAL);
  alu_reg(BPF_MUL, BPF_REG_1, BPF_REG_2);
  stx(BPF_DW, BPF_REG_10, BPF_REG_1, K_AVAIL);
  jmp_imm(BPF_JEQ, BPF_REG_8, 0, noprev);
  alu_imm(BPF_SUB, BPF_REG_8, 1);
  lookup(trace_fd, BPF_REG_8);
  jmp_imm(BPF_JEQ, BPF_REG_0, 0, pass);
  ldx(BPF_DW, BPF_REG_1, BPF_REG_0, offsetof(struct opp, cum));
  ldx(BPF_DW, BPF_REG_2, BPF_REG_10, K_AVAIL);
  alu_reg(BPF_ADD, BPF_REG_2, BPF_REG_1);
  stx(BPF_DW, BPF_REG_10, BPF_REG_2, K_AVAIL);
  mark(noprev);
  // The cursor, past what idle time lost.
  ldx(BPF_DW, BPF_REG_1, BPF_REG_7, S_CURSOR);
  ldx(BPF_DW, BPF_REG_2, BPF_REG_10, K_AVAIL);
  jmp_reg(BPF_JGE, BPF_REG_1, BPF_REG_2, havecur);
  mov_reg(BPF_REG_1, BPF_REG_2);
  mark(havecur);
  // What the link would hold with this packet.
  mov_reg(BPF_REG_3, BPF_REG_1);
  alu_reg(BPF_SUB, BPF_REG_3, BPF_REG_2);
  ldx(BPF_W, BPF_REG_4, BPF_REG_6, offsetof(struct __sk_buff, wire_len));
  stx(BPF_DW, BPF_REG_10, BPF_REG_4, K_LEN);
  alu_reg(BPF_ADD, BPF_REG_3, BPF_REG_4);
  ldx(BPF_DW, BPF_REG_5, BPF_REG_7, S_LIMIT);
  jmp_reg(BPF_JGT, BPF_REG_3, BPF_REG_5, drop);
  ldx(BPF_DW, BPF_REG_5, BPF_REG_7, S_MAXQ);
  jmp_reg(BPF_JLE, BPF_REG_3, BPF_REG_5, nomax);
  stx(BPF_DW, BPF_REG_7, BPF_REG_3, S_MAXQ);
  mark(nomax);
  alu_reg(BPF_ADD, BPF_REG_1, BPF_REG_4);
  stx(BPF_DW, BPF_REG_7, BPF_REG_1, S_CURSOR);
  // The opportunity where the cursor ends: periods m = (cursor - 1) /
  // total, then the first entry whose bytes reach the rest.
  ldx(BPF_DW, BPF_REG_2, BPF_REG_7, S_TOTAL);
  jmp_imm(BPF_JEQ, BPF_REG_2, 0, pass);
  mov_reg(BPF_REG_3, BPF_REG_1);
  alu_imm(BPF_SUB, BPF_REG_3, 1);
  alu_reg(BPF_DIV, BPF_REG_3, BPF_REG_2);
  stx(BPF_DW, BPF_REG_10, BPF_REG_3, K_M);
  alu_reg(BPF_MUL, BPF_REG_3, BPF_REG_2);
  alu_reg(BPF_SUB, BPF_REG_1, BPF_REG_3);
  stx(BPF_DW, BPF_REG_10, BPF_REG_1, K_TARGET);
  mov_imm(BPF_REG_8, 0);
  ldx(BPF_DW, BPF_REG_9, BPF_REG_7, S_N);
  search(trace_fd, offsetof(struct opp, cum), pass);
  lookup(trace_fd, BPF_REG_8);
  jmp_imm(BPF_JEQ, BPF_REG_0, 0, pass);
  ldx(BPF_DW, BPF_REG_8, BPF_REG_0, offsetof(struct opp, t));
  ldx(BPF_DW, BPF_REG_2, BPF_REG_10, K_M);
  ldx(BPF_DW, BPF_REG_3, BPF_REG_7, S_PERIOD);
  alu_reg(BPF_MUL, BPF_REG_2, BPF_REG_3);
  alu_reg(BPF_ADD, BPF_REG_8, BPF_REG_2);
  ldx(BPF_DW, BPF_REG_2, BPF_REG_7, S_BASE);
  alu_reg(BPF_ADD, BPF_REG_8, BPF_REG_2);
  // r8: the departure. Plus the delay trace's delay then.
  ldx(BPF_DW, BPF_REG_3, BPF_REG_7, S_DSTEP);
  jmp_imm(BPF_JEQ, BPF_REG_3, 0, nodelay);
  ldx(BPF_DW, BPF_REG_4, BPF_REG_7, S_DN);
  jmp_imm(BPF_JEQ, BPF_REG_4, 0, nodelay);
  mov_reg(BPF_REG_1, BPF_REG_8);
  alu_reg(BPF_SUB, BPF_REG_1, BPF_REG_2);
  alu_reg(BPF_DIV, BPF_REG_1, BPF_REG_3);
  alu_reg(BPF_MOD, BPF_REG_1, BPF_REG_4);
  lookup(delay_fd, BPF_REG_1);
  jmp_imm(BPF_JEQ, BPF_REG_0, 0, nodelay);
  ldx(BPF_DW, BPF_REG_1, BPF_REG_0, 0);
  alu_reg(BPF_ADD, BPF_REG_8, BPF_REG_1);
  mark(nodelay);
  ldx(BPF_DW, BPF_REG_1, BPF_REG_10, K_NOW);
  jmp_reg(BPF_JGE, BPF_REG_8, BPF_REG_1, late);
  mov_reg(BPF_REG_8, BPF_REG_1);
  mark(late);
  ldx(BPF_DW, BPF_REG_1, BPF_REG_7, S_LAST);
  jmp_reg(BPF_JGE, BPF_REG_8, BPF_REG_1, inorder);
  mov_reg(BPF_REG_8, BPF_REG_1);
  mark(inorder);
  stx(BPF_DW, BPF_REG_7, BPF_REG_8, S_LAST);
  stx(BPF_DW, BPF_REG_6, BPF_REG_8, offsetof(struct __sk_buff, tstamp));
  sample(samples_fd, mask);
  incr(S_PKTS, -1);
  ldx(BPF_DW, BPF_REG_2, BPF_REG_10, K_LEN);
  incr(S_BYTES, BPF_REG_2);
  mov_imm(BPF_REG_0, TC_ACT_OK);
  exit_insn();
  mark(drop);
  mov_imm(BPF_REG_8, 0);
  sample(samples_fd, mask);
  incr(S_DROPS, -1);
  mov_imm(BPF_REG_0, TC_ACT_SHOT);
  exit_insn();
  mark(pass);
  mov_imm(BPF_REG_0, TC_ACT_OK);
  exit_insn();
  for (int i = 0; i < nfixes; i++) {
    int at = label_at[fixes[i].label];
    if (at < 0) fail("unplaced label");
    prog[fixes[i].insn].off = (int16_t)(at - fixes[i].insn - 1);
  }
}

static long bpf(int cmd, union bpf_attr *attr) {
  return syscall(SYS_bpf, cmd, attr, sizeof(*attr));
}

static int map_create(uint32_t value_size, uint32_t entries) {
  union bpf_attr a;
  memset(&a, 0, sizeof(a));
  a.map_type = BPF_MAP_TYPE_ARRAY;
  a.key_size = 4;
  a.value_size = value_size;
  a.max_entries = entries;
  int fd = (int)bpf(BPF_MAP_CREATE, &a);
  if (fd < 0) die("map create");
  return fd;
}

static void map_set(int fd, uint32_t key, const void *value) {
  union bpf_attr a;
  memset(&a, 0, sizeof(a));
  a.map_fd = fd;
  a.key = (uint64_t)(uintptr_t)&key;
  a.value = (uint64_t)(uintptr_t)value;
  a.flags = BPF_ANY;
  if (bpf(BPF_MAP_UPDATE_ELEM, &a) < 0) die("map update");
}

static void map_get(int fd, uint32_t key, void *value) {
  union bpf_attr a;
  memset(&a, 0, sizeof(a));
  a.map_fd = fd;
  a.key = (uint64_t)(uintptr_t)&key;
  a.value = (uint64_t)(uintptr_t)value;
  if (bpf(BPF_MAP_LOOKUP_ELEM, &a) < 0) die("map lookup");
}

static char vlog[1 << 20];

static int prog_load(void) {
  union bpf_attr a;
  memset(&a, 0, sizeof(a));
  a.prog_type = BPF_PROG_TYPE_SCHED_CLS;
  a.insn_cnt = (uint32_t)np;
  a.insns = (uint64_t)(uintptr_t)prog;
  a.license = (uint64_t)(uintptr_t)"AGPL-3.0-only";
  a.log_level = 1;
  a.log_size = sizeof(vlog);
  a.log_buf = (uint64_t)(uintptr_t)vlog;
  strncpy(a.prog_name, "tracelink", sizeof(a.prog_name) - 1);
  int fd = (int)bpf(BPF_PROG_LOAD, &a);
  if (fd < 0) {
    size_t n = strlen(vlog);
    fprintf(stderr, "%s\n", n > 4000 ? vlog + n - 4000 : vlog);
    die("program load");
  }
  return fd;
}

static void addattr(struct nlmsghdr *n, size_t max, int type, const void *data, size_t len) {
  size_t at = NLMSG_ALIGN(n->nlmsg_len);
  if (at + RTA_SPACE(len) > max) fail("netlink message too long");
  struct rtattr *rta = (struct rtattr *)((char *)n + at);
  rta->rta_type = (unsigned short)type;
  rta->rta_len = (unsigned short)RTA_LENGTH(len);
  if (len) memcpy(RTA_DATA(rta), data, len);
  n->nlmsg_len = (uint32_t)(at + RTA_ALIGN(rta->rta_len));
}

// Attaches program `fd` to `dev`'s qdisc `parent` as a direct-action
// cls_bpf filter for every protocol.
static void attach(const char *dev, uint32_t parent, int fd) {
  struct {
    struct nlmsghdr n;
    struct tcmsg t;
    char buf[512];
  } req;
  memset(&req, 0, sizeof(req));
  unsigned ifindex = if_nametoindex(dev);
  if (!ifindex) die(dev);
  req.n.nlmsg_len = NLMSG_LENGTH(sizeof(struct tcmsg));
  req.n.nlmsg_type = RTM_NEWTFILTER;
  req.n.nlmsg_flags = NLM_F_REQUEST | NLM_F_CREATE | NLM_F_EXCL | NLM_F_ACK;
  req.n.nlmsg_seq = 1;
  req.t.tcm_family = AF_UNSPEC;
  req.t.tcm_ifindex = (int)ifindex;
  req.t.tcm_parent = parent;
  req.t.tcm_info = TC_H_MAKE(1u << 16, htons(ETH_P_ALL));
  addattr(&req.n, sizeof(req), TCA_KIND, "bpf", 4);
  size_t nest_at = NLMSG_ALIGN(req.n.nlmsg_len);
  addattr(&req.n, sizeof(req), TCA_OPTIONS, NULL, 0);
  uint32_t fd32 = (uint32_t)fd, flags = TCA_BPF_FLAG_ACT_DIRECT;
  addattr(&req.n, sizeof(req), TCA_BPF_FD, &fd32, 4);
  addattr(&req.n, sizeof(req), TCA_BPF_NAME, "tracelink", 10);
  addattr(&req.n, sizeof(req), TCA_BPF_FLAGS, &flags, 4);
  struct rtattr *nest = (struct rtattr *)((char *)&req.n + nest_at);
  nest->rta_len = (unsigned short)(req.n.nlmsg_len - nest_at);
  int s = socket(AF_NETLINK, SOCK_RAW | SOCK_CLOEXEC, NETLINK_ROUTE);
  if (s < 0) die("netlink socket");
  struct sockaddr_nl sa = {.nl_family = AF_NETLINK};
  if (sendto(s, &req, req.n.nlmsg_len, 0, (struct sockaddr *)&sa, sizeof(sa)) < 0) die("netlink send");
  char buf[8192];
  ssize_t got = recv(s, buf, sizeof(buf), 0);
  if (got < 0) die("netlink recv");
  struct nlmsghdr *h = (struct nlmsghdr *)buf;
  if (h->nlmsg_type == NLMSG_ERROR) {
    struct nlmsgerr *e = NLMSG_DATA(h);
    if (e->error) {
      errno = -e->error;
      die("attach filter");
    }
  }
  close(s);
}

static uint64_t clock_ns(clockid_t c) {
  struct timespec ts;
  clock_gettime(c, &ts);
  return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

static struct opp *opps;
static uint64_t nopps, cap_opps;

static void push(uint64_t t, uint64_t bytes, uint64_t *cum) {
  if (nopps && t < opps[nopps - 1].t) fail("trace times go back");
  if (nopps == cap_opps) {
    cap_opps = cap_opps ? cap_opps * 2 : 1 << 16;
    opps = realloc(opps, cap_opps * sizeof(*opps));
    if (!opps) fail("out of memory");
  }
  *cum += bytes;
  opps[nopps].t = t;
  opps[nopps].cum = *cum;
  nopps++;
}

// Loads a trace; returns its period in ns.
static uint64_t load(const char *path, int us, double opp_bytes, double scale) {
  FILE *f = fopen(path, "r");
  if (!f) die(path);
  char line[256];
  uint64_t cum = 0, period = 0, last = 0;
  double carry = 0;
  while (fgets(line, sizeof(line), f)) {
    unsigned long long p;
    if (sscanf(line, "# period_us %llu", &p) == 1) {
      period = (uint64_t)p * 1000;
      continue;
    }
    if (line[0] == '#' || line[0] == '\n') continue;
    unsigned long long t;
    double bytes = opp_bytes;
    if (us) {
      if (sscanf(line, "%llu %lf", &t, &bytes) != 2) fail("a `us` line is `<t_us> <bytes>`");
      t *= 1000;
    } else {
      if (sscanf(line, "%llu", &t) != 1) fail("an `mm` line is a millisecond");
      t *= 1000000;
    }
    if (bytes <= 0 || bytes > 1e9 || t > (1ull << 62)) fail("trace value out of range");
    // Fractions of a byte carry to the next opportunity.
    carry += bytes * scale;
    uint64_t whole = (uint64_t)carry;
    carry -= (double)whole;
    if (whole) push(t, whole, &cum);
    last = t;
  }
  fclose(f);
  if (!nopps) fail("empty trace");
  if (!period) period = last;
  if (period < opps[nopps - 1].t || period == 0) fail("period shorter than the trace");
  // An opportunity at the period's end is the next period's first, at its
  // time 0 (as strims-wan's Capacity takes it).
  uint64_t k = 0;
  while (k < nopps && opps[nopps - 1 - k].t == period) k++;
  if (k) {
    struct opp *r = malloc(nopps * sizeof(*r));
    if (!r) fail("out of memory");
    uint64_t sum = 0;
    for (uint64_t j = 0; j < nopps; j++) {
      uint64_t i = j < k ? nopps - k + j : j - k;
      uint64_t bytes = opps[i].cum - (i ? opps[i - 1].cum : 0);
      sum += bytes;
      r[j].t = j < k ? 0 : opps[i].t;
      r[j].cum = sum;
    }
    free(opps);
    opps = r;
  }
  if (nopps >= (1ull << SEARCH_STEPS)) fail("trace too long");
  return period;
}

static volatile sig_atomic_t stop;
static void on_signal(int sig) {
  (void)sig;
  stop = 1;
}

static uint64_t avail_at(uint64_t now, uint64_t base, uint64_t period) {
  if (now < base) return 0;
  uint64_t rel = now - base, m = rel / period, r = rel - m * period;
  uint64_t lo = 0, hi = nopps;
  while (lo < hi) {
    uint64_t mid = (lo + hi) / 2;
    if (opps[mid].t < r) lo = mid + 1; else hi = mid;
  }
  return m * opps[nopps - 1].cum + (lo ? opps[lo - 1].cum : 0);
}

int main(int argc, char **argv) {
  const char *dev = "eth0", *trace = NULL, *logpath = NULL, *delay_path = NULL, *samples_path = NULL;
  uint32_t parent = 0;
  int us = 0;
  double opp_bytes = 1514, scale = 1, offset_s = 0, delay_step_ms = 0;
  uint64_t limit = UINT64_MAX / 4, start_ms = 0;
  for (int i = 1; i < argc; i++) {
    const char *a = argv[i], *v = i + 1 < argc ? argv[i + 1] : NULL;
    if (!v) fail("a flag without its value");
    i++;
    if (!strcmp(a, "--dev")) dev = v;
    else if (!strcmp(a, "--parent")) {
      unsigned maj = 0, min = 0;
      if (sscanf(v, "%x:%x", &maj, &min) < 1) fail("--parent is MAJ: or MAJ:MIN in hex");
      parent = TC_H_MAKE(maj << 16, min);
    } else if (!strcmp(a, "--trace")) trace = v;
    else if (!strcmp(a, "--format")) {
      if (!strcmp(v, "us")) us = 1;
      else if (strcmp(v, "mm")) fail("--format is mm or us");
    } else if (!strcmp(a, "--opp-bytes")) opp_bytes = atof(v);
    else if (!strcmp(a, "--scale")) scale = atof(v);
    else if (!strcmp(a, "--limit")) limit = strtoull(v, NULL, 10);
    else if (!strcmp(a, "--start-ms")) start_ms = strtoull(v, NULL, 10);
    else if (!strcmp(a, "--offset-s")) offset_s = atof(v);
    else if (!strcmp(a, "--delay")) delay_path = v;
    else if (!strcmp(a, "--delay-step-ms")) delay_step_ms = atof(v);
    else if (!strcmp(a, "--log")) logpath = v;
    else if (!strcmp(a, "--samples")) samples_path = v;
    else fail("unknown flag");
  }
  if (!trace || !parent) fail("usage: tracelink --dev DEV --parent MAJ: --trace FILE ...");
  if (scale <= 0 || opp_bytes <= 0 || limit == 0) fail("--scale, --opp-bytes and --limit are positive");
  uint64_t period = load(trace, us, opp_bytes, scale);

  uint64_t *delays = NULL, ndelays = 0;
  if (delay_path) {
    if (delay_step_ms <= 0) fail("--delay needs --delay-step-ms");
    FILE *f = fopen(delay_path, "r");
    if (!f) die(delay_path);
    char line[128];
    uint64_t capd = 0;
    while (fgets(line, sizeof(line), f)) {
      double ms;
      if (line[0] == '#' || sscanf(line, "%lf", &ms) != 1) continue;
      if (ms < 0 || ms > 60000) fail("delay out of range");
      if (ndelays == capd) {
        capd = capd ? capd * 2 : 1024;
        delays = realloc(delays, capd * sizeof(*delays));
        if (!delays) fail("out of memory");
      }
      delays[ndelays++] = (uint64_t)(ms * 1e6);
    }
    fclose(f);
    if (!ndelays) fail("empty delay trace");
  }

  int state_fd = map_create(S_SIZE, 1);
  int trace_fd = map_create(sizeof(struct opp), (uint32_t)nopps);
  int delay_fd = map_create(8, ndelays ? (uint32_t)ndelays : 1);
  uint32_t nsamples = samples_path ? SAMPLES : 1;
  int samples_fd = map_create(24, nsamples);
  for (uint64_t i = 0; i < nopps; i++) map_set(trace_fd, (uint32_t)i, &opps[i]);
  for (uint64_t i = 0; i < ndelays; i++) map_set(delay_fd, (uint32_t)i, &delays[i]);

  // The base: the trace's time 0 at --start-ms (less --offset-s), moved
  // back by whole periods to before now.
  uint64_t mono = clock_ns(CLOCK_MONOTONIC), real = clock_ns(CLOCK_REALTIME);
  int64_t at = start_ms ? (int64_t)(start_ms * 1000000ull) - (int64_t)real : 0;
  int64_t base = (int64_t)mono + at - (int64_t)(offset_s * 1e9);
  while (base > (int64_t)mono) base -= (int64_t)period;
  uint64_t st[S_SIZE / 8];
  memset(st, 0, sizeof(st));
  st[S_BASE / 8] = (uint64_t)base;
  st[S_PERIOD / 8] = period;
  st[S_TOTAL / 8] = opps[nopps - 1].cum;
  st[S_N / 8] = nopps;
  st[S_LIMIT / 8] = limit;
  st[S_DSTEP / 8] = ndelays ? (uint64_t)(delay_step_ms * 1e6) : 0;
  st[S_DN / 8] = ndelays;
  map_set(state_fd, 0, st);

  assemble(state_fd, trace_fd, delay_fd, samples_fd, nsamples - 1);
  int pfd = prog_load();
  attach(dev, parent, pfd);

  FILE *log = logpath ? fopen(logpath, "w") : stdout;
  if (!log) die(logpath);
  FILE *samples = samples_path ? fopen(samples_path, "w") : NULL;
  if (samples_path && !samples) die(samples_path);
  int64_t to_unix = (int64_t)real - (int64_t)mono;
  uint64_t read_seq = 0;
  if (samples) fprintf(samples, "# unix_ns_in unix_ns_out bytes (out 0: dropped)\n");
  // The base in the realtime clock, to set departures beside captures.
  fprintf(log, "# base_unix_ns %lld period_ns %llu total_bytes %llu opportunities %llu limit %llu\n",
          (long long)((int64_t)real - ((int64_t)mono - base)), (unsigned long long)period,
          (unsigned long long)opps[nopps - 1].cum, (unsigned long long)nopps,
          (unsigned long long)limit);
  fprintf(log, "# unix_ms queued_bytes pkts bytes drops max_queued\n");
  fflush(log);
  signal(SIGTERM, on_signal);
  signal(SIGINT, on_signal);
  while (!stop) {
    map_get(state_fd, 0, st);
    uint64_t now = clock_ns(CLOCK_MONOTONIC);
    uint64_t avail = avail_at(now, (uint64_t)base, period);
    uint64_t cur = st[S_CURSOR / 8];
    fprintf(log, "%llu %llu %llu %llu %llu %llu\n", (unsigned long long)(clock_ns(CLOCK_REALTIME) / 1000000),
            (unsigned long long)(cur > avail ? cur - avail : 0), (unsigned long long)st[S_PKTS / 8],
            (unsigned long long)st[S_BYTES / 8], (unsigned long long)st[S_DROPS / 8],
            (unsigned long long)st[S_MAXQ / 8]);
    fflush(log);
    uint64_t seq = st[S_SEQ / 8];
    if (samples) {
      if (seq - read_seq > SAMPLES) {
        fprintf(samples, "# lost %llu\n", (unsigned long long)(seq - read_seq - SAMPLES));
        read_seq = seq - SAMPLES;
      }
      for (; read_seq < seq; read_seq++) {
        uint64_t e[3];
        map_get(samples_fd, (uint32_t)(read_seq & (SAMPLES - 1)), e);
        fprintf(samples, "%lld %lld %llu\n", (long long)((int64_t)e[0] + to_unix),
                e[1] ? (long long)((int64_t)e[1] + to_unix) : 0LL, (unsigned long long)e[2]);
      }
      fflush(samples);
    }
    struct timespec ts = {0, 100000000};
    nanosleep(&ts, NULL);
  }
  return 0;
}
