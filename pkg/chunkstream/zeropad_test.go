// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chunkstream

import (
	"bytes"
	"io"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestZeroPadWriter(t *testing.T) {
	var buf bytes.Buffer
	w, err := NewZeroPadWriterSize(&buf, 32)
	assert.NoError(t, err, "expected NewZeroPadWriterSize to return nil error")

	b := make([]byte, 75)
	for i := range b {
		b[i] = 255
	}

	n, err := w.Write(b)
	assert.NoError(t, err, "expected write to return nil error")
	assert.Equal(t, len(b), n, "bytes written count mismatch")

	err = w.Flush()
	assert.NoError(t, err, "expected flush to return nil error")

	o := buf.Bytes()

	assert.Equal(t, 96, len(o), "bytes flushed count mismatch")

	headers := []struct {
		index int
		value []byte
	}{
		{0, []byte{0x00, 0x00, 0x00, 0x00}},
		{32, []byte{0x00, 0x00, 0x00, 0x00}},
		{64, []byte{0x80, 0x00, 0x00, 0x17}},
	}
	for _, h := range headers {
		oh := o[h.index : h.index+4]
		assert.Equal(t, oh, h.value, "header mismatch at index %d", h.index)
	}
}

func TestZeroPadReader(t *testing.T) {
	bufSize := 1024
	cases := []struct {
		writes []int
		reads  []int
		offset int
		size   int
	}{
		{
			writes: []int{75, 75, 75},
			reads:  []int{75, 75, 75},
			offset: 0,
			size:   32,
		},
		{
			writes: []int{75, 75, 75},
			reads:  []int{47, 75, 75},
			offset: 32,
			size:   32,
		},
		{
			writes: []int{75, 75, 75},
			reads:  []int{75},
			offset: 3 * 32,
			size:   32,
		},
		{
			writes: []int{9, 250, 311},
			reads:  []int{9, 250, 311},
			offset: 0,
			size:   32,
		},
		{
			writes: []int{9, 250, 311, 100, 112},
			reads:  []int{311, 100, 112},
			offset: 320,
			size:   32,
		},
	}
	for _, c := range cases {
		c := c
		t.Run("", func(t *testing.T) {
			var buf bytes.Buffer
			w, err := NewZeroPadWriterSize(&buf, c.size)
			assert.NoError(t, err, "expected NewZeroPadWriterSize to return nil error")

			in := make([]byte, bufSize)
			for i := range in {
				in[i] = 255
			}

			for _, n := range c.writes {
				_, err := w.Write(in[:n])
				assert.NoError(t, err, "expected ZeroPadWriter.Write to return nil error")
				err = w.Flush()
				assert.NoError(t, err, "expected ZeroPadWriter.Flush to return nil error")
			}

			assert.Equal(t, 0, buf.Len()%c.size, "expected written byte count to be multiple of writer size")

			buf.Next(c.offset)
			r, err := NewZeroPadReaderSize(&buf, int64(c.offset), c.size)
			assert.NoError(t, err, "expected NewZeroPadReaderSize to return nil error")

			out := make([]byte, bufSize)

			for _, n := range c.reads {
				nn, err := io.ReadAtLeast(r, out, n)
				assert.NoError(t, err, "expected ZeroPadWriter.Read to return nil error")
				assert.Equal(t, n, nn, "byte read count mismatch")
				assert.Equal(t, in[:n], out[:nn], "read data mismatch")
			}
		})
	}
}

func TestZeroPadOverhead(t *testing.T) {
	w, err := NewZeroPadWriterSize(io.Discard, 128)
	assert.NoError(t, err)

	cases := []struct {
		size, overhead int
	}{
		{128, 0},
		{256, 0},
		{512, 0},
		{5, 123},
		{64, 64},
		{200, 56},
	}
	for _, c := range cases {
		assert.Equal(t, c.overhead, w.Overhead(c.size))
	}
}
