package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
)

// WriterOptions ...
type WriterOptions struct {
	SwarmOptions SwarmOptions
	Key          *pb.Key
}

// NewWriter ...
func NewWriter(o WriterOptions) (*Writer, error) {
	id := NewSwarmID(o.Key.Public)
	s, err := NewSwarm(id, o.SwarmOptions)
	if err != nil {
		return nil, err
	}

	return &Writer{
		w: store.NewWriter(s.pubSub, s.chunkSize()),
		s: s,
	}, nil
}

// Writer ...
type Writer struct {
	w *store.Writer
	s *Swarm
}

// Swarm ...
func (w *Writer) Swarm() *Swarm {
	return w.s
}

// Write ...
func (w *Writer) Write(p []byte) (int, error) {
	return w.w.Write(p)
}

// Flush ...
func (w *Writer) Flush() error {
	return w.w.Flush()
}

// Close shut down the swarm...
func (w *Writer) Close() (err error) {
	return w.s.Close()
}
