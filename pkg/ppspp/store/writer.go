package store

import (
	"bufio"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// Publisher ...
type Publisher interface {
	Publish(Chunk) bool
}

// NewWriter ...
func NewWriter(pub Publisher, chunkSize int) *Writer {
	w := &writer{
		chunkSize: chunkSize,
		pub:       pub,
	}

	return &Writer{
		bw: bufio.NewWriterSize(w, chunkSize),
	}
}

// Writer ...
type Writer struct {
	bw *bufio.Writer
}

// Write ...
func (w *Writer) Write(p []byte) (int, error) {
	return w.bw.Write(p)
}

// Flush ...
func (w *Writer) Flush() error {
	return w.bw.Flush()
}

// writer assigns addresses to chunks
type writer struct {
	pub       Publisher
	chunkSize int
	bin       binmap.Bin
}

// Write ...
func (w *writer) Write(p []byte) (n int, err error) {
	if len(p) > w.chunkSize {
		p = p[:w.chunkSize]
	}

	w.pub.Publish(Chunk{w.bin, p})
	w.bin += 2
	return len(p), nil
}
