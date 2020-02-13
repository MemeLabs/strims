package mpc

import (
	"crypto/aes"
	"encoding/binary"
	"errors"
	"math"

	"lukechampine.com/uint128"
)

// errors
var (
	ErrInvalidCuckooSetSize    = errors.New("invalid cuckoo set size")
	ErrInvalidCuckooParameters = errors.New("invalid cuckoo hash parameters")
	ErrCuckooHashMapFull       = errors.New("cuckoo hash full")
)

var cuckooMask = Block{0x0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}

const cuckooNIters = 1000

func computeNBins(n, nhashes int) (r int, err error) {
	// Numbers taken from <https://thomaschneider.de/papers/PSZ18.pdf>, §3.2.2.
	if nhashes == 3 {
		if n < 1<<27 {
			// 1.27 ?
			r = int(math.Ceil((1.37 * float64(n))))
		} else {
			r = int(math.Ceil((1.62 * float64(n))))
		}
	} else if nhashes == 4 {
		r = int(math.Ceil((1.09 * float64(n))))
	} else if nhashes == 5 {
		r = int(math.Ceil((1.05 * float64(n))))
	} else {
		err = ErrInvalidCuckooParameters
	}
	return
}

func computeMaskSize(n int) (r int, err error) {
	// Numbers taken from <https://eprint.iacr.org/2016/799>, Table 2 (the `v`
	// column).
	if n <= 1<<8 {
		r = 7
	} else if n <= 1<<12 {
		r = 8
	} else if n <= 1<<16 {
		r = 9
	} else if n <= 1<<20 {
		r = 10
	} else if n <= 1<<24 {
		r = 11
	} else if n <= 1<<28 {
		r = 12
	} else {
		err = ErrInvalidCuckooSetSize
	}
	return
}

func cuckooHashBin(hash Block, hidx, nbins int) int {
	if hidx < 3 {
		var b [4]byte
		copy(b[:], hash[hidx*4:])
		return int(binary.LittleEndian.Uint32(b[:]) % uint32(nbins))
	}

	c, _ := aes.NewCipher(hash[:])
	b := blockFromUint(uint64(hidx))
	var h Block
	c.Encrypt(h[:], b[:])
	_, n := uint128.FromBytes(h[:]).QuoRem64(uint64(nbins))
	return int(n)
}

// NewCuckooHashMap ...
func NewCuckooHashMap(inputs []Block, nhashes int) (*CuckooHashMap, error) {
	nbins, err := computeNBins(len(inputs), nhashes)
	if err != nil {
		return nil, err
	}

	c := &CuckooHashMap{
		make([]*CuckooItem, nbins),
		nbins,
		nhashes,
	}

	for i, input := range inputs {
		if err := c.Hash(input, i); err != nil {
			return nil, err
		}
	}

	return c, nil
}

// CuckooHashMap ...
type CuckooHashMap struct {
	items   []*CuckooItem
	nbins   int
	nhashes int
}

// CuckooItem ...
type CuckooItem struct {
	entry      Block
	inputIndex int
	hashIndex  int
}

// Cap ...
func (c *CuckooHashMap) Cap() int {
	return c.nbins
}

// Hash ...
func (c *CuckooHashMap) Hash(input Block, idx int) error {
	item := &CuckooItem{input, idx, 0}
	for i := 0; i < cuckooNIters; i++ {
		andBytes(item.entry[:], item.entry[:], cuckooMask[:])
		b := blockFromUint(uint64(item.hashIndex))
		xorBytes(item.entry[:], item.entry[:], b[:])

		i := cuckooHashBin(item.entry, item.hashIndex, c.nbins)
		item, c.items[i] = c.items[i], item
		if item == nil {
			return nil
		}
		item.hashIndex = (item.hashIndex + 1) % c.nhashes
	}
	return ErrCuckooHashMapFull
}

// Values ...
func (c *CuckooHashMap) Values() (v []Block) {
	for _, item := range c.items {
		if item != nil {
			v = append(v, item.entry)
		}
	}
	return
}
