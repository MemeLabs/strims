// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspptest

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestConn(t *testing.T) {
	a, b := NewConnPair()
	d := make([]byte, 500)

	n, err := a.Write(d)
	assert.Equal(t, len(d), n, "write length mismatch")
	assert.NoError(t, err, "write error")

	err = a.Flush()
	assert.NoError(t, err, "flush error")

	rd := make([]byte, 1000)
	n, err = b.Read(rd)
	assert.Equal(t, len(d), n, "write length mismatch")
	assert.NoError(t, err, "write error")
}

func TestUnbufferedConnParallel(t *testing.T) {
	done := make(chan struct{})

	a, b := NewUnbufferedConnPair()

	go func() {
		defer close(done)

		dst := make([]byte, 128)
		for i := 0; i < 1024; i++ {
			n, err := b.Read(dst)
			assert.Equal(t, 128, n)
			assert.NoError(t, err)
			if i&1 == 0 {
				for j := 0; j < 128; j++ {
					if dst[j] != byte(j) {
						t.Errorf("expected %d at %d found %d", 128+j, j, dst[j])
						t.Fail()
						return
					}
				}
			} else {
				for j := 0; j < 128; j++ {
					if dst[j] != byte(128+j) {
						t.Errorf("expected %d at %d found %d", 128+j, j, dst[j])
						t.Fail()
						return
					}
				}
			}
		}
	}()

	src := make([]byte, 256)
	for i := range src {
		src[i] = byte(i)
	}
	for i := 0; i < 4; i++ {
		go func() {
			for i := 0; i < 128; i++ {
				a.Write(src)
			}
		}()
	}

	<-done
}

func TestConnBufferBorrow(t *testing.T) {
	a, b := NewConnPair()

	assert.Equal(t, connMTU, a.Available())
	assert.Equal(t, connMTU, b.Available())
	assert.Equal(t, 0, len(a.AvailableBuffer()))
	assert.LessOrEqual(t, connMTU, cap(a.AvailableBuffer()))
	assert.Equal(t, 0, len(b.AvailableBuffer()))
	assert.LessOrEqual(t, connMTU, cap(b.AvailableBuffer()))

	a.Write(make([]byte, 32))

	assert.Equal(t, connMTU-32, a.Available())
	assert.Equal(t, 0, len(a.AvailableBuffer()))
	assert.LessOrEqual(t, connMTU-32, cap(a.AvailableBuffer()))

	err := a.Flush()
	assert.NoError(t, err)

	assert.Equal(t, connMTU, a.Available())
	assert.Equal(t, 0, len(a.AvailableBuffer()))
	assert.LessOrEqual(t, connMTU, cap(a.AvailableBuffer()))
}
