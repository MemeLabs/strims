package encoding

import (
	"log"
	"math/rand"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/davecgh/go-spew/spew"
)

func TestChunkScheduler(t *testing.T) {
	requested := binmap.New()
	available := binmap.New()

	rand.Seed(1)
	p := float64(1)
	c := 300
	var ns []int
	for i := 0; i < c; i++ {
		p -= 1 / float64(c)
		if rand.Float64() < p {
			available.Set(binmap.Bin(i) * 2)
			ns = append(ns, i*2)
		}
	}

	s := &TestChunkSelector{}
	b, n := s.SelectBins(4, available, requested, available)

	log.Println(spew.Sdump(b))
	log.Println(n)
}
