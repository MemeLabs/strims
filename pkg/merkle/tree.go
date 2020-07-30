package merkle

import (
	"bytes"
	"errors"
	"hash"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// ErrHashMismatch ...
var ErrHashMismatch = errors.New("hash mismatch")

// NewTree creates an empty Merkle tree
func NewTree(rootBin binmap.Bin, chunkSize int, h hash.Hash) *Tree {
	return &Tree{
		hash:      h,
		chunkSize: chunkSize,
		rootBin:   rootBin,
		baseLeft:  rootBin.BaseLeft(),
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
		baseLeft:  t.baseLeft,
		digests:   make([]byte, len(t.digests)),
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
	// bin of leftmost node
	baseLeft binmap.Bin
	// verified bitmask containing nodes the tree knows as verified
	verified uint64
	// known hashes to the tree
	digests []byte
}

// Reset sets the tree's rootbin and sets the verified bitmask to 0
func (t *Tree) Reset(rootBin binmap.Bin, parent *Tree) {
	t.parent = parent
	t.rootBin = rootBin
	t.baseLeft = rootBin.BaseLeft()
	t.verified = 0

	if l := int(rootBin.BaseLength()*2-1) * t.hash.Size(); cap(t.digests) < l {
		t.digests = make([]byte, l)
	} else {
		t.digests = t.digests[:l]
	}
}

// Merge copies the verified hashes from o to the corresponding bin in t if the
// bin in t is not verified
func (t *Tree) Merge(o *Tree) {
	bl := int(t.rootBin.BaseLength())
	for i := 0; i < bl*2; i++ {
		if o.verified&(1<<i) != 0 && t.verified&(1<<i) == 0 {
			t.verified |= 1 << i
			s := t.hash.Size()
			copy(t.digests[i*s:], o.digests[i*s:(i+1)*s])
		}
	}
}

func (t *Tree) SetParent(parent *Tree) {
	t.parent = parent
}

// SetRoot set the root hash to the given data
func (t *Tree) SetRoot(digest []byte) {
	t.Set(t.rootBin, digest)
	t.setVerified(t.rootBin)
}

func (t *Tree) RootBin() binmap.Bin {
	return t.rootBin
}

func (t *Tree) BaseLeft() binmap.Bin {
	return t.baseLeft
}

func (t *Tree) Verified() []binmap.Bin {
	v := []binmap.Bin{}
	for i := t.rootBin.BaseLeft(); i < t.rootBin.BaseRight(); i++ {
		if t.isVerified(i) {
			v = append(v, i-t.RootBin().BaseLeft())
		}
	}
	return v
}

// Set the hash of b to the given data
func (t *Tree) Set(b binmap.Bin, d []byte) {
	start := int(b-t.baseLeft) * t.hash.Size()
	copy(t.digests[start:], d)
}

// check if the bin is verified
func (t *Tree) isVerified(b binmap.Bin) bool {
	return t.verified&(1<<(b-t.baseLeft)) != 0
}

// mark the bin as verified
func (t *Tree) setVerified(b binmap.Bin) {
	t.verified |= 1 << (b - t.baseLeft)
}

// Get ...
func (t *Tree) Get(b binmap.Bin) []byte {
	if t.parent != nil && t.parent.isVerified(b) {
		return t.parent.Get(b)
	}

	i := int(b - t.baseLeft)
	s := t.hash.Size()
	return t.digests[i*s : (i+1)*s]
}

// setOrVerify updates the hash of b. If the node b of the parent tree is
// verified their hashes are compared
func (t *Tree) setOrVerify(b binmap.Bin) (ok, verified bool) {
	d := t.Get(b)
	// overwrite the hash of the current node with the one from the hash we
	// calculated
	t.hash.Sum(d[:0])
	t.hash.Reset()

	// if we have a parent tree and the parent tree's node with the current index
	// has a verified hash
	if t.parent != nil && t.parent.isVerified(b) {
		// ok if the hashes match - we found a node with a verified counterpart in
		// the parent tree
		return bytes.Equal(d, t.parent.Get(b)), true
	}

	// we are not at the top so set the current node as verified on the assumption
	// that the hashes match
	t.setVerified(b)
	// ok, but not done yet
	return true, false
}

// Fill fills the leaf nodes under bin with data and verify that the reference
// tree doesn't have other hashes for the affected nodes
func (t *Tree) Fill(b binmap.Bin, d []byte) (bool, error) {
	l := b.BaseLeft()
	r := b.BaseRight()

	// compute hash of data (leaf) nodes under b from left to right
	bl := int(b.BaseLength())
	for i := 0; i < bl; i++ {
		if _, err := t.hash.Write(d[i*t.chunkSize : (i+1)*t.chunkSize]); err != nil {
			return false, err
		}

		if ok, verified := t.setOrVerify(l + binmap.Bin(i*2)); !ok {
			return false, ErrHashMismatch
		} else if verified && b.Layer() == 0 {
			return true, nil
		}
	}

	for i := uint64(1); i <= b.Layer(); i++ {
		l = l.Parent()
		r = r.Parent()
		w := binmap.Bin(1 << (i + 1))
		for j := l; j <= r; j += w {
			if _, err := t.hash.Write(t.Get(j.Left())); err != nil {
				return false, err
			}
			if _, err := t.hash.Write(t.Get(j.Right())); err != nil {
				return false, err
			}

			if ok, verified := t.setOrVerify(j); !ok {
				return false, ErrHashMismatch
			} else if verified && b.Layer() == i {
				return true, nil
			}
		}
	}

	return false, nil
}

// Verify that the hashes of the target tree and it's parent  match if we assign
// the specified data to the specified bin
func (t *Tree) Verify(b binmap.Bin, d []byte) (bool, error) {
	if verified, err := t.Fill(b, d); err != nil {
		return false, err
	} else if verified {
		return true, nil
	}

	for b != t.rootBin {
		if b.IsLeft() {
			if _, err := t.hash.Write(t.Get(b)); err != nil {
				return false, err
			}
			if _, err := t.hash.Write(t.Get(b.Sibling())); err != nil {
				return false, err
			}
		} else {
			if _, err := t.hash.Write(t.Get(b.Sibling())); err != nil {
				return false, err
			}
			if _, err := t.hash.Write(t.Get(b)); err != nil {
				return false, err
			}
		}
		t.setVerified(b.Sibling())

		b = b.Parent()
		if ok, verified := t.setOrVerify(b); !ok {
			return false, ErrHashMismatch
		} else if verified {
			return true, nil
		}
	}

	return false, nil
}
