package hmac_drbg

import (
	"crypto/hmac"
	"hash"
)

// NewReader ...
func NewReader(h func() hash.Hash, seed []byte) *Reader {
	size := h().Size()
	r := &Reader{
		h:    h,
		size: size,
		a:    make([]byte, len(seed)),
		k:    make([]byte, size),
		v:    make([]byte, size),
	}
	copy(r.a, seed)
	for i := range r.v {
		r.v[i] = 0x10
	}
	return r
}

// Reader ...
type Reader struct {
	h    func() hash.Hash
	size int
	a    []byte
	k    []byte
	v    []byte
}

// Size returns the number of bytes available per read
func (r *Reader) Size() int {
	return r.size
}

func (r *Reader) update() {
	t := make([]byte, r.size+1+len(r.a))

	h := hmac.New(r.h, r.k)
	copy(t, r.v)
	copy(t[r.size+1:], r.a)
	h.Write(t)
	kTemp := h.Sum(nil)

	h = hmac.New(r.h, kTemp)
	h.Write(r.v)
	vTemp := h.Sum(nil)

	h = hmac.New(r.h, kTemp)
	copy(t, vTemp)
	t[r.size] = 1
	copy(t[r.size+1:], r.a)
	h.Write(t)
	r.k = h.Sum(r.k[:0])

	h = hmac.New(r.h, r.k)
	h.Write(vTemp)
	r.v = h.Sum(r.v[:0])
}

func (r *Reader) Read(b []byte) (n int, err error) {
	r.update()

	h := hmac.New(r.h, r.k)
	h.Write(r.v)
	n = copy(b, h.Sum(nil))

	return
}
