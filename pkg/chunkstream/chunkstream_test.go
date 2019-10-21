package chunkstream

import (
	"bytes"
	"log"
	"testing"
)

func TestWriter_Write(t *testing.T) {
	// prefixIvl := 32 - 1
	testLen := 25 << 5

	b := &bytes.Buffer{}
	// w, err := NewWriterSize(b, prefixIvl)
	w, err := NewWriter(b)
	if err != nil {
		panic(err)
	}

	_, err = w.Write(make([]byte, testLen))
	if err != nil {
		panic(err)
	}
	w.Flush()

	// bb := b.Bytes()
	// spew.Dump(bb[0:31])
	// spew.Dump(bb[31:62])
	// spew.Dump(bb[62:70])

	// r, err := NewReaderSize(b, 0, prefixIvl)
	r, err := NewReader(b, 0)
	if err != nil {
		panic(err)
	}
	_ = r

	var total int
	for {
		p := make([]byte, 1000)
		n, err := r.Read(p)
		total += n
		if err == EOR {
			break
		}
	}

	log.Println("total", total)

	if total != testLen {
		t.Error("output length mismatch")
	}
}

func TestWriter_Write1(t *testing.T) {

	v := make([]byte, 4)
	for i := range v {
		v[i] = 255
	}

	b := &bytes.Buffer{}
	// w, err := NewWriterSize(b, prefixIvl)
	w, err := NewWriter(b)
	if err != nil {
		t.Error(err)
		return
	}
	_, err = w.Write(v)
	if err != nil {
		t.Error(err)
		return
	}
	w.Flush()

	bb := b.Bytes()

	expectedLen := len(v) + 2
	if len(bb) != expectedLen {
		t.Errorf("output byte length mismatch: received %d, expected %d", len(bb), expectedLen)
		return
	}

	expected := []byte{128, 4, 255, 255, 255, 255}
	for i := range expected {
		if bb[i] != expected[i] {
			t.Errorf("unexpected value at offset %d: received %d, expected %d", i, bb[i], expected[i])
			return
		}
	}
}
