package prefixstream

import (
	"bytes"
	"encoding/binary"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/bytereader"
)

var sentinel = [8]byte{0, 1, 1, 2, 3, 5, 8, 13}

const sentinelLen = len(sentinel)

// NewWriter ...
func NewWriter(w io.Writer) *Writer {
	return &Writer{
		w: w,
	}
}

// Writer ...
type Writer struct {
	w io.Writer
	b []byte
}

// Write ...
func (w *Writer) Write(p []byte) (int, error) {
	n := len(p) + sentinelLen + binary.MaxVarintLen32
	if len(w.b) < n {
		w.b = make([]byte, n)
	}
	b := w.b[:n]

	n = copy(b, sentinel[:])
	n += binary.PutUvarint(b[n:], uint64(len(p)))
	n += copy(b[n:], p)

	return w.w.Write(b[:n])
}

// NewReader ...
func NewReader(r io.Reader) *Reader {
	return &Reader{r: r}
}

// Reader ...
type Reader struct {
	r  io.Reader
	h  bytes.Buffer
	hn int
	n  int
}

// Read ...
func (r *Reader) Read(p []byte) (int, error) {
	if err := r.readHeader(); err != nil {
		return 0, err
	}

	n := len(p)
	if n > r.n {
		n = r.n
	}
	n, err := io.ReadFull(r.r, p[:n])
	if err != nil {
		return 0, err
	}

	r.n -= n
	if r.n == 0 {
		err = io.EOF
	}

	return n, err
}

func (r *Reader) readHeader() error {
	for r.n == 0 {
		if _, err := io.CopyN(&r.h, r.r, int64(r.hn)); err != nil {
			return err
		}

		i := bytes.Index(r.h.Bytes(), sentinel[:])
		if i == -1 {
			r.hn = 1
			continue
		}
		r.h.Reset()

		n, err := binary.ReadUvarint(bytereader.New(r.r))
		if err != nil {
			return err
		}
		r.n = int(n)
		r.hn = sentinelLen
	}
	return nil
}
