// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mpc

import (
	"crypto/aes"
	"encoding/binary"
	"errors"
	"io"
	"math"

	"github.com/bwesterb/go-ristretto"
)

// Conn ...
type Conn interface {
	io.ReadWriter
	Flush() error
}

// OTSender ...
type OTSender interface {
	Send(conn Conn, inputs [][2]Block, rng io.Reader) error
}

// OTReceiver ...
type OTReceiver interface {
	Receive(conn Conn, inputs []bool, rng io.Reader) ([]Block, error)
}

func readInt(r io.Reader) (int, error) {
	var b [4]byte
	if _, err := io.ReadFull(r, b[:]); err != nil {
		return 0, err
	}
	return int(binary.BigEndian.Uint32(b[:])), nil
}

func writeInt(w io.Writer, n int) error {
	var b [4]byte
	binary.BigEndian.PutUint32(b[:], uint32(n))
	_, err := w.Write(b[:])
	return err
}

func xorBytes(out, a, b []byte) {
	if len(out) != len(a) || len(a) != len(b) {
		panic("byte length mismatch")
	}
	for i := 0; i < len(out); i++ {
		out[i] = a[i] ^ b[i]
	}
}

func andBytes(out, a, b []byte) {
	if len(out) != len(a) || len(a) != len(b) {
		panic("byte length mismatch")
	}
	for i := 0; i < len(out); i++ {
		out[i] = a[i] & b[i]
	}
}

func boolsFromBytes(b []byte) []bool {
	s := make([]bool, len(b)*8)
	for i := 0; i < len(s); i++ {
		m := byte(1) << (7 - (i % 8))
		s[i] = b[i/8]&m != 0
	}
	return s
}

func bytesFromBools(b []bool) []byte {
	s := make([]byte, (len(b)+7)/8)
	for i, v := range b {
		if v {
			s[i/8] |= byte(1) << (7 - (i % 8))
		}
	}
	return s
}

var transposeMasks = [3]byte{
	0x55,
	0x33,
	0x0f,
}

// transposeMatrix transposes a bit-matrix using Eklundh's algorithm. m is
// expected to be in row major order with ncols bytes per row.
//
// TODO: this implementation has some limitations.
// * the longer dimension of a rectangular matrix must be a multiple of the
//   shorter dimension in bits.
// * the shorter of a rectangular and both dimensions of a square matrix must
//   be powers of 2.
func transposeMatrix(m []byte, nrows, ncols int) {
	width := 1

	for i := 0; i < 3; i++ {
		for r := 0; r < nrows; r += width * 2 {
			for c := 0; c < ncols; c++ {
				for j := 0; j < width; j++ {
					mi0 := c + (r+j)*ncols
					mi1 := c + (r+width+j)*ncols

					m0 := transposeMasks[i]
					m1 := ^m0

					t := m[mi0]
					m[mi0] = (m[mi0] & m1) | ((m[mi1] & m1) >> width)
					m[mi1] = (m[mi1] & m0) | ((t & m0) << width)
				}
			}
		}
		width *= 2
	}

	for width < nrows && width < ncols*8 {
		for r := 0; r < nrows; r += width * 2 {
			for c := width / 8; c < ncols; c += width / 4 {
				for j := 0; j < width; j++ {
					for k := 0; k < width/8; k++ {
						mi0 := c + k + (r+j)*ncols
						mi1 := c + k + (r+width+j)*ncols - width/8
						m[mi0], m[mi1] = m[mi1], m[mi0]
					}
				}
			}
		}
		width *= 2
	}

	if nrows == ncols*8 {
		return
	}

	t := make([]byte, len(m))
	copy(t, m)
	for i := 0; i < len(m); i++ {
		m[i] = 0
	}

	if nrows < ncols*8 {
		for r := 0; r < nrows; r++ {
			for c := 0; c < ncols; c += nrows / 8 {
				ti := c + r*ncols
				mi := r*nrows/8 + c*nrows
				copy(m[mi:], t[ti:ti+nrows/8])
			}
		}
	} else {
		for i := 0; i < nrows/(ncols*8); i++ {
			for j := 0; j < ncols*8; j++ {
				ti := i*ncols*ncols*8 + j*ncols
				mi := i*ncols + j*nrows/8
				copy(m[mi:], t[ti:ti+ncols])
			}
		}
	}
}

func padMatrix(ncols, nrows int) int {
	if ncols < 8 {
		return 8
	}

	c := int(math.Pow(2, math.Ceil(math.Log2(float64(ncols)))))
	if c < nrows {
		return c
	}

	if ncols%nrows != 0 {
		ncols += nrows - ncols%nrows
	}
	return ncols
}

func newRNG(seed []byte) (io.Reader, error) {
	return NewAESRNG(seed)
}

// Errors ...
var (
	ErrInvalidPoint = errors.New("bytes do not encode a valid point")
)

func readRistrettoPoint(r io.Reader, p *ristretto.Point) error {
	var b [32]byte
	if _, err := io.ReadFull(r, b[:]); err != nil {
		return err
	}

	if !p.SetBytes(&b) {
		return ErrInvalidPoint
	}
	return nil
}

func hashPoint(dst []byte, tweak uint64, pt *ristretto.Point) {
	b, _ := aes.NewCipher(pt.Bytes())
	src := blockFromUint(tweak)
	b.Encrypt(dst, src[:])
}

func ristrettoPointFromRNG(rng io.Reader) *ristretto.Point {
	var buf [32]byte
	if _, err := rng.Read(buf[:]); err != nil {
		panic(err)
	}
	p := new(ristretto.Point)
	return p.SetElligator(&buf)
}

func ristrettoScalarFromRNG(rng io.Reader) *ristretto.Scalar {
	var buf [64]byte
	if _, err := rng.Read(buf[:]); err != nil {
		panic(err)
	}
	s := new(ristretto.Scalar)
	return s.SetReduced(&buf)
}
