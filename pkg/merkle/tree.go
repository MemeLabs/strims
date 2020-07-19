package merkle

import (
	"bytes"
	"hash"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// NewTree creates an empty Merkle tree, taking in the root bin, chunck size, and root hash
func NewTree(rootBin binmap.Bin, chunkSize int, h hash.Hash) *Tree {
	return &Tree{
		hash:      h,
		chunkSize: chunkSize,
		rootBin:   rootBin,
		digests:   make([]byte, int(rootBin.BaseLength()*2-1)*h.Size()),
	}
}

// NewProvisionalTree ...
func NewProvisionalTree(t *Tree) *Tree {
	return &Tree{
		parent:    t,
		hash:      t.hash,
		chunkSize: t.chunkSize,
		rootBin:   t.rootBin,
		digests:   make([]byte, int(t.rootBin.BaseLength()*2-1)*t.hash.Size()),
	}
}

// Tree resprents a Merkle tree, a tree of hashes.
type Tree struct {
	// parent merkle tree. Nil if the tree does not have a parent
	parent *Tree
	// hash of the current tree's nodes
	hash hash.Hash
	// size of chunks to write
	chunkSize int
	// root node of the tree
	rootBin binmap.Bin
	// verified bitmask containing nodes the tree knows as verified
	verified uint64
	// known hashes to the tree
	digests []byte
}

// Reset sets the tree's rootbin and sets the verified bitmask to 0
func (t *Tree) Reset(rootBin binmap.Bin) {
	t.rootBin = rootBin
	t.verified = 0
}

// Merge copies the verified hashes from o to the corresponding bin in t if the bin in t is not verified
func (t *Tree) Merge(o *Tree) {
	for i := 0; i < int(t.rootBin.BaseLength()*2); i++ {
		if o.verified&(1<<i) != 0 && t.verified&(1<<i) == 0 {
			t.verified |= 1 << i
			s := t.hash.Size()
			copy(t.digests[i*s:], o.digests[i*s:(i+1)*s])
		}
	}
}

// SetRoot set the root hash to the given data
func (t *Tree) SetRoot(digest []byte) {
	t.Set(t.rootBin, digest)
	t.setVerified(t.rootBin)
}

// Set the hash of b to the given data
func (t *Tree) Set(b binmap.Bin, d []byte) {
	start := int(b-t.rootBin.BaseLeft()) * t.hash.Size()
	copy(t.digests[start:], d)
}

// check if the bin is verified
func (t *Tree) isVerified(b binmap.Bin) bool {
	return t.verified&(1<<(b-t.rootBin.BaseLeft())) != 0
}

// mark the bin as verified
func (t *Tree) setVerified(b binmap.Bin) {
	t.verified |= 1 << (b - t.rootBin.BaseLeft())
}

// Get ...
func (t *Tree) Get(b binmap.Bin) []byte {
	i := int(b - t.rootBin.BaseLeft())
	s := t.hash.Size()
	return t.digests[i*s : (i+1)*s]
}

// setOrVerify updates the hash of b. If the node b of the parent tree is verified their hashes are compared
func (t *Tree) setOrVerify(b binmap.Bin) (ok, done bool) {
	d := t.Get(b)
	// overwrite the hash of the current node with the one from the hash we calculated
	t.hash.Sum(d[:0])

	// if we have a parent tree and the parent tree's node with the current index has a verified hash
	if t.parent != nil && t.parent.isVerified(b) {
		// ok if the hashes match ok == true - we got to a node with a verified counterpart in the parent tree, so done == true
		return bytes.Equal(d, t.parent.Get(b)), true
	}

	// we are not at the top so set the current node as verified on the assumption that the hashes match
	t.setVerified(b)
	// ok, but not done yet
	return true, false
}

// Fill fills the leaf nodes under bin with data and verify that the reference tree doesn't have other hashes for the affected nodes
func (t *Tree) Fill(b binmap.Bin, data []byte) bool {
	leftBin := b.BaseLeft()
	rightBin := b.BaseRight()

	// compute hash of data (leaf) nodes under b from left to right
	for i := 0; i < int(b.BaseLength()); i++ {
		t.hash.Reset()
		t.hash.Write(data[i*t.chunkSize : (i+1)*t.chunkSize])
		// set hash and verify integrity
		if ok, _ := t.setOrVerify(leftBin + binmap.Bin(i*2)); !ok {
			return false
		}
	}

	// iterate through layers from 1 to b's layer
	for i := uint64(1); i <= b.Layer(); i++ {
		leftBin = leftBin.Parent()
		rightBin = rightBin.Parent()
		w := binmap.Bin(1 << (i + 1))
		// move through laery nodes under b from left to right
		for j := leftBin; j <= rightBin; j += w {
			t.hash.Reset()
			// calculate and verify hash
			t.hash.Write(t.Get(j.Left()))
			t.hash.Write(t.Get(j.Right()))
			if ok, _ := t.setOrVerify(j); !ok {
				return false
			}
		}
	}

	return true
}

// Verify that the hashes of the target tree and it's parent  match if we assign the specified data to the specified bin
func (t *Tree) Verify(b binmap.Bin, data []byte) bool {
	if !t.Fill(b, data) {
		return false
	}

	for b != t.rootBin {
		t.hash.Reset()
		// calculate hash from left to right
		if b.IsLeft() {
			t.hash.Write(t.Get(b))
			t.hash.Write(t.Get(b.Sibling()))
		} else {
			t.hash.Write(t.Get(b.Sibling()))
			t.hash.Write(t.Get(b))
		}
		// switch to parent node
		b = b.Parent()
		// set hash or verify the hash if parent tree node at b is verified
		ok, done := t.setOrVerify(b)
		if !ok {
			// reached a verified node in the parent tree and hashes did not match
			return false
		}
		if done {
			// found verified parent node and hashes matched
			return true
		}
	}

	// should be false since we never found a reference hash?
	return true
}
