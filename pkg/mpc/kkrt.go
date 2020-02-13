package mpc

import (
	"io"
)

// NewKKRTSender ...
func NewKKRTSender(c Conn, ot OTReceiver, rng io.Reader) (*KKRTSender, error) {
	var sb [64]byte
	rng.Read(sb[:])
	s := boolsFromBytes(sb[:])
	var seeds [4]Block
	for i := 0; i < 4; i++ {
		rng.Read(seeds[i][:])
	}
	keys, err := cointossSend(c, seeds[:])
	if err != nil {
		return nil, err
	}
	code, err := NewPseudorandomCode(keys[0], keys[1], keys[2], keys[3])
	if err != nil {
		return nil, err
	}

	ks, err := ot.Receive(c, s, rng)
	if err != nil {
		return nil, err
	}
	rngs := make([]io.Reader, len(ks))
	for i, k := range ks {
		rngs[i], err = newRNG(k[:])
		if err != nil {
			return nil, err
		}
	}
	return &KKRTSender{s, sb, code, rngs}, nil
}

// KKRTSender ...
type KKRTSender struct {
	s    []bool
	sb   [64]byte
	code *PseudorandomCode
	rngs []io.Reader
}

// Send ...
func (k *KKRTSender) Send(
	conn Conn,
	m int,
	rng io.Reader,
) ([]Block512, error) {
	ncols := 512
	nrows := m
	if nrows%16 != 0 {
		nrows += 16 - nrows%16
	}
	nrows = padMatrix(nrows, ncols)

	t0 := make([]byte, nrows/8)
	t1 := make([]byte, nrows/8)
	qs := make([]byte, nrows*ncols/8)
	for i, b := range k.s {
		q := qs[i*nrows/8 : (i+1)*nrows/8]
		k.rngs[i].Read(q)
		if _, err := io.ReadFull(conn, t0); err != nil {
			return nil, err
		}
		if _, err := io.ReadFull(conn, t1); err != nil {
			return nil, err
		}
		t := t0
		if b {
			t = t1
		}
		xorBytes(q, q, t)
	}
	transposeMatrix(qs, ncols, nrows/8)

	seeds := make([]Block512, m)
	for i := 0; i < m; i++ {
		copy(seeds[i][:], qs[i*ncols/8:])
	}
	return seeds[:m], nil
}

// Compute ...
func (k *KKRTSender) Compute(seed Block512, src Block) (dst Block512) {
	k.Encode(&dst, src)
	xorBytes(dst[:], dst[:], seed[:])
	return
}

// Encode ...
func (k *KKRTSender) Encode(dst *Block512, src Block) {
	k.code.Encode(dst, src)
	andBytes(dst[:], dst[:], k.sb[:])
}

// NewKKRTReceiver ...
func NewKKRTReceiver(c Conn, ot OTSender, rng io.Reader) (*KKRTReceiver, error) {
	var seeds [4]Block
	for i := 0; i < 4; i++ {
		rng.Read(seeds[i][:])
	}
	keys, err := cointossReceive(c, seeds[:])
	if err != nil {
		return nil, err
	}
	code, err := NewPseudorandomCode(keys[0], keys[1], keys[2], keys[3])
	if err != nil {
		return nil, err
	}

	ks := make([][2]Block, 512)
	for i := 0; i < 512; i++ {
		rng.Read(ks[i][0][:])
		rng.Read(ks[i][1][:])
	}
	if err := ot.Send(c, ks, rng); err != nil {
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
	return &KKRTReceiver{code, rngs}, nil
}

// KKRTReceiver ...
type KKRTReceiver struct {
	code *PseudorandomCode
	rngs [][2]io.Reader
}

// Receive ...
func (k *KKRTReceiver) Receive(
	conn Conn,
	inputs []Block,
	rng io.Reader,
) ([]Block512, error) {
	m := len(inputs)
	ncols := 512
	nrows := m
	if nrows%16 != 0 {
		nrows += 16 - nrows%16
	}
	nrows = padMatrix(nrows, ncols)

	t0s := make([]byte, nrows*ncols/8)
	rng.Read(t0s)
	out := make([]Block512, nrows)
	for i := 0; i < nrows; i++ {
		copy(out[i][:], t0s[i*ncols/8:])
	}
	t1s := make([]byte, nrows*ncols/8)
	copy(t1s, t0s)

	var c Block512
	for i, input := range inputs {
		t1 := t1s[i*ncols/8 : (i+1)*ncols/8]
		k.code.Encode(&c, input)
		xorBytes(t1, t1, c[:])
	}
	transposeMatrix(t0s, nrows, ncols/8)
	transposeMatrix(t1s, nrows, ncols/8)

	t := make([]byte, nrows/8)
	for i, rngs := range k.rngs {
		t0 := t0s[i*nrows/8 : (i+1)*nrows/8]
		rngs[0].Read(t)
		xorBytes(t, t, t0)
		if _, err := conn.Write(t); err != nil {
			return nil, err
		}
		t1 := t1s[i*nrows/8 : (i+1)*nrows/8]
		rngs[1].Read(t)
		xorBytes(t, t, t1)
		if _, err := conn.Write(t); err != nil {
			return nil, err
		}
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}
	return out[:m], nil
}
