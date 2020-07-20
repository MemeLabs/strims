package mpc

import (
	"bytes"
	"crypto/sha256"
	"encoding/binary"
	"io"
	"math/rand"

	"github.com/emirpasic/gods/trees/redblacktree"
)

const pszNHashes = 3

// NewPSZSender ...
func NewPSZSender(oprf *KKRTSender) (*PSZSender, error) {
	return &PSZSender{oprf}, nil
}

// PSZSender ...
type PSZSender struct {
	oprf *KKRTSender
}

// Send ...
func (p *PSZSender) Send(
	conn Conn,
	inputs [][]byte,
	rng io.Reader,
) error {
	n := len(inputs)
	if err := writeInt(conn, n); err != nil {
		return err
	}
	if err := conn.Flush(); err != nil {
		return err
	}
	pn, err := readInt(conn)
	if err != nil {
		return err
	}
	if n < pn {
		n = pn
	}

	var seed Block
	if _, err := rng.Read(seed[:]); err != nil {
		return err
	}
	keys, err := cointossSend(conn, []Block{seed})
	if err != nil {
		return err
	}
	digests, err := compressAndHashInputs(inputs, keys[0])
	if err != nil {
		return err
	}

	nbins, err := computeNBins(pn, pszNHashes)
	if err != nil {
		return err
	}
	maskSize, err := computeMaskSize(n)
	if err != nil {
		return err
	}
	seeds, err := p.oprf.Send(conn, nbins, rng)
	if err != nil {
		return err
	}

	indices := make([]int, len(inputs))
	for i := range inputs {
		indices[i] = i
	}

	var input Block
	var encoded Block512
	for i := 0; i < pszNHashes; i++ {
		if err := shuffleInts(indices, rng); err != nil {
			return err
		}

		hidx := blockFromUint(uint64(i))
		for _, j := range indices {
			bin := cuckooHashBin(digests[j], i, nbins)
			xorBytes(input[:], digests[j][:], hidx[:])
			p.oprf.Encode(&encoded, input)
			xorBytes(encoded[:], encoded[:], seeds[bin][:])
			if _, err := conn.Write(encoded[:maskSize]); err != nil {
				return err
			}
		}
	}
	if err := conn.Flush(); err != nil {
		return err
	}
	return nil
}

// SendPayloads ...
func (p *PSZSender) SendPayloads(
	conn Conn,
	inputs [][]byte,
	rng io.Reader,
) ([]Block, error) {
	return nil, nil
}

// NewPSZReceiver ...
func NewPSZReceiver(oprf *KKRTReceiver) (*PSZReceiver, error) {
	return &PSZReceiver{oprf}, nil
}

// PSZReceiver ...
type PSZReceiver struct {
	oprf *KKRTReceiver
}

// Receive ...
func (p *PSZReceiver) Receive(
	conn Conn,
	inputs [][]byte,
	rng io.Reader,
) ([][]byte, error) {
	n := len(inputs)
	pn, err := readInt(conn)
	if err != nil {
		return nil, err
	}
	if err := writeInt(conn, n); err != nil {
		return nil, err
	}
	if err := conn.Flush(); err != nil {
		return nil, err
	}
	if n < pn {
		n = pn
	}

	maskSize, err := computeMaskSize(n)
	if err != nil {
		return nil, err
	}

	c, outputs, err := p.performOPRFS(conn, inputs, rng)
	if err != nil {
		return nil, err
	}

	hs := make([]map[[12]byte]struct{}, pszNHashes)
	var prefix [12]byte
	for i := range hs {
		h := map[[12]byte]struct{}{}
		hs[i] = h
		for i := 0; i < pn; i++ {
			if _, err := io.ReadFull(conn, prefix[:maskSize]); err != nil {
				return nil, err
			}
			h[prefix] = struct{}{}
		}
	}

	intersection := [][]byte{}
	for i, item := range c.items {
		if item == nil {
			continue
		}
		copy(prefix[:], outputs[i][:maskSize])
		if _, ok := hs[item.hashIndex][prefix]; ok {
			intersection = append(intersection, inputs[item.inputIndex])
		}
	}

	return intersection, nil
}

// ReceivePayloads ...
func (p *PSZReceiver) ReceivePayloads(
	conn Conn,
	inputs [][]byte,
	rng io.Reader,
) (*redblacktree.Tree, error) {
	t := redblacktree.NewWith(bytesComparator)

	return t, nil
}

func (p *PSZReceiver) performOPRFS(
	conn Conn,
	inputs [][]byte,
	rng io.Reader,
) (*CuckooHashMap, []Block512, error) {
	var seed Block
	if _, err := rng.Read(seed[:]); err != nil {
		return nil, nil, err
	}
	keys, err := cointossReceive(conn, []Block{seed})
	if err != nil {
		return nil, nil, err
	}
	digests, err := compressAndHashInputs(inputs, keys[0])
	if err != nil {
		return nil, nil, err
	}

	c, err := NewCuckooHashMap(digests, pszNHashes)
	if err != nil {
		return nil, nil, err
	}

	oprfInputs := make([]Block, c.nbins)
	for i, item := range c.items {
		if item != nil {
			oprfInputs[i] = item.entry
		}
	}
	oprfOutputs, err := p.oprf.Receive(conn, oprfInputs, rng)
	if err != nil {
		return nil, nil, err
	}
	return c, oprfOutputs, nil
}

func bytesComparator(a, b interface{}) int {
	return bytes.Compare(a.([]byte), b.([]byte))
}

func compressAndHashInputs(inputs [][]byte, key Block) ([]Block, error) {
	hash := sha256.New()
	aes := NewAESHash(key)
	digests := make([]Block, len(inputs))
	for i, input := range inputs {
		var tmp Block
		if len(input) < 16 {
			copy(tmp[:], input)
		} else {
			hash.Reset()
			if _, err := hash.Write(input); err != nil {
				return nil, err
			}
			copy(tmp[:], hash.Sum(nil))
		}
		tmp = aes.CRHash(tmp)
		andBytes(digests[i][:], tmp[:], cuckooMask[:])
	}
	return digests, nil
}

func shuffleInts(ns []int, rng io.Reader) error {
	var b [8]byte
	if _, err := rng.Read(b[:]); err != nil {
		return err
	}
	seed := int64(binary.BigEndian.Uint64(b[:]) & (1<<63 - 1))
	r := rand.New(rand.NewSource(seed))
	r.Shuffle(len(ns), func(i int, j int) {
		ns[i], ns[j] = ns[j], ns[i]
	})
	return nil
}
