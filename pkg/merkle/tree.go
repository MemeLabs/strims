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

// Tree ...
type Tree struct {
	parent    *Tree
	hash      hash.Hash
	chunkSize int
	rootBin   binmap.Bin
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
	copy(t.digests[int(b-t.rootBin.BaseLeft())*t.hash.Size():], d)
}

func (t *Tree) isVerified(b binmap.Bin) bool {
	return t.verified&(1<<(b-t.rootBin.BaseLeft())) != 0
}

func (t *Tree) setVerified(b binmap.Bin) {
	t.verified |= 1 << (b - t.rootBin.BaseLeft())
}

// Get ...
func (t *Tree) Get(b binmap.Bin) []byte {
	i := int(b - t.rootBin.BaseLeft())
	s := t.hash.Size()
	return t.digests[i*s : (i+1)*s]
}

func (t *Tree) setOrVerify(b binmap.Bin) (ok, verified bool) {
	d := t.Get(b)
	t.hash.Sum(d[:0])

	if t.parent != nil && t.parent.isVerified(b) {
		return bytes.Equal(d, t.parent.Get(b)), true
	}

	t.setVerified(b)
	return true, false
}

// Fill ...
func (t *Tree) Fill(b binmap.Bin, d []byte) bool {
	l := b.BaseLeft()
	r := b.BaseRight()

	for i := 0; i < int(b.BaseLength()); i++ {
		t.hash.Reset()
		t.hash.Write(d[i*t.chunkSize : (i+1)*t.chunkSize])
		if ok, _ := t.setOrVerify(l + binmap.Bin(i*2)); !ok {
			return false
		}
	}

	for i := uint64(1); i <= b.Layer(); i++ {
		l = l.Parent()
		r = r.Parent()
		w := binmap.Bin(1 << (i + 1))
		for j := l; j <= r; j += w {
			t.hash.Reset()
			t.hash.Write(t.Get(j.Left()))
			t.hash.Write(t.Get(j.Right()))
			if ok, _ := t.setOrVerify(j); !ok {
				return false
			}
		}
	}

	return true
}

// Verify ...
func (t *Tree) Verify(b binmap.Bin, d []byte) bool {
	if !t.Fill(b, d) {
		return false
	}

	for b != t.rootBin {
		t.hash.Reset()
		if b.IsLeft() {
			t.hash.Write(t.Get(b))
			t.hash.Write(t.Get(b.Sibling()))
		} else {
			t.hash.Write(t.Get(b.Sibling()))
			t.hash.Write(t.Get(b))
		}
		b = b.Parent()
		ok, verified := t.setOrVerify(b)
		if !ok {
			return false
		}
		if verified {
			return true
		}
	}

	return true
}
