package chunkstream

import (
	"bytes"
	"io"
	"testing"

	"github.com/davecgh/go-spew/spew"
	"github.com/stretchr/testify/assert"
)

func TestWriter(t *testing.T) {
	var buf bytes.Buffer
	w, err := NewWriterSize(&buf, 32)
	if err != nil {
		t.Fatalf("NewWriterSize failed %s", err)
	}

	b := make([]byte, 75)
	for i := range b {
		b[i] = 255
	}

	n, err := w.Write(b)
	if err != nil {
		t.Fatalf("Write failed %s", err)
	}
	if n != len(b) {
		t.Fatalf("%d bytes written, expected %d", n, len(b))
	}

	err = w.Flush()
	if err != nil {
		t.Fatalf("Flush failed %s", err)
	}

	o := buf.Bytes()

	headers := []struct {
		index int
		value []byte
	}{
		{0, []byte{0, 0, 0, 0}},
		{32, []byte{0, 0, 0, 0}},
		{64, []byte{0x80, 0, 0, 0x17}},
	}
	for _, h := range headers {
		oh := o[h.index : h.index+headerLen]
		if !bytes.Equal(oh, h.value) {
			t.Errorf("expected %x at %d, found %x", h.value, h.index, oh)
		}
	}
}

func TestReader(t *testing.T) {
	var buf bytes.Buffer
	w, _ := NewWriterSize(&buf, 32)

	b := make([]byte, 75)
	for i := range b {
		b[i] = 255
	}

	if _, err := w.Write(b); err != nil {
		t.Fatal("failed to write bytes")
	}
	w.Flush()

	r, err := NewReaderSize(&buf, 0, 32)
	if err != nil {
		t.Fatalf("NewReaderSize failed %s", err)
	}

	o := make([]byte, 1024)
	n, err := io.ReadAtLeast(r, o, 75)
	if err != nil {
		t.Fatalf("Read failed %s", err)
	}
	if n != len(b) {
		t.Errorf("%d bytes read, expected %d", n, len(b))
	}

	if !bytes.Equal(o[:n], b) {
		t.Errorf("expected \n%s\nread \n%s", spew.Sdump(b), spew.Sdump(o[:n]))
	}
}

func TestOffsetReader(t *testing.T) {
	var buf bytes.Buffer
	w, _ := NewWriterSize(&buf, 32)

	b := make([]byte, 75)
	for i := range b {
		b[i] = 255
	}

	if _, err := w.Write(b); err != nil {
		t.Fatal("failed to write bytes")
	}
	w.Flush()

	ob := make([]byte, 95)
	off := len(ob) - buf.Len()
	copy(ob[off:], buf.Bytes())

	r, err := NewReaderSize(bytes.NewBuffer(ob), int64(32-off), 32)
	if err != nil {
		t.Fatalf("NewReaderSize failed %s", err)
	}

	o := make([]byte, 1024)
	n, err := io.ReadAtLeast(r, o, 75)
	if err != nil {
		t.Fatalf("Read failed %s", err)
	}
	if n != off+len(b) {
		t.Errorf("%d bytes read, expected %d", n, off+len(b))
	}

	if !bytes.Equal(o[off:n], b) {
		t.Errorf("expected \n%s\nread \n%s", spew.Sdump(b), spew.Sdump(o[off:n]))
	}
}

func TestLengthAlignedWrite(t *testing.T) {
	size := 128 * 1024

	var buf bytes.Buffer
	w, err := NewWriterSize(&buf, size)
	assert.Nil(t, err)

	for i := 0; i < 3; i++ {
		w.Write(make([]byte, size-3))
		w.Flush()
	}

	r, err := NewReaderSize(&buf, 0, size)
	assert.Nil(t, err)

	b := make([]byte, 8*1024)
	for i := 0; i < 12; i++ {
		_, err := r.Read(b)
		if err != nil {
			assert.ErrorIs(t, err, io.EOF)
		}
	}
}
