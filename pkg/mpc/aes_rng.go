package mpc

import (
	"bufio"
	"crypto/aes"
	"crypto/cipher"
	"encoding/binary"
	"io"
)

// newAESRNGReader ...
func newAESRNGReader(seed []byte) (*aesRNGReader, error) {
	c, err := aes.NewCipher(seed)
	if err != nil {
		return nil, err
	}
	return &aesRNGReader{c, 0}, nil
}

// aesRNGReader ...
type aesRNGReader struct {
	c     cipher.Block
	state uint64
}

// Size returns the number of bytes available per read
func (r *aesRNGReader) Size() int {
	return r.c.BlockSize()
}

// Read implements io.Reader
func (r *aesRNGReader) Read(b []byte) (int, error) {
	var t [16]byte
	binary.LittleEndian.PutUint64(t[:], r.state)
	r.state++
	binary.LittleEndian.PutUint64(t[8:], r.state)
	r.state++
	r.c.Encrypt(b[:], t[:])
	return 16, nil
}

// NewAESRNG ...
func NewAESRNG(seed []byte) (*AESRNG, error) {
	r, err := newAESRNGReader(seed)
	if err != nil {
		return nil, err
	}
	rng := &AESRNG{
		r: bufio.NewReaderSize(r, r.Size()),
	}
	return rng, nil
}

// AESRNG ...
type AESRNG struct {
	r *bufio.Reader
}

// Read fills b with random bytes
func (a *AESRNG) Read(b []byte) (int, error) {
	return io.ReadFull(a.r, b)
}
