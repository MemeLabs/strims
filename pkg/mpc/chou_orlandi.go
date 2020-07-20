package mpc

import (
	"io"

	"github.com/bwesterb/go-ristretto"
)

// NewChaoOrlandiSender ...
func NewChaoOrlandiSender(conn Conn, rng io.Reader) (*ChaoOrlandiSender, error) {
	y := ristrettoScalarFromRNG(rng)
	s := new(ristretto.Point).ScalarMultBase(y)
	if _, err := conn.Write(s.Bytes()); err != nil {
		return nil, err
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}
	return &ChaoOrlandiSender{y, s}, nil
}

// ChaoOrlandiSender oblivious transfer sender
type ChaoOrlandiSender struct {
	y *ristretto.Scalar
	s *ristretto.Point
}

// Send values
func (o *ChaoOrlandiSender) Send(
	conn Conn,
	inputs [][2]Block,
	rng io.Reader,
) error {
	ys := new(ristretto.Point).ScalarMult(o.s, o.y)
	ks := make([][2]Block, len(inputs))
	r := new(ristretto.Point)
	for i := range inputs {
		if err := readRistrettoPoint(conn, r); err != nil {
			return err
		}
		r.ScalarMult(r, o.y)
		hashPoint(ks[i][0][:], uint64(i), r)
		hashPoint(ks[i][1][:], uint64(i), r.Sub(r, ys))
	}
	var c Block
	for i := range inputs {
		xorBytes(c[:], ks[i][0][:], inputs[i][0][:])
		if _, err := conn.Write(c[:]); err != nil {
			return err
		}
		xorBytes(c[:], ks[i][1][:], inputs[i][1][:])
		if _, err := conn.Write(c[:]); err != nil {
			return err
		}
	}
	if err := conn.Flush(); err != nil {
		return err
	}
	return nil
}

// NewChaoOrlandiReceiver ...
func NewChaoOrlandiReceiver(conn Conn, rng io.Reader) (*ChaoOrlandiReceiver, error) {
	p := new(ristretto.Point)
	if err := readRistrettoPoint(conn, p); err != nil {
		return nil, err
	}
	s := new(ristretto.ScalarMultTable)
	s.Compute(p)
	return &ChaoOrlandiReceiver{s}, nil
}

// ChaoOrlandiReceiver oblivious transfer receiver
type ChaoOrlandiReceiver struct {
	s *ristretto.ScalarMultTable
}

// Receive values
func (o *ChaoOrlandiReceiver) Receive(
	conn Conn,
	inputs []bool,
	rng io.Reader,
) ([]Block, error) {
	zero := new(ristretto.Point).ScalarMultTable(o.s, new(ristretto.Scalar).SetZero())
	one := new(ristretto.Point).ScalarMultTable(o.s, new(ristretto.Scalar).SetOne())

	var xb [64]byte
	x := new(ristretto.Scalar)
	var c *ristretto.Point
	r := new(ristretto.Point)
	ks := make([]Block, len(inputs))
	for i, b := range inputs {
		if _, err := rng.Read(xb[:]); err != nil {
			return nil, err
		}
		x.SetReduced(&xb)
		c = zero
		if b {
			c = one
		}
		r.Add(c, r.ScalarMultBase(x))
		if _, err := conn.Write(r.Bytes()); err != nil {
			return nil, err
		}
		hashPoint(ks[i][:], uint64(i), r.ScalarMultTable(o.s, x))
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}

	var c0, c1 Block
	rs := make([]Block, len(inputs))
	for i, b := range inputs {
		if _, err := io.ReadFull(conn, c0[:]); err != nil {
			return nil, err
		}
		if _, err := io.ReadFull(conn, c1[:]); err != nil {
			return nil, err
		}
		if b {
			xorBytes(rs[i][:], ks[i][:], c1[:])
		} else {
			xorBytes(rs[i][:], ks[i][:], c0[:])
		}
	}

	return rs, nil
}
