package integrity

import (
	"bufio"
	"errors"
	"io"
	"math/bits"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/merkle"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
)

// errors ...
var (
	ErrMissingHashSubtree = errors.New("missing hash subtree")
	ErrSignatureTooShort  = errors.New("signature too short")
	ErrInvalidSignature   = errors.New("signature mismatch")
)

// MerkleOptions ...
type MerkleOptions struct {
	LiveDiscardWindow  int
	ChunkSize          int
	ChunksPerSignature int
	Verifier           SignatureVerifier
	Hash               hashFunc
}

// NewMerkleSwarmVerifier ...
func NewMerkleSwarmVerifier(o *MerkleOptions) *MerkleSwarmVerifier {
	size := uint64(o.LiveDiscardWindow / o.ChunksPerSignature)
	return &MerkleSwarmVerifier{
		treeHeight:        uint64(bits.TrailingZeros(uint(o.ChunksPerSignature))) + 1,
		mask:              size - 1,
		size:              size,
		segments:          make([]*merkleTreeSegment, size),
		chunkSize:         o.ChunkSize,
		signatureVerifier: o.Verifier,
		hash:              o.Hash,
	}
}

// MerkleSwarmVerifier ...
type MerkleSwarmVerifier struct {
	lock              sync.Mutex
	treeHeight        uint64
	mask              uint64
	size              uint64
	head, tail        uint64
	segments          []*merkleTreeSegment
	chunkSize         int
	signatureVerifier SignatureVerifier
	hash              hashFunc
	treePool          sync.Pool
}

func (v *MerkleSwarmVerifier) tree(b binmap.Bin) *merkle.Tree {
	b = binmap.NewBin(v.treeHeight-1, uint64(b)>>v.treeHeight)
	if ti := v.treePool.Get(); ti != nil {
		t := ti.(*merkle.Tree)
		t.Reset(b, nil)
		return t
	}
	return merkle.NewTree(b, v.chunkSize, v.hash())
}

func (v *MerkleSwarmVerifier) segment(b binmap.Bin) (*merkleTreeSegment, uint64) {
	v.lock.Lock()
	defer v.lock.Unlock()

	i := uint64(b >> v.treeHeight)
	if i >= v.head || i < v.tail {
		return nil, 0
	}

	s := v.segments[i&v.mask]
	if s == nil {
		return nil, 0
	}

	return s, s.Semaphore()
}

func (v *MerkleSwarmVerifier) storeSegment(ts time.Time, tree *merkle.Tree, sig []byte) {
	v.lock.Lock()
	defer v.lock.Unlock()

	i := uint64(tree.RootBin() >> v.treeHeight)
	if i < v.tail {
		return
	}

	if head := i + 1; head > v.head {
		if v.head != 0 {
			for i := v.head; i < head; i++ {
				s := v.segments[i&v.mask]
				if s == nil {
					continue
				}

				s.Touch()
				v.treePool.Put(s.Tree)
				segmentPool.Put(s)
				v.segments[i&v.mask] = nil
			}
		}
		v.head = head
		if v.head > v.size {
			v.tail = v.head - v.size
		}
	}

	if s := v.segments[i&v.mask]; s != nil {
		s.Merge(tree)
		v.treePool.Put(tree)
	} else {
		s := segmentPool.Get().(*merkleTreeSegment)
		s.Reset(ts, tree, sig)
		v.segments[i&v.mask] = s
	}
}

// WriteIntegrity ...
func (v *MerkleSwarmVerifier) WriteIntegrity(b binmap.Bin, m *binmap.Map, w Writer) (int, error) {
	s, sem := v.segment(b)
	if s == nil {
		return 0, ErrMissingHashSubtree
	}

	if !s.LockIf(sem) {
		return 0, ErrMissingHashSubtree
	}
	defer s.Unlock()

	var n int

	if m.EmptyAt(s.Tree.RootBin()) {
		nn, err := w.WriteSignedIntegrity(codec.SignedIntegrity{
			Address:   codec.Address(s.Tree.RootBin()),
			Timestamp: codec.Timestamp{Time: s.Timestamp},
			Signature: s.Signature,
		})
		n += nn
		if err != nil {
			return n, err
		}
	}

	for b != s.Tree.RootBin() {
		p := b.Parent()
		if !m.EmptyAt(p) {
			return n, nil
		}

		b = b.Sibling()
		nn, err := w.WriteIntegrity(codec.Integrity{
			Address: codec.Address(b),
			Hash:    s.Tree.Get(b),
		})
		n += nn
		if err != nil {
			return n, err
		}

		b = p
	}

	return n, nil
}

// ChannelVerifier ...
func (v *MerkleSwarmVerifier) ChannelVerifier() ChannelVerifier {
	return newMerkleChannelVerifier(v)
}

var segmentPool = sync.Pool{
	New: func() interface{} {
		return &merkleTreeSegment{}
	},
}

type merkleTreeSegment struct {
	sync.Mutex
	semaphor  uint64
	Timestamp time.Time
	Signature []byte
	Tree      *merkle.Tree
}

func (s *merkleTreeSegment) Reset(ts time.Time, tree *merkle.Tree, sig []byte) {
	s.Timestamp = ts
	s.Signature = append(s.Signature[:0], sig...)
	s.Tree = tree
}

