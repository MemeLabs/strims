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
		conn.Write(com[:])
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
		conn.Write(seed[:])
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
		conn.Write(seed[:])
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
		rng.Read(rc[:])
		if bytes.Compare(cs[i][:], rc[:]) != 0 {
			return nil, errors.New("commitment check failed")
		}

		var o Block
		xorBytes(o[:], seed[:], s[:])
		out = append(out, o)
	}

	return out, nil
}
