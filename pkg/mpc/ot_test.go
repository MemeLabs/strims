// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mpc

import (
	"bytes"
	"errors"
	"io"
	"testing"

	"github.com/MemeLabs/strims/pkg/mpc/mpctest"
)

func testOT(
	n int,
	newSender func(c Conn, rng io.Reader) (OTSender, error),
	newReceiver func(c Conn, rng io.Reader) (OTReceiver, error),
) error {
	seed := []byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16}

	values := make([][2]Block, n)
	receive := []bool{}
	{
		rng, err := newRNG(seed)
		if err != nil {
			panic(err)
		}
		for i := 0; i < n; i++ {
			if _, err := rng.Read(values[i][0][:]); err != nil {
				return err
			}
			if _, err := rng.Read(values[i][1][:]); err != nil {
				return err
			}
		}

		tmp := make([]byte, (n+7)/8)
		if _, err := rng.Read(tmp); err != nil {
			return err
		}
		receive = boolsFromBytes(tmp)[:n]
	}

	done := make(chan bool)
	ca, cb := mpctest.Pipe()

	go func() {
		rng, err := newRNG(seed)
		if err != nil {
			panic(err)
		}
		ot, err := newSender(ca, rng)
		if err != nil {
			panic(err)
		}
		if err := ot.Send(ca, values, rng); err != nil {
			panic(err)
		}
		close(done)
	}()

	rng, err := newRNG(seed)
	if err != nil {
		panic(err)
	}
	ot, err := newReceiver(cb, rng)
	if err != nil {
		panic(err)
	}
	res, err := ot.Receive(cb, receive, rng)
	if err != nil {
		panic(err)
	}

	<-done

	for i := range values {
		var expected Block
		if !receive[i] {
			expected = values[i][0]
		} else {
			expected = values[i][1]
		}
		if !bytes.Equal(res[i][:], expected[:]) {
			return errors.New("received unexpected value")
		}
	}
	return nil
}

func TestNaorPinkas(t *testing.T) {
	newSender := func(c Conn, rng io.Reader) (OTSender, error) {
		return &NaorPinkasSender{}, nil
	}
	newReceiver := func(c Conn, rng io.Reader) (OTReceiver, error) {
		return &NaorPinkasReceiver{}, nil
	}

	for i := 10; i <= 1000; i *= 10 {
		if err := testOT(i, newSender, newReceiver); err != nil {
			t.Errorf("failed with %d values with error %s", i, err)
			t.FailNow()
		}
	}
}

func TestChaoOrlandi(t *testing.T) {
	newSender := func(c Conn, rng io.Reader) (OTSender, error) {
		return NewChaoOrlandiSender(c, rng)
	}
	newReceiver := func(c Conn, rng io.Reader) (OTReceiver, error) {
		return NewChaoOrlandiReceiver(c, rng)
	}

	for i := 10; i <= 1000; i *= 10 {
		if err := testOT(i, newSender, newReceiver); err != nil {
			t.Errorf("failed with %d values with error %s", i, err)
			t.FailNow()
		}
	}
}

func TestALSZ(t *testing.T) {
	newSender := func(c Conn, rng io.Reader) (OTSender, error) {
		return NewALSZSender(c, &NaorPinkasReceiver{}, rng)
	}
	newReceiver := func(c Conn, rng io.Reader) (OTReceiver, error) {
		return NewALSZReceiver(c, &NaorPinkasSender{}, rng)
	}

	for i := 10; i <= 10000; i *= 10 {
		if err := testOT(i, newSender, newReceiver); err != nil {
			t.Errorf("failed with %d values with error %s", i, err)
			t.FailNow()
		}
	}
}

func TestKOS(t *testing.T) {
	newSender := func(c Conn, rng io.Reader) (OTSender, error) {
		return NewKOSSender(c, &NaorPinkasReceiver{}, rng)
	}
	newReceiver := func(c Conn, rng io.Reader) (OTReceiver, error) {
		return NewKOSReceiver(c, &NaorPinkasSender{}, rng)
	}

	for i := 10; i <= 10000; i *= 10 {
		if err := testOT(i, newSender, newReceiver); err != nil {
			t.Errorf("failed with %d values with error %s", i, err)
			t.FailNow()
		}
	}
}
