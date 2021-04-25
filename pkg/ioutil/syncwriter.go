package ioutil

import (
	"io"
	"sync"
)

func NewSyncWriter(w io.Writer) *SyncWriter {
	return &SyncWriter{w: w}
}

type SyncWriter struct {
	w  io.Writer
	mu sync.Mutex
}

func (w *SyncWriter) Write(p []byte) (int, error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	return w.w.Write(p)
}
