// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mpc

import (
	"io"
)

// NewALSZSender ...
func NewALSZSender(
	conn Conn,
	ot OTReceiver,
	rng io.Reader,
) (*ALSZSender, error) {
	var sb [16]byte
	if _, err := rng.Read(sb[:]); err != nil {
		return nil, err
	}
	s := boolsFromBytes(sb[:])
	ks, err := ot.Receive(conn, s, rng)
	if err != nil {
		return nil, err
	}
	rngs := make([]io.Reader, len(s))
	for i, k := range ks {
		rngs[i], err = newRNG(k[:])
		if err != nil {
			return nil, err
		}
	}
	r := &ALSZSender{
		ot:   ot,
		s:    s,
		sb:   sb,
		rngs: rngs,
		hash: fixedKeyAESHash,
	}
	return r, nil
}

// ALSZSender ...
type ALSZSender struct {
	ot   OTReceiver
	s    []bool
	sb   [16]byte
	rngs []io.Reader
	hash *AESHash
}

// SendSetup ...
func (a *ALSZSender) SendSetup(
	conn Conn,
	m int,
) ([]byte, error) {
	nrows := 128
	ncols := padMatrix(m, nrows) / 8
	qs := make([]byte, nrows*ncols)
	u := make([]byte, ncols)
	zero := make([]byte, ncols)
	for i := range a.rngs {
		qi := i * ncols
		q := qs[qi : qi+ncols]
		if _, err := io.ReadFull(conn, u); err != nil {
			return nil, err
		}
		if _, err := a.rngs[i].Read(q); err != nil {
			return nil, err
		}
		if a.s[i] {
			xorBytes(q, q, u)
		} else {
			xorBytes(q, q, zero)
		}
	}
	transposeMatrix(qs, nrows, ncols)
	return qs, nil
}

// Send ...
func (a *ALSZSender) Send(
	conn Conn,
	inputs [][2]Block,
	rng io.Reader,
) error {
	m := len(inputs)
	qs, err := a.SendSetup(conn, m)
	if err != nil {
		return err
	}
	for i, input := range inputs {
		q := blockFromBytes(qs[i*16 : (i+1)*16])
		y0 := a.hash.CRHash(q)
		xorBytes(y0[:], y0[:], input[0][:])
		xorBytes(q[:], q[:], a.sb[:])
		y1 := a.hash.CRHash(q)
		xorBytes(y1[:], y1[:], input[1][:])
		if _, err := conn.Write(y0[:]); err != nil {
			return err
		}
		if _, err := conn.Write(y1[:]); err != nil {
			return err
		}
	}
	if err := conn.Flush(); err != nil {
		return err
	}
	return nil
}

// NewALSZReceiver ...
func NewALSZReceiver(
	conn Conn,
	ot OTSender,
	rng io.Reader,
) (r *ALSZReceiver, err error) {
	ks := make([][2]Block, 128)
	for i := range ks {
		if _, err := rng.Read(ks[i][0][:]); err != nil {
			return nil, err
		}
		if _, err := rng.Read(ks[i][1][:]); err != nil {
			return nil, err
		}
	}
	if err := ot.Send(conn, ks, rng); err != nil {
		return nil, err
	}

	rngs := make([][2]io.Reader, len(ks))
	for i, k := range ks {
		rngs[i][0], err = newRNG(k[0][:])
		if err != nil {
			return nil, err
		}
		rngs[i][1], err = newRNG(k[1][:])
		if err != nil {
			return nil, err
		}
	}
	r = &ALSZReceiver{
		ot:   ot,
		rngs: rngs,
		hash: fixedKeyAESHash,
	}
	return r, nil
}

// ALSZReceiver ...
type ALSZReceiver struct {
	ot   OTSender
	rngs [][2]io.Reader
	hash *AESHash
}

// ReceiveSetup ...
func (a *ALSZReceiver) ReceiveSetup(
	conn Conn,
	r []byte,
	m int,
) ([]byte, error) {
	nrows := 128
	ncols := padMatrix(m, nrows) / 8
	ts := make([]byte, nrows*ncols)
	g := make([]byte, ncols)
	r = append(r, make([]byte, ncols-len(r))...)
	for i := range a.rngs {
		t := ts[i*ncols : (i+1)*ncols]
		if _, err := a.rngs[i][0].Read(t); err != nil {
			return nil, err
		}
		if _, err := a.rngs[i][1].Read(g); err != nil {
			return nil, err
		}
		xorBytes(g, g, t)
		xorBytes(g, g, r)
		if _, err := conn.Write(g); err != nil {
			return nil, err
		}
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}
	transposeMatrix(ts, nrows, ncols)
	return ts, nil
}

// Receive ...
func (a *ALSZReceiver) Receive(
	conn Conn,
	inputs []bool,
	rng io.Reader,
) ([]Block, error) {
	r := bytesFromBools(inputs)
	ts, err := a.ReceiveSetup(conn, r, len(inputs))
	if err != nil {
		return nil, err
	}

	out := make([]Block, len(inputs))
	for i, b := range inputs {
		t := ts[i*16 : (i+1)*16]
		var y0, y1 Block
		if _, err := io.ReadFull(conn, y0[:]); err != nil {
			return nil, err
		}
		if _, err := io.ReadFull(conn, y1[:]); err != nil {
			return nil, err
		}
		y := y0
		if b {
			y = y1
		}
		h := a.hash.CRHash(blockFromBytes(t))
		xorBytes(y[:], y[:], h[:])
		out[i] = y
	}
	return out, nil
}
