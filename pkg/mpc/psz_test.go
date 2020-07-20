package mpc

import (
	"bytes"
	"errors"
	"log"
	"math/rand"
	"sort"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/mpc/mpctest"
)

func testPSZ(alen, blen, ilen int) error {
	seed := []byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16}

	aset := make([][]byte, alen)
	bset := make([][]byte, blen)
	expected := make([][]byte, ilen)
	{
		rng, err := newRNG(seed)
		if err != nil {
			return err
		}
		for i := 0; i < alen; i++ {
			aset[i] = make([]byte, 16)
			if _, err := rng.Read(aset[i]); err != nil {
				return err
			}
		}
		for i := 0; i < blen; i++ {
			bset[i] = make([]byte, 16)
			if _, err := rng.Read(aset[i]); err != nil {
				return err
			}
		}

		log.Printf("sending %d, receiving %d", len(aset), len(bset))

		copy(expected, bset[:ilen])
		copy(aset, bset[:ilen])
		rand.Shuffle(len(aset), func(i, j int) { aset[i], aset[j] = aset[j], aset[i] })
		rand.Shuffle(len(bset), func(i, j int) { bset[i], bset[j] = bset[j], bset[i] })
	}

	done := make(chan bool)
	ca, cb := mpctest.Pipe()

	go func() {
		rng, err := newRNG(seed)
		if err != nil {
			panic(err)
		}
		ot, err := NewChaoOrlandiSender(ca, rng)
		if err != nil {
			panic(err)
		}
		ote, err := NewKOSReceiver(ca, ot, rng)
		if err != nil {
			panic(err)
		}
		oprf, err := NewKKRTSender(ca, ote, rng)
		if err != nil {
			panic(err)
		}
		psi, err := NewPSZSender(oprf)
		if err != nil {
			panic(err)
		}
		err = psi.Send(ca, aset, rng)
		if err != nil {
			panic(err)
		}

		close(done)
	}()

	rng, err := newRNG(seed)
	if err != nil {
		return err
	}
	ot, err := NewChaoOrlandiReceiver(cb, rng)
	if err != nil {
		return err
	}
	ote, err := NewKOSSender(cb, ot, rng)
	if err != nil {
		return err
	}
	oprf, err := NewKKRTReceiver(cb, ote, rng)
	if err != nil {
		return err
	}
	psi, err := NewPSZReceiver(oprf)
	if err != nil {
		return err
	}
	results, err := psi.Receive(cb, bset, rng)
	if err != nil {
		panic(err)
	}

	<-done

	_ = results
	sort.Sort(bytesSlice(results))
	for _, v := range expected {
		i := sort.Search(ilen, func(i int) bool {
			return bytes.Compare(v, results[i]) <= 0
		})
		if i == ilen || !bytes.Equal(v, results[i]) {
			return errors.New("missing expected result from intersection")
		}
	}

	log.Printf("sent %d, received %d, flushes %d", ca.WrittenBytes(), ca.ReadBytes(), ca.Flushes())
	log.Printf("sent %d, received %d, flushes %d", cb.WrittenBytes(), cb.ReadBytes(), cb.Flushes())

	return nil
}

type bytesSlice [][]byte

func (s bytesSlice) Len() int {
	return len(s)
}

func (s bytesSlice) Less(i, j int) bool {
	return bytes.Compare(s[i], s[j]) == -1
}

func (s bytesSlice) Swap(i, j int) {
	s[i], s[j] = s[j], s[i]
}

func TestPSZ(t *testing.T) {
	cases := []struct {
		alen, blen, ilen int
	}{
		{0, 0, 0},
		{1, 0, 0},
		{15, 15, 15},
		{100, 100, 10},
		{1000, 1000, 250},
	}
	for _, c := range cases {
		if err := testPSZ(c.alen, c.blen, c.ilen); err != nil {
			t.Error(err)
			t.FailNow()
		}
	}
}
