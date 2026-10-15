// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Command golden records a differential trace of pkg/binmap: random
// operations on two maps, interleaved with queries and their results, for
// crates/strims-binmap/tests/golden.rs to replay against strims-binmap.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-binmap/golden > crates/strims-binmap/golden/trace.txt
//
// Format, one record per line, every number a decimal uint64:
//
//	scenario <n>                         two new maps, 0 and 1
//	<op> <map> <bin>                     op: set reset fillbefore resetbefore
//	<query> <map> <bin> = <result>       query: filledat emptyat cover
//	                                     findemptyafter findfilledafter
//	<query> <map> = <result>             query: findempty findfilled
//	                                     findlastfilled root empty filled
//	                                     dumphash (FNV-1a 64 of DebugPrintMap)
//	iter <map> <kind> <bin> <mode> <start> = <bins>
//	inter <kind0> <kind1> <bin> <mode> <start> = <bins>
//
// kind is "filled" or "empty". mode is "next" (Next from the start),
// "base" (NextBase), "after" (NextAfter(start) then Next) or "baseafter"
// (NextBaseAfter(start) then NextBase; iter only); start is 0 for the
// first two. <bins> is a comma separated list of at most iterCap values,
// or "-" if empty.
package main

import (
	"bufio"
	"fmt"
	"hash/fnv"
	"math/rand"
	"os"
	"strings"

	"github.com/MemeLabs/strims/pkg/binmap"
)

const (
	scenarios = 40
	opsPer    = 25
	iterCap   = 20
)

var (
	rng = rand.New(rand.NewSource(20260101))
	out = bufio.NewWriter(os.Stdout)
)

// randBin draws bins concentrated where maps have interesting structure,
// with a tail of large layers, far offsets and the sentinels.
func randBin() binmap.Bin {
	switch p := rng.Intn(100); {
	case p < 55:
		l := uint64(rng.Intn(5))
		return binmap.NewBin(l, uint64(rng.Intn(2048>>l)))
	case p < 80:
		l := uint64(5 + rng.Intn(5))
		return binmap.NewBin(l, uint64(rng.Intn(1+(4096>>l))))
	case p < 92:
		l := uint64(rng.Intn(13))
		return binmap.NewBin(l, uint64(rng.Intn(1+(1<<16>>l))))
	case p < 96:
		return binmap.NewBin(uint64(10+rng.Intn(40)), uint64(rng.Intn(3)))
	case p < 98:
		return binmap.All
	default:
		return binmap.None
	}
}

// randOpBin is randBin without ALL: with an ALL root, FindEmptyAfter can
// return bin 0 for any target and IntersectionIterator.Next never returns.
func randOpBin() binmap.Bin {
	for {
		if b := randBin(); !b.IsAll() {
			return b
		}
	}
}

func randKind() string {
	if rng.Intn(2) == 0 {
		return "filled"
	}
	return "empty"
}

func iterator(m *binmap.Map, kind string, b binmap.Bin) binmap.Iterator {
	if kind == "filled" {
		return m.IterateFilledAt(b)
	}
	return m.IterateEmptyAt(b)
}

func list(bins []binmap.Bin) string {
	if len(bins) == 0 {
		return "-"
	}
	s := make([]string, len(bins))
	for i, b := range bins {
		s[i] = fmt.Sprint(uint64(b))
	}
	return strings.Join(s, ",")
}

func collect(first func() bool, next func() bool, value func() binmap.Bin) []binmap.Bin {
	var bins []binmap.Bin
	for ok := first(); ok && len(bins) < iterCap; ok = next() {
		bins = append(bins, value())
	}
	return bins
}

func flag(v bool) int {
	if v {
		return 1
	}
	return 0
}

func dumpHash(m *binmap.Map) uint64 {
	h := fnv.New64a()
	h.Write([]byte(binmap.DebugPrintMap(m)))
	return h.Sum64()
}

func pointQuery(id int, m *binmap.Map) {
	b := randBin()
	var r uint64
	q := [...]string{"filledat", "emptyat", "cover", "findemptyafter", "findfilledafter"}[rng.Intn(5)]
	switch q {
	case "filledat":
		r = uint64(flag(m.FilledAt(b)))
	case "emptyat":
		r = uint64(flag(m.EmptyAt(b)))
	case "cover":
		r = uint64(m.Cover(b))
	case "findemptyafter":
		r = uint64(m.FindEmptyAfter(b))
	case "findfilledafter":
		r = uint64(m.FindFilledAfter(b))
	}
	fmt.Fprintf(out, "%s %d %d = %d\n", q, id, uint64(b), r)
}

