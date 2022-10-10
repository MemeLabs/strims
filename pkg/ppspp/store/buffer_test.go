// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package store

import (
	"context"
	"io"
	"testing"
	"time"

	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/stretchr/testify/assert"
)

func TestBufferWriteRead(t *testing.T) {
	type test struct {
		label      string
		chunkCount int
		chunkSize  int
		inputBin   binmap.Bin
		writeOrder []binmap.Bin
		readSize   int
	}

	cases := []test{
		{
			label:      "write 65536 bytes read 4096 byte segments",
			chunkCount: 1024,
			chunkSize:  2048,
			inputBin:   binmap.NewBin(5, 0),
			writeOrder: []binmap.Bin{binmap.NewBin(5, 0)},
			readSize:   4096,
		},
		{
			label:      "write 16384 bytes read 1024 byte segments",
			chunkCount: 1024,
			chunkSize:  1024,
			inputBin:   binmap.NewBin(4, 3),
			writeOrder: []binmap.Bin{binmap.NewBin(4, 3)},
			readSize:   1024,
		},
		{
			label:      "write 8192 bytes advancing head read 128 byte segments",
			chunkCount: 1024,
			chunkSize:  1024,
			inputBin:   binmap.NewBin(3, 128),
			writeOrder: []binmap.Bin{binmap.NewBin(3, 128)},
			readSize:   128,
		},
		{
			label:      "write 8192 bytes in order read 128 byte segments",
			chunkCount: 1024,
			chunkSize:  1024,
			inputBin:   binmap.NewBin(3, 1),
			writeOrder: []binmap.Bin{
				binmap.NewBin(2, 2),
				binmap.NewBin(2, 3),
			},
			readSize: 128,
		},
		{
			label:      "write 8192 bytes in reverse order read 128 byte segments",
			chunkCount: 1024,
			chunkSize:  1024,
			inputBin:   binmap.NewBin(3, 0),
			writeOrder: []binmap.Bin{
				binmap.NewBin(2, 1),
				binmap.NewBin(1, 1),
				binmap.NewBin(0, 1),
				binmap.NewBin(0, 0),
			},
			readSize: 128,
		},
		{
			label:      "write 8192 bytes in random order read 128 byte segments",
			chunkCount: 1024,
			chunkSize:  1024,
			inputBin:   binmap.NewBin(3, 0),
			writeOrder: []binmap.Bin{
				binmap.NewBin(0, 7),
				binmap.NewBin(0, 6),
				binmap.NewBin(0, 3),
				binmap.NewBin(0, 4),
				binmap.NewBin(0, 0),
				binmap.NewBin(0, 1),
				binmap.NewBin(0, 2),
				binmap.NewBin(0, 5),
			},
			readSize: 128,
		},
	}

	for _, c := range cases {
		c := c
		t.Run(c.label, func(t *testing.T) {
			b, err := NewBuffer(c.chunkCount, c.chunkSize)
			assert.NoError(t, err, "buffer constructor failed")

			binByteLen := int(c.inputBin.BaseLength()) * c.chunkSize
			binData := make([]byte, binByteLen)
			readData := make([]byte, c.readSize)

			for i := 0; i < binByteLen; i++ {
				binData[i] = byte(i / c.readSize)
			}

			go func() {
				b.SetOffset(c.inputBin)

				off := binByte(c.inputBin.BaseLeft(), uint64(c.chunkSize))
				for _, bin := range c.writeOrder {
					l := binByte(bin.BaseLeft(), uint64(c.chunkSize)) - off
					h := binByte(bin.BaseRight()+2, uint64(c.chunkSize)) - off
					b.Consume(Chunk{bin, binData[l:h]})
				}
			}()

			r := NewBufferReader(b)

			byteOffset := binByte(c.inputBin.BaseLeft(), uint64(c.chunkSize))
			assert.Equal(t, byteOffset, r.Offset(), "read offset mismatch")

			for i := 0; i < binByteLen/c.readSize; i++ {
				n, err := r.Read(readData)
				assert.NoError(t, err, "read failed")

				l := i * c.readSize
				h := l + c.readSize
				assert.Equal(t, c.readSize, n, "incomplete read from %d - %d", l, h)
				assert.EqualValues(t, binData[l:h], readData, "misaligned read from %d - %d", l, h)
			}
		})
	}
}

func TestMultipleReaders(t *testing.T) {
	chunkCount := 1024
	chunkSize := 1024
	total := chunkSize * chunkCount

	b, err := NewBuffer(chunkCount, chunkSize)
	assert.NoError(t, err, "buffer constructor failed")

	b.SetOffset(0)

	src := make([]byte, chunkSize)
	for i := 0; i < chunkCount; i++ {
		b.Consume(Chunk{binmap.NewBin(0, uint64(i)), src})
	}

	for i := 0; i < 3; i++ {
		r := NewBufferReader(b)
		n, err := io.Copy(io.Discard, io.LimitReader(r, int64(total)))
		assert.EqualValues(t, total, n)
		assert.NoError(t, err)
	}
}

func TestBufferRecover(t *testing.T) {
	b, err := NewBuffer(1024, 16)
	assert.NoError(t, err, "buffer construction failed")

	b.SetOffset(0)

	src := make([]byte, 16)
	dst := make([]byte, 128*16)

	for i := binmap.Bin(0); i < 256; i = i.LayerRight() {
		b.Consume(Chunk{i, src})
	}

	r := NewBufferReader(b)

	n, err := r.Read(dst)
	assert.EqualValues(t, 128*16, n)
	assert.NoError(t, err)

	b.Consume(Chunk{4096, src})

	_, err = r.Read(dst)
	assert.Equal(t, ErrBufferUnderrun, err)

	rn, err := r.Recover()
	assert.EqualValues(t, 1920*16, rn)
	assert.NoError(t, err)

	n, err = r.Read(dst)
	assert.EqualValues(t, 16, n)
	assert.NoError(t, err)
}

func TestBufferReadStop(t *testing.T) {
	b, _ := NewBuffer(1024, 16)

	ch := make(chan struct{})
	close(ch)

	r := NewBufferReader(b)
	r.SetReadStopper(ch)
	_, err := r.Read(nil)
	assert.ErrorIs(t, err, ioutil.ErrStopped)
}

func TestReaderClose(t *testing.T) {
	chunkCount := 1024
	chunkSize := 1024
	total := chunkSize * chunkCount

	b, err := NewBuffer(chunkCount, chunkSize)
	assert.NoError(t, err, "buffer constructor failed")

	b.SetOffset(0)

	src := make([]byte, chunkSize)
	for i := 0; i < chunkCount; i++ {
		b.Consume(Chunk{binmap.NewBin(0, uint64(i)), src})
	}

	r := NewBufferReader(b)
	n, err := io.Copy(io.Discard, io.LimitReader(r, int64(total)))
	assert.EqualValues(t, total, n)
	assert.NoError(t, err)

	done := make(chan struct{})
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	go func() {
		r.SetReadStopper(ctx.Done())
		_, err := r.Read(nil)
		assert.ErrorIs(t, err, ErrClosed, "blocked reader should return close")
		close(done)
	}()

	time.Sleep(10 * time.Millisecond)

	err = r.Close()
	assert.NoError(t, err)

	_, err = r.Read(nil)
	assert.ErrorIs(t, err, ErrClosed, "read after close should return closed")

	<-done
}
