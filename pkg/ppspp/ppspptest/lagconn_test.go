// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspptest

import (
	"encoding/binary"
	"sync"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestLagConn(t *testing.T) {
	latency := 100 * time.Millisecond
	tolerance := 5 * time.Millisecond

	a, b := NewConnPair()
	a, b = NewLagConnPair(a, b, latency, 0)

	start := time.Now()
	var wg sync.WaitGroup
	wg.Add(2)

	go func() {
		defer wg.Done()
		d := make([]byte, 8)
		for i := 0; i < 10; i++ {
			binary.BigEndian.PutUint64(d, uint64(time.Now().UnixNano()))
			if _, err := a.Write(d); err != nil {
				return
			}
			if err := a.Flush(); err != nil {
				return
			}
		}
	}()

	go func() {
		defer wg.Done()
		d := make([]byte, 8)
		for i := 0; i < 10; i++ {
			if _, err := b.Read(d); err != nil {
				return
			}

			l := time.Since(time.Unix(0, int64(binary.BigEndian.Uint64(d))))
			assert.GreaterOrEqual(t, int64(latency+tolerance), int64(l), "read completed too slowly")
			assert.LessOrEqual(t, int64(latency-tolerance), int64(l), "read completed too quickly")
		}
	}()

	wg.Wait()

	l := time.Since(start)
	assert.GreaterOrEqual(t, int64(latency+tolerance), int64(l), "test completed too slowly")
	assert.LessOrEqual(t, int64(latency-tolerance), int64(l), "test completed too quickly")
}
