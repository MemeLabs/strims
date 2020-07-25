package integrity

import (
	"bufio"
	"errors"
	"hash"
	"io"
	"log"
	"math/bits"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/merkle"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/davecgh/go-spew/spew"
	"github.com/petar/GoLLRB/llrb"
)

var errMissingHashSubtree = errors.New("missing hash subtree")

type MerkleTreeOptions struct {
	LiveDiscardWindow int
	ChunkSize         int
	Verifier          SignatureVerifier
	Hash              hashFunc
}

func NewDefaultMerkleTreeOptions() MerkleTreeOptions {
	return MerkleTreeOptions{
		LiveDiscardWindow: 1 << 12,
	}
}

// NewMerkleSwarmVerifier ...
func NewMerkleSwarmVerifier(o *MerkleTreeOptions) *MerkleSwarmVerifier {
	return &MerkleSwarmVerifier{
		capacity:          o.LiveDiscardWindow,
		segments:          llrb.New(),
		chunkSize:         o.ChunkSize,
		signatureVerifier: o.Verifier,
		hash:              o.Hash,
	}
}

// MerkleSwarmVerifier ...
type MerkleSwarmVerifier struct {
	lock              sync.Mutex
	length            int
	capacity          int
	segments          *llrb.LLRB
	chunkSize         int
	signatureVerifier SignatureVerifier
	hash              hashFunc
}

func (v *MerkleSwarmVerifier) findSegment(b binmap.Bin) *merkleTreeSegment {
	si := v.segments.Get(&merkleTreeSegment{
		l: b.BaseLeft(),
		r: b.BaseRight(),
	})
	if si == nil {
		return nil
	}
	return si.(*merkleTreeSegment)
}

func (v *MerkleSwarmVerifier) provisionalSegment(b binmap.Bin) *merkleTreeSegment {
	segment := v.findSegment(b)
	if segment == nil {
		return newMerkleTreeSegment(b, v.chunkSize, v.signatureVerifier, v.hash())
	}
	return newProvisionalMerkleTreeSegment(segment)
}

func (v *MerkleSwarmVerifier) storeSegment(s *merkleTreeSegment) {
	si := v.segments.Get(s)
	if si != nil {
		si.(*merkleTreeSegment).Merge(s)
		return
	}

	v.length += int(s.tree.RootBin().BaseLength())
	for v.length > v.capacity {
		if si := v.segments.DeleteMin(); si != nil {
			v.length -= int(si.(*merkleTreeSegment).tree.RootBin().BaseLength())
		}
	}

	v.segments.InsertNoReplace(s)
}

// WriteIntegrity ...
func (v *MerkleSwarmVerifier) WriteIntegrity(b binmap.Bin, m *binmap.Map, w IntegrityWriter) (int, error) {
	v.lock.Lock()
	defer v.lock.Unlock()

	segment := v.findSegment(b)
	if segment == nil {
		return 0, errMissingHashSubtree
	}

	var n int

	if m.EmptyAt(segment.tree.RootBin()) {
		nn, err := w.WriteSignedIntegrity(codec.SignedIntegrity{
			Address:   codec.Address(segment.tree.RootBin()),
			Timestamp: codec.Timestamp{Time: segment.timestamp},
			Signature: segment.signature,
		})
		n += nn
		if err != nil {
			return n, err
		}
	}

	for b != segment.tree.RootBin() {
		p := b.Parent()
		if !m.EmptyAt(p) {
			return n, nil
		}

		b = b.Sibling()
		nn, err := w.WriteIntegrity(codec.Integrity{
			Address: codec.Address(b),
			Hash:    segment.tree.Get(b),
		})
		n += nn
		if err != nil {
			return n, err
		}

		b = p
	}

	return n, nil
}

func (v *MerkleSwarmVerifier) ChannelVerifier() ChannelVerifier {
	return newMerkleChannelVerifier(v)
}

func newMerkleTreeSegment(
	rootBin binmap.Bin,
	chunkSize int,
	v SignatureVerifier,
	h hash.Hash,
) *merkleTreeSegment {
	return &merkleTreeSegment{
		l:                 rootBin.BaseLeft(),
		r:                 rootBin.BaseRight(),
		signature:         make([]byte, 0, v.Size()),
		tree:              merkle.NewTree(rootBin, chunkSize, h),
		signatureVerifier: v,
	}
}

func newProvisionalMerkleTreeSegment(t *merkleTreeSegment) *merkleTreeSegment {
	return &merkleTreeSegment{
		l:                 t.l,
		r:                 t.r,
		tree:              merkle.NewProvisionalTree(t.tree),
		signatureVerifier: t.signatureVerifier,
	}
}

type merkleTreeSegment struct {
	l, r              binmap.Bin
	timestamp         time.Time
	signature         []byte
	tree              *merkle.Tree
	signatureVerifier SignatureVerifier
}

func (t *merkleTreeSegment) Reset(rootBin binmap.Bin, ts time.Time) {
	t.l = rootBin.BaseLeft()
	t.r = rootBin.BaseRight()
	t.timestamp = ts
	t.signature = t.signature[:0]
	t.tree.Reset(rootBin)
}

