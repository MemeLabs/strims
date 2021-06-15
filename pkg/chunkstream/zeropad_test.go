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
		{0, []byte{0, 0}},
		{32, []byte{0, 0}},
		{64, []byte{0x80, 0x11}},
	}
	for _, h := range headers {
		oh := o[h.index : h.index+2]
		assert.Equal(t, oh, h.value, "header mismatch at index %d", h.index)
	}
}

func TestZeroPadReader(t *testing.T) {
	var buf bytes.Buffer
	w, err := NewZeroPadWriterSize(&buf, 32)
	assert.NoError(t, err, "expected NewZeroPadWriterSize to return nil error")

	b := make([]byte, 75)
	for i := range b {
		b[i] = 255
	}

	for i := 0; i < 3; i++ {
		_, err := w.Write(b)
		assert.NoError(t, err, "expected ZeroPadWriter.Write to return nil error")
		err = w.Flush()
		assert.NoError(t, err, "expected ZeroPadWriter.Flush to return nil error")
	}

	assert.Equal(t, 288, buf.Len(), "expected written byte count to be multiple of writer size")

	r, err := NewZeroPadReaderSize(&buf, 0, 32)
	assert.NoError(t, err, "expected NewZeroPadReaderSize to return nil error")

	o := make([]byte, 1024)

	for i := 0; i < 3; i++ {
		n, err := io.ReadAtLeast(r, o, 75)
		assert.NoError(t, err, "expected ZeroPadWriter.Read to return nil error")
		assert.Equal(t, len(b), n, "byte read count mismatch")
		assert.Equal(t, b, o[:n], "read data mismatch")
	}
}
