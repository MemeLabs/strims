package mpc

import (
	"errors"
	"io"
)

// SPP sender ot padding
const SPP = 40

// NewKOSSender ...
func NewKOSSender(conn Conn, ot OTReceiver, rng io.Reader) (*KOSSender, error) {
	alsz, err := NewALSZSender(conn, ot, rng)
	if err != nil {
		return nil, err
	}
	kos := &KOSSender{
		conn: conn,
		rng:  rng,
		ot:   alsz,
	}
	return kos, nil
}

// ErrKOSConsistencyCheckFailed ...
var ErrKOSConsistencyCheckFailed = errors.New("consistency check failed")

// KOSSender ...
type KOSSender struct {
	conn Conn
	rng  io.Reader
	ot   *ALSZSender
}

// SendSetup ...
func (a *KOSSender) SendSetup(
	conn Conn,
	m int,
	rng io.Reader,
) ([]byte, error) {
	if m%8 != 0 {
		m = m + (8 - m%8)
	}
	ncols := m + 128 + SPP
	qs, err := a.ot.SendSetup(conn, ncols)
	if err != nil {
		return nil, err
	}
	var seed Block
	if _, err := rng.Read(seed[:]); err != nil {
		return nil, err
	}
	seeds, err := cointossSend(conn, []Block{seed})
	if err != nil {
		return nil, err
	}

	rng, err = newRNG(seeds[0][:])
	if err != nil {
		return nil, err
	}
	var check [2]Block
	var q, chi Block
	for i := 0; i < ncols; i++ {
		copy(q[:], qs[i*16:(i+1)*16])
		if _, err := rng.Read(chi[:]); err != nil {
			return nil, err
		}
		tmp := clmulBlock(q, chi)
		xorBytes(check[0][:], check[0][:], tmp[0][:])
		xorBytes(check[1][:], check[1][:], tmp[1][:])
	}

	var x Block
	var t [2]Block
	if _, err := io.ReadFull(conn, x[:]); err != nil {
		return nil, err
	}
	if _, err := io.ReadFull(conn, t[0][:]); err != nil {
		return nil, err
	}
	if _, err := io.ReadFull(conn, t[1][:]); err != nil {
		return nil, err
	}

	tmp := clmulBlock(x, a.ot.sb)
	xorBytes(check[0][:], check[0][:], tmp[0][:])
	xorBytes(check[1][:], check[1][:], tmp[1][:])
	if check != t {
		return nil, ErrKOSConsistencyCheckFailed
	}
	return qs, nil
}

// Send ...
func (a *KOSSender) Send(
	conn Conn,
	inputs [][2]Block,
	rng io.Reader,
) error {
	m := len(inputs)
	qs, err := a.SendSetup(conn, m, rng)
	if err != nil {
		return err
	}
	var q Block
	for i, input := range inputs {
		copy(q[:], qs[i*16:(i+1)*16])
		y0 := a.ot.hash.TCCRHash(blockFromUint(uint64(i)), q)
		xorBytes(y0[:], y0[:], input[0][:])
		xorBytes(q[:], q[:], a.ot.sb[:])
		y1 := a.ot.hash.TCCRHash(blockFromUint(uint64(i)), q)
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

// NewKOSReceiver ...
func NewKOSReceiver(conn Conn, ot OTSender, rng io.Reader) (*KOSReceiver, error) {
	alsz, err := NewALSZReceiver(conn, ot, rng)
	if err != nil {
		return nil, err
	}
	kos := &KOSReceiver{
		conn: conn,
		rng:  rng,
		ot:   alsz,
	}
	return kos, nil
}

// KOSReceiver ...
type KOSReceiver struct {
	conn Conn
	rng  io.Reader
	ot   *ALSZReceiver
}

// ReceiveSetup ...
func (a *KOSReceiver) ReceiveSetup(
	conn Conn,
	inputs []bool,
	rng io.Reader,
) ([]byte, error) {
	m := len(inputs)
	if m%8 != 0 {
		m += 8 - m%8
	}
	mp := m + 128 + SPP
	r := bytesFromBools(inputs)
	ext := make([]byte, (mp-m)/8)
	if _, err := rng.Read(ext); err != nil {
		return nil, err
	}
	r = append(r, ext...)

	ts, err := a.ot.ReceiveSetup(conn, r, mp)
	if err != nil {
		return nil, err
	}
	var seed Block
	if _, err := rng.Read(seed[:]); err != nil {
		return nil, err
	}
	seeds, err := cointossReceive(conn, []Block{seed})
	if err != nil {
		return nil, err
	}

	rng, err = newRNG(seeds[0][:])
	if err != nil {
		return nil, err
	}
	var t [2]Block
	var x, ti, chi, zero Block
	for i, xi := range boolsFromBytes(r) {
		copy(ti[:], ts[i*16:(i+1)*16])
		if _, err := rng.Read(chi[:]); err != nil {
			return nil, err
		}
		if xi {
			xorBytes(x[:], x[:], chi[:])
		} else {
			xorBytes(x[:], x[:], zero[:])
		}
		tmp := clmulBlock(ti, chi)
		xorBytes(t[0][:], t[0][:], tmp[0][:])
		xorBytes(t[1][:], t[1][:], tmp[1][:])
	}

	if _, err := conn.Write(x[:]); err != nil {
		return nil, err
	}
	if _, err := conn.Write(t[0][:]); err != nil {
		return nil, err
	}
	if _, err := conn.Write(t[1][:]); err != nil {
		return nil, err
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}
	return ts, nil
}

// Receive ...
func (a *KOSReceiver) Receive(
	conn Conn,
	inputs []bool,
	rng io.Reader,
) ([]Block, error) {
	qs, err := a.ReceiveSetup(conn, inputs, rng)
	if err != nil {
		return nil, err
	}
	out := make([]Block, len(inputs))
	var t, y0, y1 Block
	for i, b := range inputs {
		copy(t[:], qs[i*16:(i+1)*16])
		if _, err := io.ReadFull(conn, y0[:]); err != nil {
			return nil, err
		}
		if _, err := io.ReadFull(conn, y1[:]); err != nil {
			return nil, err
		}
		th := a.ot.hash.TCCRHash(blockFromUint(uint64(i)), t)
		if b {
			xorBytes(out[i][:], y1[:], th[:])
		} else {
			xorBytes(out[i][:], y0[:], th[:])
		}
	}
	return out, nil
}