func (t *merkleTreeSegment) Less(i llrb.Item) bool {
	if o, ok := i.(*merkleTreeSegment); ok {
		return t.l < o.l && t.r < o.r
	}
	return !i.Less(t)
}

func (t *merkleTreeSegment) SetTimestamp(ts time.Time) {
	t.timestamp = ts
}

func (t *merkleTreeSegment) SetSignature(d []byte) {
	t.signature = append(t.signature, d...)
}

func (t *merkleTreeSegment) Verify(b binmap.Bin, d []byte) bool {
	if ok, verified := t.tree.Verify(b, d); !ok {
		log.Println("tree verification failed")
		return false
	} else if verified {
		return true
	}

	if len(t.signature) != t.signatureVerifier.Size() {
		log.Println(spew.Sdump(t.tree))
		log.Println("signature length mismatch", len(t.signature))
		return false
	}

	return t.signatureVerifier.Verify(t.timestamp, t.tree.Get(t.tree.RootBin()), t.signature)
}

func (t *merkleTreeSegment) Merge(o *merkleTreeSegment) {
	t.tree.Merge(o.tree)
}

func newMerkleChannelVerifier(v *MerkleSwarmVerifier) *MerkleChannelVerifier {
	return &MerkleChannelVerifier{
		swarmVerifier: v,
		chunkVerifier: MerkleChunkVerifier{
			swarmVerifier: v,
		},
	}
}

// MerkleChannelVerifier ...
type MerkleChannelVerifier struct {
	swarmVerifier *MerkleSwarmVerifier
	chunkVerifier MerkleChunkVerifier
}

// ChunkVerifier ...
func (v *MerkleChannelVerifier) ChunkVerifier(b binmap.Bin) ChunkVerifier {
	if !v.chunkVerifier.bin.Contains(b) {
		v.chunkVerifier.bin = b
		v.chunkVerifier.segment = v.swarmVerifier.provisionalSegment(b)
	}
	return &v.chunkVerifier
}

// MerkleChunkVerifier ...
type MerkleChunkVerifier struct {
	bin           binmap.Bin
	segment       *merkleTreeSegment
	swarmVerifier *MerkleSwarmVerifier
}

// SetSignedIntegrity ...
func (v *MerkleChunkVerifier) SetSignedIntegrity(b binmap.Bin, ts time.Time, sig []byte) {
	v.segment.SetTimestamp(ts)
	v.segment.SetSignature(sig)
}

// SetIntegrity ...
func (v *MerkleChunkVerifier) SetIntegrity(b binmap.Bin, hash []byte) {
	v.segment.tree.Set(b, hash)
}

// Verify ...
func (v *MerkleChunkVerifier) Verify(b binmap.Bin, d []byte) bool {
	v.swarmVerifier.lock.Lock()
	defer func() {
		v.swarmVerifier.lock.Unlock()

		v.bin = binmap.None
		v.segment = nil
	}()

	if !v.segment.Verify(b, d) {
		return false
	}

	v.swarmVerifier.storeSegment(v.segment)
	return true
}

// --------------------

type MerkleWriterOptions struct {
	Verifier           *MerkleSwarmVerifier
	Writer             WriteFlusher
	ChunksPerSignature int
	ChunkSize          int
	Signer             SignatureSigner
}

func NewMerkleWriter(o *MerkleWriterOptions) *MerkleWriter {
	mw := &merkleWriter{
		chunkSize:       o.ChunkSize,
		munroLayer:      uint64(bits.TrailingZeros64(uint64(o.ChunksPerSignature))),
		swarmVerifier:   o.Verifier,
		signatureSigner: o.Signer,
		w:               o.Writer,
	}
	return &MerkleWriter{
		bw: bufio.NewWriterSize(mw, o.ChunksPerSignature*o.ChunkSize),
	}
}

type MerkleWriter struct {
	bw *bufio.Writer
}

func (w *MerkleWriter) Write(p []byte) (int, error) {
	return w.bw.Write(p)
}

func (w *MerkleWriter) Flush() error {
	return w.bw.Flush()
}

type merkleWriter struct {
	chunkSize       int
	munroLayer      uint64
	n               uint64
	swarmVerifier   *MerkleSwarmVerifier
	signatureSigner SignatureSigner
	w               io.Writer
}

// Write ...
func (w *merkleWriter) Write(p []byte) (int, error) {
	b := binmap.NewBin(w.munroLayer, w.n)
	w.n++

	w.swarmVerifier.lock.Lock()

	t := time.Now()
	segment := w.swarmVerifier.provisionalSegment(b)
	segment.tree.Fill(b, p)
	segment.SetTimestamp(t)
	segment.SetSignature(w.signatureSigner.Sign(t, segment.tree.Get(b)))
	w.swarmVerifier.storeSegment(segment)

	w.swarmVerifier.lock.Unlock()

	return w.w.Write(p)
}
