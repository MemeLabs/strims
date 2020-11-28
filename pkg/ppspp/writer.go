package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/store"
)

// WriterOptions ...
type WriterOptions struct {
	SwarmOptions SwarmOptions
	Key          *pb.Key
	Integrity    integrity.WriterOptions
}

// NewWriter ...
func NewWriter(o WriterOptions) (*Writer, error) {
	id := NewSwarmID(o.Key.Public)
	s, err := NewSwarm(id, o.SwarmOptions)
	if err != nil {
		return nil, err
	}

	w, err := integrity.NewWriter(o.Key.Private, integrity.SwarmWriterOptions{
		LiveSignatureAlgorithm: s.liveSignatureAlgorithm(),
		ProtectionMethod:       s.contentIntegrityProtectionMethod(),
		ChunkSize:              s.chunkSize(),
		Verifier:               s.verifier,
		Writer:                 store.NewWriter(s.pubSub, s.chunkSize()),
		WriterOptions: integrity.WriterOptions{
			ChunksPerSignature: s.chunksPerSignature(),
		},
	})
	if err != nil {
		return nil, err
	}

	return &Writer{
		w: w,
		s: s,
	}, nil
}

// Writer ...
type Writer struct {
	w integrity.WriteFlusher
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
