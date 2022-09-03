// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package integrity

import (
	"errors"
	"math/bits"
	"sync"

	swarmpb "github.com/MemeLabs/strims/pkg/apis/type/swarm"
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/bufioutil"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/merkle"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

// errors ...
var (
	ErrMissingHashSubtree = errors.New("missing hash subtree")
	ErrSignatureTooShort  = errors.New("signature too short")
	ErrInvalidSignature   = errors.New("signature mismatch")
	ErrBinOutOfBounds     = errors.New("bin out of bounds")
)

// MerkleOptions ...
type MerkleOptions struct {
	LiveDiscardWindow  int
	ChunkSize          int
	ChunksPerSignature int
	Verifier           SignatureVerifier
	Hash               HashFunc
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
	hash              HashFunc
	treePool          sync.Pool
}

func (v *MerkleSwarmVerifier) tree(b binmap.Bin) *merkle.Tree {
	b = binmap.NewBin(v.treeHeight-1, uint64(b)>>v.treeHeight)
	if ti := v.treePool.Get(); ti != nil {
		t := ti.(*merkle.Tree)
		t.Reset(b)
		return t
	}
	return merkle.NewTree(b, v.chunkSize, v.hash)
}

func (v *MerkleSwarmVerifier) segment(b binmap.Bin) *merkleTreeSegment {
	v.lock.Lock()
	defer v.lock.Unlock()

	i := uint64(b >> v.treeHeight)
	if i >= v.head || i < v.tail {
		return nil
	}

	return v.segments[i&v.mask]
}

func (v *MerkleSwarmVerifier) storeSegment(ts timeutil.Time, tree *merkle.Tree, sig []byte) {
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

				v.treePool.Put(s.Free())
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
	s := v.segment(b)
	if s == nil || !s.LockIf(b) {
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

func (v *MerkleSwarmVerifier) ImportCache(c *swarmpb.Cache) error {
	ic := c.Integrity.MerkleIntegrity
	if ic == nil {
		return errors.New("no supported integrity cache found")
	}

	for i, t := range ic.Timestamps {
		ts := timeutil.Time(t)
		b := binmap.NewBin(v.treeHeight-1, uint64(i))
		tree := v.tree(b)

		l := b.BaseOffset() * uint64(v.chunkSize)
		r := (b.BaseOffset() + b.BaseLength()) * uint64(v.chunkSize)
		tree.Fill(b, c.Data[l:r])

		if !v.signatureVerifier.Verify(ts, tree.Get(tree.RootBin()), ic.Signatures[i]) {
			return ErrInvalidSignature
		}
		v.storeSegment(ts, tree, ic.Signatures[i])
	}

	return nil
}

func (v *MerkleSwarmVerifier) ExportCache() *swarmpb.Cache_Integrity {
	v.lock.Lock()
	defer v.lock.Unlock()

	c := &swarmpb.Cache_MerkleIntegrity{}

	for _, s := range v.segments {
		if s == nil {
			break
		}
		c.Timestamps = append(c.Timestamps, int64(s.Timestamp))
		c.Signatures = append(c.Signatures, s.Signature)
	}

	return &swarmpb.Cache_Integrity{MerkleIntegrity: c}
}

func (v *MerkleSwarmVerifier) Reset() {
	v.lock.Lock()
	defer v.lock.Unlock()

	v.head = 0
	v.tail = 0

	for i, s := range v.segments {
		if s != nil {
			v.treePool.Put(s.Free())
			segmentPool.Put(s)
			v.segments[i] = nil
		}
	}
}

var segmentPool = sync.Pool{
	New: func() any {
		return &merkleTreeSegment{}
	},
}

type merkleTreeSegment struct {
	sync.Mutex
	Timestamp timeutil.Time
	Signature []byte
	Tree      *merkle.Tree
}

func (s *merkleTreeSegment) Reset(ts timeutil.Time, tree *merkle.Tree, sig []byte) {
	s.Lock()
	defer s.Unlock()

	s.Timestamp = ts
	s.Signature = append(s.Signature[:0], sig...)
	s.Tree = tree
}

func (s *merkleTreeSegment) Free() *merkle.Tree {
	s.Lock()
	defer s.Unlock()

	tree := s.Tree
	s.Tree = nil
	return tree
}

func (s *merkleTreeSegment) LockIf(b binmap.Bin) bool {
	s.Lock()
	if s.Tree == nil || !s.Tree.RootBin().Contains(b) {
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
		munroLayer: v.treeHeight - 1,
		chunkVerifier: &MerkleChunkVerifier{
			swarmVerifier: v,
		},
	}
}

// MerkleChannelVerifier ...
type MerkleChannelVerifier struct {
	munroLayer    uint64
	chunkVerifier *MerkleChunkVerifier
}

// ChunkVerifier ...
func (v *MerkleChannelVerifier) ChunkVerifier(b binmap.Bin) ChunkVerifier {
	b = b.LayerShift(v.munroLayer)
	if v.chunkVerifier.bin != b {
		v.chunkVerifier.Reset(b)
	}
	return v.chunkVerifier
}

// MerkleChunkVerifier ...
type MerkleChunkVerifier struct {
	bin           binmap.Bin
	swarmVerifier *MerkleSwarmVerifier
	timestamp     timeutil.Time
	signature     []byte
	tree          *merkle.Tree
}

// Reset ...
func (v *MerkleChunkVerifier) Reset(b binmap.Bin) {
	v.tree = v.swarmVerifier.tree(b)
	v.bin = v.tree.RootBin()
	v.signature = v.signature[:0]
}

// SetSignedIntegrity ...
func (v *MerkleChunkVerifier) SetSignedIntegrity(b binmap.Bin, ts timeutil.Time, sig []byte) {
	v.timestamp = ts
	v.signature = append(v.signature[:0], sig...)
}

// SetIntegrity ...
func (v *MerkleChunkVerifier) SetIntegrity(b binmap.Bin, hash []byte) {
	if v.bin.Contains(b) {
		v.tree.Set(b, hash)
	}
}

func (v *MerkleChunkVerifier) verify(b binmap.Bin, d []byte) (bool, error) {
	if !v.bin.Contains(b) {
		return false, ErrBinOutOfBounds
	}

	var tree *merkle.Tree
	if s := v.swarmVerifier.segment(b); s != nil {
		if !s.LockIf(b) {
			return false, ErrMissingHashSubtree
		}
		defer s.Unlock()
		tree = s.Tree
	}

	if verified, err := v.tree.Verify(b, d, tree); err != nil {
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
	verified, err := v.verify(b, d)
	if verified && err == nil {
		v.swarmVerifier.storeSegment(v.timestamp, v.tree, v.signature)
	}

	v.bin = binmap.None

	return verified, err
}

// MerkleWriterOptions ...
type MerkleWriterOptions struct {
	Verifier           *MerkleSwarmVerifier
	Writer             ioutil.WriteFlushResetter
	ChunksPerSignature int
	ChunkSize          int
	Signer             SignatureSigner
}

// NewMerkleWriter ...
func NewMerkleWriter(o *MerkleWriterOptions) ioutil.WriteFlushResetter {
	mw := &merkleWriter{
		munroLayer:      uint64(bits.TrailingZeros64(uint64(o.ChunksPerSignature))),
		swarmVerifier:   o.Verifier,
		signatureSigner: o.Signer,
		w:               o.Writer,
	}
	return bufioutil.NewWriter(mw, o.ChunksPerSignature*o.ChunkSize)
}

type merkleWriter struct {
	munroLayer      uint64
	n               uint64
	swarmVerifier   *MerkleSwarmVerifier
	signatureSigner SignatureSigner
	w               ioutil.WriteFlushResetter
}

// Write ...
func (w *merkleWriter) Write(p []byte) (int, error) {
	b := binmap.NewBin(w.munroLayer, w.n)
	w.n++

	ts := timeutil.Now()
	tree := w.swarmVerifier.tree(b)
	tree.Fill(b, p)
	sig := w.signatureSigner.Sign(ts, tree.Get(b))
	w.swarmVerifier.storeSegment(ts, tree, sig)

	return w.w.Write(p)
}

func (w *merkleWriter) Reset() {
	w.n = 0
	w.swarmVerifier.Reset()
	w.w.Reset()
}
