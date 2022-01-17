package store

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
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
					b.Set(bin, binData[l:h])
				}
			}()

			byteOffset := binByte(c.inputBin.BaseLeft(), uint64(c.chunkSize))
			assert.EqualValues(t, int(byteOffset), int(b.Offset()), "read offset mismatch")

			for i := 0; i < binByteLen/c.readSize; i++ {
				n, err := b.Read(readData)
				assert.NoError(t, err, "read failed")

				l := i * c.readSize
				h := l + c.readSize
				assert.Equal(t, c.readSize, n, "incomplete read from %d - %d", l, h)
				assert.EqualValues(t, binData[l:h], readData, "misaligned read from %d - %d", l, h)
			}
		})
	}
}

func TestBufferBinOps(t *testing.T) {
	chunkCount := 1024
	chunkSize := 2048
	inputBin := binmap.NewBin(5, 0)
	readSize := 4096

	b, err := NewBuffer(chunkCount, chunkSize)
	assert.NoError(t, err, "buffer constructor failed")

	binByteLen := int(inputBin.BaseLength()) * chunkSize
	binData := make([]byte, binByteLen)

	for i := 0; i < binByteLen; i++ {
		binData[i] = byte(i / readSize)
	}

	go func() {
		b.SetOffset(inputBin)

		off := binByte(inputBin.BaseLeft(), uint64(chunkSize))
		l := binByte(inputBin.BaseLeft(), uint64(chunkSize)) - off
		h := binByte(inputBin.BaseRight()+2, uint64(chunkSize)) - off
		b.Set(inputBin, binData[l:h])
	}()

	byteOffset := binByte(inputBin.BaseLeft(), uint64(chunkSize))
	assert.EqualValues(t, int(byteOffset), int(b.Offset()), "read offset mismatch")

	assert.Equal(t, false, b.EmptyAt(binmap.NewBin(0, 0)), "EmptyAt returned incorrect value")
	assert.Equal(t, true, b.FilledAt(binmap.NewBin(0, 0)), "FilledAt returned incorrect value")
	assert.Equal(t, inputBin, b.Cover(binmap.NewBin(0, 0)), "Cover returned incorrect value")
	assert.Equal(t, true, b.ReadBin(binmap.NewBin(0, 0), make([]byte, chunkSize)), "ReadBin failed")
}

func TestBufferRecover(t *testing.T) {
	b, err := NewBuffer(1024, 16)
	assert.NoError(t, err, "buffer construction failed")

	b.SetOffset(0)

	src := make([]byte, 16)
	dst := make([]byte, 128*16)

	for i := binmap.Bin(0); i < 256; i = i.LayerRight() {
		b.Set(i, src)
	}

	n, err := b.Read(dst)
	assert.EqualValues(t, 128*16, n)
	assert.NoError(t, err)

	b.Set(4096, src)

	_, err = b.Read(dst)
	assert.Equal(t, ErrBufferUnderrun, err)

	rn, err := b.Recover()
	assert.EqualValues(t, 1920*16, rn)
	assert.NoError(t, err)

	n, err = b.Read(dst)
	assert.EqualValues(t, 16, n)
	assert.NoError(t, err)
}

func TestBufferReadStop(t *testing.T) {
	b, _ := NewBuffer(1024, 16)

	ch := make(chan struct{})
	close(ch)

	b.SetReadStopper(ch)
	_, err := b.Read(nil)
	assert.ErrorIs(t, err, ioutil.ErrStopped)
}