func globalQueries(id int, m *binmap.Map) {
	fmt.Fprintf(out, "findempty %d = %d\n", id, uint64(m.FindEmpty()))
	fmt.Fprintf(out, "findfilled %d = %d\n", id, uint64(m.FindFilled()))
	fmt.Fprintf(out, "findlastfilled %d = %d\n", id, uint64(m.FindLastFilled()))
	fmt.Fprintf(out, "root %d = %d\n", id, uint64(m.RootBin()))
	fmt.Fprintf(out, "empty %d = %d\n", id, flag(m.Empty()))
	fmt.Fprintf(out, "filled %d = %d\n", id, flag(m.Filled()))
	fmt.Fprintf(out, "dumphash %d = %d\n", id, dumpHash(m))
}

// iterBin picks the bin to iterate: a map's root, a sub-bin or a bin larger
// than the root.
func iterBin(m *binmap.Map) binmap.Bin {
	switch rng.Intn(3) {
	case 0:
		return m.RootBin()
	case 1:
		return binmap.NewBin(uint64(4+rng.Intn(5)), uint64(rng.Intn(4)))
	default:
		return m.RootBin().Parent()
	}
}

func iterQueries(id int, m *binmap.Map) {
	for _, mode := range []string{"next", "base", "after", "baseafter"} {
		kind := randKind()
		b := iterBin(m)
		var start binmap.Bin
		it := iterator(m, kind, b)
		var bins []binmap.Bin
		switch mode {
		case "next":
			bins = collect(it.Next, it.Next, it.Value)
		case "base":
			bins = collect(it.NextBase, it.NextBase, it.Value)
		case "after":
			start = binmap.NewBin(uint64(rng.Intn(3)), uint64(rng.Intn(512)))
			bins = collect(func() bool { return it.NextAfter(start) }, it.Next, it.Value)
		case "baseafter":
			start = binmap.NewBin(0, uint64(rng.Intn(1024)))
			bins = collect(func() bool { return it.NextBaseAfter(start) }, it.NextBase, it.Value)
		}
		fmt.Fprintf(out, "iter %d %s %d %s %d = %s\n", id, kind, uint64(b), mode, uint64(start), list(bins))
	}
}

func interQueries(m0, m1 *binmap.Map) {
	for _, mode := range []string{"next", "next", "base", "after"} {
		k0, k1 := randKind(), randKind()
		b := iterBin([]*binmap.Map{m0, m1}[rng.Intn(2)])
		it := binmap.NewIntersectionIterator(iterator(m0, k0, b), iterator(m1, k1, b))
		var start binmap.Bin
		var bins []binmap.Bin
		switch mode {
		case "next":
			bins = collect(it.Next, it.Next, it.Value)
		case "base":
			bins = collect(it.NextBase, it.NextBase, it.Value)
		case "after":
			start = binmap.NewBin(0, uint64(rng.Intn(1024)))
			bins = collect(func() bool { return it.NextAfter(start) }, it.Next, it.Value)
		}
		fmt.Fprintf(out, "inter %s %s %d %s %d = %s\n", k0, k1, uint64(b), mode, uint64(start), list(bins))
	}
}

func main() {
	defer out.Flush()

	for s := 0; s < scenarios; s++ {
		fmt.Fprintf(out, "scenario %d\n", s)
		maps := [2]*binmap.Map{binmap.New(), binmap.New()}

		for i := 0; i < opsPer; i++ {
			id := rng.Intn(2)
			m := maps[id]
			bins := []binmap.Bin{randOpBin()}
			// Half the time, repeat the op 32, 64, 128 or 256 bases to the
			// right: identical sibling cells get packed into bitmaps repeated
			// across a high layer half, which random ops almost never reach.
			if b, shift := bins[0], uint64(32<<rng.Intn(4)); rng.Intn(2) == 0 && b.BaseLength() <= shift {
				bins = append(bins, binmap.NewBin(b.Layer(), b.LayerOffset()+shift>>b.Layer()))
			}
			// Mostly sets, so maps accumulate structure.
			p := rng.Intn(100)
			for _, b := range bins {
				switch {
				case p < 50:
					fmt.Fprintf(out, "set %d %d\n", id, uint64(b))
					m.Set(b)
				case p < 80:
					fmt.Fprintf(out, "reset %d %d\n", id, uint64(b))
					m.Reset(b)
				case p < 90:
					fmt.Fprintf(out, "fillbefore %d %d\n", id, uint64(b))
					m.FillBefore(b)
				default:
					fmt.Fprintf(out, "resetbefore %d %d\n", id, uint64(b))
					m.ResetBefore(b)
				}
			}
			pointQuery(id, m)
			pointQuery(id, m)
		}

		for id, m := range maps {
			globalQueries(id, m)
			iterQueries(id, m)
		}
		interQueries(maps[0], maps[1])
	}
}
