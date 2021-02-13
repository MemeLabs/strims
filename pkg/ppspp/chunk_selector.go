package ppspp

import (
	"math"
	"math/rand"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// TestChunkSelector ...
type TestChunkSelector struct{}

var newAvailableBinCount = promauto.NewHistogram(prometheus.HistogramOpts{
	Name:    "strims_ppspp_scheduler_new_available_bins",
	Help:    "The number of new bins available to chunk selector",
	Buckets: []float64{0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, math.Inf(1)},
})

// SelectBins ...
func (r *TestChunkSelector) SelectBins(count int, seen, requested, available *binmap.Map) (bins []binmap.Bin, n int) {
	start := requested.FindEmpty().BaseLeft()
	if start.IsNone() {
		start = requested.RootBin().BaseRight() + 2
	}

	start = available.FindFilledAfter(start)
	if start.IsNone() {
		return
	}

	end := seen.FindLastFilled().BaseRight()
	if start >= end {
		newAvailableBinCount.Observe(0)
		return
	}
	newAvailableBinCount.Observe(float64(end-start) / 2)

	var rc = uint64(count)

	pmax := 1.0
	var d float64
	bn := float64(end-start) / 2
	if bn > 8 {
		d = pmax / bn
	}

	binm := binmap.New()
	aend := available.FindLastFilled().BaseRight()
	for rc > 0 {
		found := 0
		for b, p := start, pmax; b < aend; b += 2 {
			if !requested.FilledAt(b) && available.FilledAt(b) {
				found++
				if rand.Float64() < p {
					requested.Set(b)
					binm.Set(b)
					rc--
				}
				p -= d
			}
		}
		if found == 0 {
			break
		}
	}

	for b := binm.FindFilled(); !b.IsNone(); b = binm.FindFilledAfter(b.BaseRight() + 2) {

		bins = append(bins, b)
	}

	n = count - int(rc)
	return
}

// LiveChunkSelector ...
type LiveChunkSelector = Test2ChunkSelector

// Test2ChunkSelector ...
type Test2ChunkSelector struct{}

// SelectBins ...
func (r *Test2ChunkSelector) SelectBins(count int, seen, requested, available *binmap.Map) (bins []binmap.Bin, n int) {
	var rc = uint64(count)
	var ab, bb binmap.Bin

	for rc > 0 {
		if requested.Filled() {
			ab = requested.RootBin().BaseRight() + 2
		} else {
			ab = requested.FindEmptyAfter(ab)
		}

		if !available.RootBin().Contains(ab) {
			break
		}

		bb = available.FindFilledAfter(ab)
		if bb.IsNone() {
			break
		}

		ab = requested.Cover(ab)
		bb = available.Cover(bb)

		if ab.Contains(bb) {
			ab = bb
		} else if !bb.Contains(ab) {
			ab = bb.BaseLeft()
			continue
		}

		for ab.BaseLength() > rc {
			ab = ab.Left()
		}
		rc -= ab.BaseLength()

		bins = append(bins, ab)
		requested.Set(ab)

		ab = ab.BaseRight() + binmap.Bin((rand.Intn(16)+1)*2)
	}

	n = count - int(rc)
	return
}

// Test3ChunkSelector ...
type Test3ChunkSelector struct {
	seed uint64
}

// SelectBins ...
func (r *Test3ChunkSelector) SelectBins(count int, have, seen, requested, available *binmap.Map) (bins []binmap.Bin, n int) {
	first := have.FindEmpty()
	if have.Filled() {
		first = have.RootBin().BaseRight() + 2
	}
	last := seen.FindLastFilled()

	if last < first {
		return
	}

	d := last - first
	_ = d

	// mid := (first + d/4) &^ 1
	mid := first + 8

	var rc = uint64(count)
	var ab, bb binmap.Bin

	// var k int
	// print := func(fmt string, v ...interface{}) {
	// 	if k++; k > 5000 {
	// 		log.Printf(fmt, v...)
	// 	}
	// }

	for rc > 0 {
		if requested.Filled() {
			ab = requested.RootBin().BaseRight() + 2
		} else {
			ab = requested.FindEmptyAfter(ab)
		}

		bb = available.FindFilledAfter(ab)
		if bb.IsNone() {
			break
		}

		ab = requested.Cover(ab)
		bb = available.Cover(bb)

		if ab.Contains(bb) {
			ab = bb
		} else if !bb.Contains(ab) {
			ab = bb.BaseLeft()
			continue
		}

		min := ab.BaseLeft()
		if min < mid {
			for ab.BaseRight() > mid || ab.BaseLength() > rc {
				ab = ab.Left()
			}

			rc -= ab.BaseLength()
			bins = append(bins, ab)
			requested.Set(ab)
		} else {
			max := ab.BaseRight()
			for b := min; b <= max && rc > 0; b += 2 {
				if r.seed&(1<<((b>>1)&63)) == 0 {
					rc--
					bins = append(bins, b)
					requested.Set(b)
				}
			}
		}

		ab = ab.BaseRight() + 2
	}

	n = count - int(rc)
	return
}

// SequentialChunkSelector ...
type SequentialChunkSelector struct{}

// SelectBins ...
func (r *SequentialChunkSelector) SelectBins(count int, seen, requested, available *binmap.Map) (bins []binmap.Bin, n int) {
	var rc = uint64(count)
	var ab, bb binmap.Bin

	for rc > 0 {
		if requested.Filled() {
			ab = requested.RootBin().BaseRight() + 2
		} else {
			ab = requested.FindEmptyAfter(ab)
		}

		if !available.RootBin().Contains(ab) {
			break
		}

		bb = available.FindFilledAfter(ab)
		if bb.IsNone() {
			break
		}

		ab = requested.Cover(ab)
		bb = available.Cover(bb)

		if ab.Contains(bb) {
			ab = bb
		} else if !bb.Contains(ab) {
			ab = bb.BaseLeft()
			continue
		}

		for ab.BaseLength() > rc {
			ab = ab.Left()
		}
		rc -= ab.BaseLength()

		bins = append(bins, ab)
		requested.Set(ab)

		ab = ab.BaseRight() + 2
	}

	n = count - int(rc)
	return
}

// FirstChunkSelector ...
type FirstChunkSelector struct{}

// SelectBins ...
func (r *FirstChunkSelector) SelectBins(count int, seen, requested, available *binmap.Map) (bins []binmap.Bin, n int) {
	// TODO: select some range of bins near the tail of the peer's available
	// set... maybe try to pick the start of the last chunkstream segment?

	end := seen.FindLastFilled()
	if end.IsNone() {
		return
	}

	bins = append(bins, end)

	requested.FillBefore(end + 2)

	// log.Println(">>>>>>>>", requested.FindLastFilled(), bins[0])

	return
}
