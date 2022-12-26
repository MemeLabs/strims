// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package merkle

import (
	"bytes"
	"errors"
	"hash"
	"math/bits"

	"github.com/MemeLabs/strims/pkg/binmap"
)

// ErrHashMismatch ...
var ErrHashMismatch = errors.New("hash mismatch")

// NewTree creates an empty Merkle tree
func NewTree(rootBin binmap.Bin, chunkSize int, hashFunc func() hash.Hash) *Tree {
	h := hashFunc()
	return &Tree{
		hash:      h,
		chunkSize: chunkSize,
		rootBin:   rootBin,
		baseLeft:  rootBin.BaseLeft(),
		verified:  make([]uint64, (rootBin.BaseLength()*2+63)/64),
		digests:   make([]byte, int(rootBin.BaseLength()*2-1)*h.Size()),
	}
}

// Tree resprents a Merkle tree, a tree of hashes.
type Tree struct {
	hash      hash.Hash
	chunkSize int
	rootBin   binmap.Bin
	baseLeft  binmap.Bin
	verified  []uint64
	digests   []byte
}

// Reset sets the tree's root bin and clears the verified bitmap
func (t *Tree) Reset(rootBin binmap.Bin) {
	if t.rootBin.BaseLength() != rootBin.BaseLength() {
		panic("reset cannot change root bin size")
	}

	t.rootBin = rootBin
	t.baseLeft = rootBin.BaseLeft()
	for i := range t.verified {
		t.verified[i] = 0
	}
}

// Merge copies the verified hashes from o to the corresponding bin in t if the
// bin in t is not verified
func (t *Tree) Merge(o *Tree) {
	s := t.hash.Size()
	for i := 0; i < len(o.verified); i++ {
		added := o.verified[i] & ^t.verified[i]
		t.verified[i] |= o.verified[i]

		for j := 0; added != 0; {
			n := bits.TrailingZeros64(added)
			added >>= n
			j += n

			n = bits.TrailingZeros64(added + 1)
			copy(t.digests[(j+i*64)*s:], o.digests[(j+i*64)*s:(j+i*64+n)*s])
			added >>= n
			j += n
		}
	}
}

// RootBin ...
func (t *Tree) RootBin() binmap.Bin {
	return t.rootBin
}

// Verified ...
func (t *Tree) Verified() []binmap.Bin {
	v := []binmap.Bin{}
	for i := t.rootBin.BaseLeft(); i < t.rootBin.BaseRight(); i++ {
		if t.isVerified(i) {
			v = append(v, i)
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
	j := b - t.baseLeft
	return t.verified[j>>6]&(1<<(j&0x3f)) != 0
}

// mark the bin as verified
func (t *Tree) setVerified(b binmap.Bin) {
	j := b - t.baseLeft
	t.verified[j>>6] |= 1 << (j & 0x3f)
}

// Get hash by bin
func (t *Tree) Get(b binmap.Bin) []byte {
	return t.get(b, nil)
}

func (t *Tree) get(b binmap.Bin, p *Tree) []byte {
	if p != nil && p.isVerified(b) {
		return p.Get(b)
	}

	i := int(b - t.baseLeft)
	s := t.hash.Size()
	return t.digests[i*s : (i+1)*s]
}

// setOrVerify updates the hash of b. If the node b of the parent tree is
// verified their hashes are compared
func (t *Tree) setOrVerify(b binmap.Bin, p *Tree) (ok, verified bool) {
	d := t.get(b, nil)
	t.hash.Sum(d[:0])
	t.hash.Reset()

	if p != nil && p.isVerified(b) {
		return bytes.Equal(d, p.get(b, nil)), true
	}

	t.setVerified(b)
	return true, false
}

func (t *Tree) setOrVerifyBranch(b binmap.Bin, p *Tree) (ok, verified bool) {
	if _, err := t.hash.Write(t.get(b.Left(), p)); err != nil {
		return false, false
	}
	if _, err := t.hash.Write(t.get(b.Right(), p)); err != nil {
		return false, false
	}
	return t.setOrVerify(b, p)
}

// Fill fills the leaf nodes under bin with data and verify that the reference
// tree doesn't have other hashes for the affected nodes
func (t *Tree) Fill(b binmap.Bin, d []byte) (bool, error) {
	return t.fill(b, d, nil)
}

func (t *Tree) fill(b binmap.Bin, d []byte, p *Tree) (bool, error) {
	l := b.BaseLeft()
	r := b.BaseRight()

	bl := int(b.BaseLength())
	for i := 0; i < bl; i++ {
		if _, err := t.hash.Write(d[i*t.chunkSize : (i+1)*t.chunkSize]); err != nil {
			return false, err
		}

		if ok, verified := t.setOrVerify(l+binmap.Bin(i*2), p); !ok {
			return false, ErrHashMismatch
		} else if verified && b.IsBase() {
			return true, nil
		}
	}

	for i := uint64(1); i <= b.Layer(); i++ {
		l = l.Parent()
		r = r.Parent()
		w := binmap.Bin(1 << (i + 1))
		for j := l; j <= r; j += w {
			if ok, verified := t.setOrVerifyBranch(j, p); !ok {
				return false, ErrHashMismatch
			} else if verified && b.Layer() == i {
				return true, nil
			}
		}
	}

	return false, nil
}

// Verify checks the integrity of the data d at bin b using the new hashes in t
// and the previously verified hashes in p.
func (t *Tree) Verify(b binmap.Bin, d []byte, p *Tree) (bool, error) {
	if verified, err := t.fill(b, d, p); err != nil {
		return false, err
	} else if verified {
		return true, nil
	}

	for b != t.rootBin {
		t.setVerified(b.Sibling())

		b = b.Parent()
		if ok, verified := t.setOrVerifyBranch(b, p); !ok {
			return false, ErrHashMismatch
		} else if verified {
			return true, nil
		}
	}

	return false, nil
}
