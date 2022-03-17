package bufioutil

import (
	"io"
)

func NewWriter(w io.Writer, size int) *Writer {
	return &Writer{
		buf: make([]byte, size),
		w:   w,
	}
}

type Writer struct {
	n   int
	buf []byte
	w   io.Writer
}

// Write writes exactly n bytes to the underlying buffer as soon as they are
// available. This differs from bufio.Writer in two ways - bytes are never
// buffered when they could be written and the underlying writer never receives
// more than n bytes.
func (w *Writer) Write(p []byte) (nn int, err error) {
	for w.n+len(p) >= len(w.buf) && err == nil {
		var n int
		if w.n == 0 {
			n, err = w.w.Write(p[:len(w.buf)])
		} else {
			n = copy(w.buf[w.n:], p)
			w.n += n
			err = w.Flush()
		}

		nn += n
		p = p[n:]
	}
	if err != nil {
		return nn, err
	}

	n := copy(w.buf[w.n:], p)
	w.n += n
	nn += n

	return nn, nil
}

func (w *Writer) Flush() error {
	if w.n == 0 {
		return nil
	}
	_, err := w.w.Write(w.buf[:w.n])
	w.n = 0
	return err
}

func (w *Writer) Available() int {
	return len(w.buf) - w.n
}

func (w *Writer) AvailableBuffer() []byte {
	return w.buf[w.n:][:0]
}
