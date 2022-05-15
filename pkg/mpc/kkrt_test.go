// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mpc

import (
	"log"
	"testing"

	"github.com/MemeLabs/strims/pkg/mpc/mpctest"
)

func TestKKRT(t *testing.T) {
	seed := []byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16}

	inputs := make([]Block, 100)
	{
		rng, err := newRNG(seed)
		if err != nil {
			panic(err)
		}
		for i := 0; i < len(inputs); i++ {
			if _, err := rng.Read(inputs[i][:]); err != nil {
				log.Println(err)
			}
		}
	}

	done := make(chan bool)
	ca, cb := mpctest.Pipe()

	var results []Block512

	go func() {
		rng, err := newRNG(seed)
		if err != nil {
			panic(err)
		}
		ot, err := NewKOSReceiver(ca, &NaorPinkasSender{}, rng)
		if err != nil {
			panic(err)
		}
		oprf, err := NewKKRTSender(ca, ot, rng)
		if err != nil {
			panic(err)
		}

		seeds, err := oprf.Send(ca, len(inputs), rng)
		if err != nil {
			panic(err)
		}
		results = make([]Block512, len(inputs))
		for i, selection := range inputs {
			results[i] = oprf.Compute(seeds[i], selection)
		}

		close(done)
	}()

	rng, err := newRNG(seed)
	if err != nil {
		panic(err)
	}
	ot, err := NewKOSSender(cb, &NaorPinkasReceiver{}, rng)
	if err != nil {
		panic(err)
	}
	oprf, err := NewKKRTReceiver(cb, ot, rng)
	if err != nil {
		panic(err)
	}

	outputs, err := oprf.Receive(cb, inputs, rng)
	if err != nil {
		panic(err)
	}

	<-done
	for i := 0; i < len(results); i++ {
		if results[i] != outputs[i] {
			t.Error("output mismatch")
			t.FailNow()
		}
	}
}
