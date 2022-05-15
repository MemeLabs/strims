// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mpc

import (
	"bytes"
	"errors"
	"io"
)

func cointossSend(conn Conn, seeds []Block) (out []Block, err error) {
	var com Block
	for _, seed := range seeds {
		rng, err := newRNG(seed[:])
		if err != nil {
			return nil, err
		}
		if _, err := rng.Read(com[:]); err != nil {
			return nil, err
		}
		if _, err := conn.Write(com[:]); err != nil {
			return nil, err
		}
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}

	var s Block
	for _, seed := range seeds {
		if _, err := io.ReadFull(conn, s[:]); err != nil {
			return nil, err
		}
		var o Block
		xorBytes(o[:], seed[:], s[:])
		out = append(out, o)
	}

	for _, seed := range seeds {
		if _, err := conn.Write(seed[:]); err != nil {
			return nil, err
		}
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}
	return out, nil
}

func cointossReceive(conn Conn, seeds []Block) (out []Block, err error) {
	cs := make([]Block, len(seeds))
	for i := range seeds {
		if _, err := io.ReadFull(conn, cs[i][:]); err != nil {
			return nil, err
		}
	}

	for _, seed := range seeds {
		if _, err := conn.Write(seed[:]); err != nil {
			return nil, err
		}
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}

	var s, rc Block
	for i, seed := range seeds {
		if _, err := io.ReadFull(conn, s[:]); err != nil {
			return nil, err
		}
		rng, err := newRNG(s[:])
		if err != nil {
			return nil, err
		}
		if _, err := rng.Read(rc[:]); err != nil {
			return nil, err
		}
		if !bytes.Equal(cs[i][:], rc[:]) {
			return nil, errors.New("commitment check failed")
		}

		var o Block
		xorBytes(o[:], seed[:], s[:])
		out = append(out, o)
	}

	return out, nil
}
