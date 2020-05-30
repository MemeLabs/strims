package encoding

import (
	"bufio"
	"encoding/binary"
	"io"
	"testing"
	"time"
)

type sink struct{}

func (s *sink) Write(b []byte) (int, error) {
	return len(b), nil
}

func newWriter() *writer {
	return &writer{
		tmp: make([]byte, 1024),
	}
}

type writer struct {
	tmp []byte
}

func (w *writer) WriteUint32(ww io.Writer, n uint32) {
	binary.BigEndian.PutUint32(w.tmp, n)
	ww.Write(w.tmp[:4])
}

func BenchmarkTest(b *testing.B) {
	w := bufio.NewWriterSize(&sink{}, 1024)
	k := newWriter()

	for i := 0; i < b.N; i++ {
		k.WriteUint32(w, uint32(i))
	}
}

func BenchmarkTime(b *testing.B) {
	v := make([]time.Time, 1000)
	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		v[i%1000] = time.Unix(int64(i), 0)
	}
}
