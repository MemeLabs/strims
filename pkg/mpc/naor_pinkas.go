package mpc

// Implementation of the Naor-Pinkas oblivious transfer protocol (cf.
// <https://dl.acm.org/citation.cfm?id=365502>).

import (
	"io"

	"github.com/bwesterb/go-ristretto"
)

// NaorPinkasSender oblivious transfer sender
type NaorPinkasSender struct{}

// Send values
func (o *NaorPinkasSender) Send(
	conn Conn,
	inputs [][2]Block,
	rng io.Reader,
) error {
	cs := make([]*ristretto.Point, len(inputs))
	pks := make([][2]*ristretto.Point, len(inputs))
	for i := range inputs {
		cs[i] = ristrettoPointFromRNG(rng)
		if _, err := conn.Write(cs[i].Bytes()); err != nil {
			return err
		}
	}
	if err := conn.Flush(); err != nil {
		return err
	}
	for i := range inputs {
		pk0 := new(ristretto.Point)
		if err := readRistrettoPoint(conn, pk0); err != nil {
			return err
		}
		pks[i] = [2]*ristretto.Point{pk0, new(ristretto.Point).Sub(cs[i], pk0)}
	}
	for i := range inputs {
		var e01, e11, h Block
		r0 := ristrettoScalarFromRNG(rng)
		r1 := ristrettoScalarFromRNG(rng)
		hashPoint(h[:], uint64(i), new(ristretto.Point).ScalarMult(pks[i][0], r0))
		xorBytes(e01[:], inputs[i][0][:], h[:])
		hashPoint(h[:], uint64(i), new(ristretto.Point).ScalarMult(pks[i][1], r1))
		xorBytes(e11[:], inputs[i][1][:], h[:])

		e00 := new(ristretto.Point).ScalarMultBase(r0)
		e10 := new(ristretto.Point).ScalarMultBase(r1)
		if _, err := conn.Write(e00.Bytes()); err != nil {
			return err
		}
		if _, err := conn.Write(e01[:]); err != nil {
			return err
		}
		if _, err := conn.Write(e10.Bytes()); err != nil {
			return err
		}
		if _, err := conn.Write(e11[:]); err != nil {
			return err
		}
	}
	if err := conn.Flush(); err != nil {
		return err
	}
	return nil
}

// NaorPinkasReceiver oblivious transfer receiver
type NaorPinkasReceiver struct{}

// Receive values
func (o *NaorPinkasReceiver) Receive(
	conn Conn,
	inputs []bool,
	rng io.Reader,
) ([]Block, error) {
	cs := make([]*ristretto.Point, len(inputs))
	ks := make([]*ristretto.Scalar, len(inputs))
	rs := make([]Block, len(inputs))

	for i := range inputs {
		cs[i] = new(ristretto.Point)
		if err := readRistrettoPoint(conn, cs[i]); err != nil {
			return nil, err
		}
	}
	for i := range inputs {
		ks[i] = ristrettoScalarFromRNG(rng)
		pk := new(ristretto.Point).ScalarMultBase(ks[i])
		if !inputs[i] {
			if _, err := conn.Write(pk.Bytes()); err != nil {
				return nil, err
			}
		} else {
			if _, err := conn.Write(new(ristretto.Point).Sub(cs[i], pk).Bytes()); err != nil {
				return nil, err
			}
		}
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}
	for i := range inputs {
		e00 := new(ristretto.Point)
		e10 := new(ristretto.Point)
		var e01, e11 Block
		if err := readRistrettoPoint(conn, e00); err != nil {
			return nil, err
		}
		if _, err := io.ReadFull(conn, e01[:]); err != nil {
			return nil, err
		}
		if err := readRistrettoPoint(conn, e10); err != nil {
			return nil, err
		}
		if _, err := io.ReadFull(conn, e11[:]); err != nil {
			return nil, err
		}

		var e0 *ristretto.Point
		var e1 Block
		if !inputs[i] {
			e0, e1 = e00, e01
		} else {
			e0, e1 = e10, e11
		}

		var h Block
		hashPoint(h[:], uint64(i), new(ristretto.Point).ScalarMult(e0, ks[i]))
		xorBytes(rs[i][:], h[:], e1[:])
	}
	return rs, nil
}