func (s *merkleTreeSegment) Touch() {
	s.Lock()
	defer s.Unlock()
	s.semaphor++
}

func (s *merkleTreeSegment) Semaphore() uint64 {
	s.Lock()
	defer s.Unlock()
	return s.semaphor
}

func (s *merkleTreeSegment) LockIf(sem uint64) bool {
	s.Lock()
	if s.semaphor != sem {
		s.Unlock()
		return false
	}
	return true
}

func (s *merkleTreeSegment) Merge(tree *merkle.Tree) {
	s.Lock()
	defer s.Unlock()
	s.Tree.Merge(tree)
}

func newMerkleChannelVerifier(v *MerkleSwarmVerifier) *MerkleChannelVerifier {
	return &MerkleChannelVerifier{
		chunkVerifier: MerkleChunkVerifier{
			swarmVerifier: v,
		},
	}
}

// MerkleChannelVerifier ...
type MerkleChannelVerifier struct {
	chunkVerifier MerkleChunkVerifier
}

// ChunkVerifier ...
func (v *MerkleChannelVerifier) ChunkVerifier(b binmap.Bin) ChunkVerifier {
	if !v.chunkVerifier.bin.Contains(b) {
		v.chunkVerifier.Reset(b)
	}
	return &v.chunkVerifier
}

// MerkleChunkVerifier ...
type MerkleChunkVerifier struct {
	bin           binmap.Bin
	swarmVerifier *MerkleSwarmVerifier
	segment       *merkleTreeSegment
	segmentSem    uint64
	timestamp     time.Time
	signature     []byte
	tree          *merkle.Tree
}

// Reset ...
func (v *MerkleChunkVerifier) Reset(b binmap.Bin) {
	v.tree = v.swarmVerifier.tree(b)
	v.segment, v.segmentSem = v.swarmVerifier.segment(b)
	if v.segment != nil {
		v.tree.SetParent(v.segment.Tree)
	}

	v.bin = v.tree.RootBin()
	v.signature = nil
}

// SetSignedIntegrity ...
func (v *MerkleChunkVerifier) SetSignedIntegrity(b binmap.Bin, ts time.Time, sig []byte) {
	v.timestamp = ts
	v.signature = append(v.signature[:0], sig...)
}

// SetIntegrity ...
func (v *MerkleChunkVerifier) SetIntegrity(b binmap.Bin, hash []byte) {
	v.tree.Set(b, hash)
}

func (v *MerkleChunkVerifier) verify(b binmap.Bin, d []byte) (bool, error) {
	if v.segment != nil {
		if !v.segment.LockIf(v.segmentSem) {
			return false, ErrMissingHashSubtree
		}
		defer v.segment.Unlock()
	}

	if verified, err := v.tree.Verify(b, d); err != nil {
		return false, err
	} else if verified {
		return true, nil
	}

	if len(v.signature) != v.swarmVerifier.signatureVerifier.Size() {
		return false, ErrSignatureTooShort
	}
	if !v.swarmVerifier.signatureVerifier.Verify(v.timestamp, v.tree.Get(v.tree.RootBin()), v.signature) {
		return false, ErrInvalidSignature
	}
	return true, nil
}

// Verify ...
func (v *MerkleChunkVerifier) Verify(b binmap.Bin, d []byte) (bool, error) {
	v.bin = binmap.None

	if verified, err := v.verify(b, d); !verified || err != nil {
		return false, err
	}

	v.swarmVerifier.storeSegment(v.timestamp, v.tree, v.signature)
	return true, nil
}

// MerkleWriterOptions ...
type MerkleWriterOptions struct {
	Verifier           *MerkleSwarmVerifier
	Writer             ioutil.WriteFlusher
	ChunksPerSignature int
	ChunkSize          int
	Signer             SignatureSigner
}

// NewMerkleWriter ...
func NewMerkleWriter(o *MerkleWriterOptions) *MerkleWriter {
	mw := &merkleWriter{
		munroLayer:      uint64(bits.TrailingZeros64(uint64(o.ChunksPerSignature))),
		segmentSize:     o.ChunksPerSignature * o.ChunkSize,
		swarmVerifier:   o.Verifier,
		signatureSigner: o.Signer,
		w:               o.Writer,
	}
	return &MerkleWriter{
		bw: bufio.NewWriterSize(mw, mw.segmentSize),
	}
}

// MerkleWriter ...
type MerkleWriter struct {
	bw *bufio.Writer
}

// Write ...
func (w *MerkleWriter) Write(p []byte) (int, error) {
	return w.bw.Write(p)
}

// Flush ...
func (w *MerkleWriter) Flush() error {
	return w.bw.Flush()
}

type merkleWriter struct {
	munroLayer      uint64
	n               uint64
	segmentSize     int
	swarmVerifier   *MerkleSwarmVerifier
	signatureSigner SignatureSigner
	w               io.Writer
}

// Write ...
func (w *merkleWriter) Write(p []byte) (int, error) {
	if len(p) > w.segmentSize {
		p = p[:w.segmentSize]
	}

	b := binmap.NewBin(w.munroLayer, w.n)
	w.n++

	ts := iotime.Load()
	tree := w.swarmVerifier.tree(b)
	tree.Fill(b, p)
	sig := w.signatureSigner.Sign(ts, tree.Get(b))
	w.swarmVerifier.storeSegment(ts, tree, sig)

	return w.w.Write(p)
}
