package merkle

import (
	"bytes"
	"hash"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// NewTree ...
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
		digests:   make([]byte, int(t.rootBin.BaseLength()*2-1)*t.hash.Size()),
	}
}

// Tree ...
type Tree struct {
	parent    *Tree
	hash      hash.Hash
	chunkSize int
	rootBin   binmap.Bin
	baseLeft  binmap.Bin
	verified  uint64
	digests   []byte
}

// Reset ...
func (t *Tree) Reset(rootBin binmap.Bin) {
	t.rootBin = rootBin
	t.verified = 0
}

// Merge ...
func (t *Tree) Merge(o *Tree) {
	for i := 0; i < int(t.rootBin.BaseLength()*2); i++ {
		if o.verified&(1<<i) != 0 && t.verified&(1<<i) == 0 {
			t.verified |= 1 << i
			s := t.hash.Size()
			copy(t.digests[i*s:], o.digests[i*s:(i+1)*s])
		}
	}
}

// SetRoot ...
func (t *Tree) SetRoot(digest []byte) {
	t.Set(t.rootBin, digest)
	t.setVerified(t.rootBin)
}

// Set ...
func (t *Tree) Set(b binmap.Bin, d []byte) {
	copy(t.digests[int(b-t.baseLeft)*t.hash.Size():], d)
}

func (t *Tree) isVerified(b binmap.Bin) bool {
	return t.verified&(1<<(b-t.baseLeft)) != 0
}

func (t *Tree) setVerified(b binmap.Bin) {
	t.verified |= 1 << (b - t.baseLeft)
}

// Get ...
func (t *Tree) Get(b binmap.Bin) []byte {
	i := int(b - t.baseLeft)
	s := t.hash.Size()
	return t.digests[i*s : (i+1)*s]
}

func (t *Tree) setOrVerify(b binmap.Bin) (ok, verified bool) {
	d := t.Get(b)
	t.hash.Sum(d[:0])
	t.hash.Reset()

	if t.parent != nil && t.parent.isVerified(b) {
		return bytes.Equal(d, t.parent.Get(b)), true
	}

	t.setVerified(b)
	return true, false
}

// Fill ...
func (t *Tree) Fill(b binmap.Bin, d []byte) (ok, verified bool) {
	l := b.BaseLeft()
	r := b.BaseRight()

	for i := 0; i < int(b.BaseLength()); i++ {
		if _, err := t.hash.Write(d[i*t.chunkSize : (i+1)*t.chunkSize]); err != nil {
			panic(err)
			// return false, false
		}

		if ok, verified := t.setOrVerify(l + binmap.Bin(i*2)); !ok {
			return false, false
		} else if verified {
			return true, true
		}
	}

	for i := uint64(1); i <= b.Layer(); i++ {
		l = l.Parent()
		r = r.Parent()
		w := binmap.Bin(1 << (i + 1))
		for j := l; j <= r; j += w {
			if _, err := t.hash.Write(t.Get(j.Left())); err != nil {
				panic(err)
			}
			if _, err := t.hash.Write(t.Get(j.Right())); err != nil {
				panic(err)
			}

			if ok, verified := t.setOrVerify(j); !ok {
				return false, false
			} else if verified && i == b.Layer() {
				return true, true
			}
		}
	}

	return true, false
}

// Verify ...
func (t *Tree) Verify(b binmap.Bin, d []byte) bool {
	if ok, verified := t.Fill(b, d); !ok {
		return false
	} else if verified {
		return true
	}

	for b != t.rootBin {
		if b.IsLeft() {
			if _, err := t.hash.Write(t.Get(b)); err != nil {
				panic(err)
			}
			if _, err := t.hash.Write(t.Get(b.Sibling())); err != nil {
				panic(err)
			}
		} else {
			if _, err := t.hash.Write(t.Get(b.Sibling())); err != nil {
				panic(err)
			}
			if _, err := t.hash.Write(t.Get(b)); err != nil {
				panic(err)
			}
		}
		t.setVerified(b.Sibling())

		b = b.Parent()
		if ok, verified := t.setOrVerify(b); !ok {
			return false
		} else if verified {
			return true
		}
	}

	return false
}
