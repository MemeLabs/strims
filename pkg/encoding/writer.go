package encoding

import (
	"bufio"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// constants...
const (
	// DefaultChunkSize          = 1024
	DefaultChunksPerSignature = 64
	DefaultBufferSize         = 16 * 1024
)

// SwarmWriterOptions ...
type SwarmWriterOptions struct {
	SwarmOptions
	Key *pb.Key
	// ChunkSize          int
	// ChunksPerSignature int
	// LiveDiscardWindow  int
	// ChunkAddressingMethod ChunkAddressingMethod,
	// ContentIntegrityProtectionMethod ContentIntegrityProtectionMethod,
	// MerkleHashTreeFunction MerkleHashTreeFunction,
	// LiveSignatureAlgorithm LiveSignatureAlgorithm,
}

// DefaultSwarmWriterOptions ...
var DefaultSwarmWriterOptions = SwarmWriterOptions{
	SwarmOptions: NewDefaultSwarmOptions(),
	// ChunkSize:          DefaultChunkSize,
	// ChunksPerSignature: DefaultChunksPerSignature,
	// LiveDiscardWindow:  DefaultBufferSize,
	// ChunkAddressingMethod: ChunkAddressingMethod.Bin32,
	// ContentIntegrityProtectionMethod: ContentIntegrityProtectionMethod.UnifiedMerkleTree,
	// MerkleHashTreeFunction: MerkleHashTreeFunction.SHA256,
	// LiveSignatureAlgorithm: LiveSignatureAlgorithm.ECDSAP256SHA256,
}

// NewWriter ...
func NewWriter(o SwarmWriterOptions) (w *SwarmWriter, err error) {
	// privateKey, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	// if err != nil {
	// 	return
	// }

	// encodedPub, err := x509.MarshalPKIXPublicKey(privateKey.Public())
	// if err != nil {
	// 	return
	// }

	// signatureSize := o.ChunkSize * o.ChunksPerSignature

	// signer := NewSigner(privateKey, crypto.SHA256)

	id := NewSwarmID(o.Key.Public)
	s, err := NewSwarm(id, o.SwarmOptions)
	if err != nil {
		return
	}

	bw := bufio.NewWriterSize(&swarmWriter{s: s}, s.ChunkSize)

	w = &SwarmWriter{
		s:  s,
		bw: bw,
	}
	return
}

// SwarmWriter ...
type SwarmWriter struct {
	s  *Swarm
	bw *bufio.Writer
}

// Swarm ...
func (w *SwarmWriter) Swarm() *Swarm {
	return w.s
}

// Write ...
func (w *SwarmWriter) Write(p []byte) (n int, err error) {
	return w.bw.Write(p)
}

// Flush ...
func (w *SwarmWriter) Flush() (err error) {
	if err = w.bw.Flush(); err != nil {
		return err
	}
	// TODO: wait for the data to be written to some number of peers?
	return
}

// Close shut down the swarm...
func (w *SwarmWriter) Close() (err error) {
	return w.s.Leave()
}

// swarmWriter assigns addresses to chunks
type swarmWriter struct {
	s   *Swarm
	bin binmap.Bin
}

// Write ...
func (s *swarmWriter) Write(p []byte) (n int, err error) {
	if len(p) > s.s.ChunkSize {
		p = p[:s.s.ChunkSize]
	}

	s.s.WriteChunk(s.bin, p)
	s.bin += 2
	return len(p), nil
}
