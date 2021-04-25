package randutil

import (
	"crypto/rand"
	"encoding/binary"
	"math/big"
)

// Uint16n ...
func Uint16n(max uint16) (uint16, error) {
	v, err := rand.Int(rand.Reader, big.NewInt(int64(max)))
	if err != nil {
		return 0, err
	}
	return uint16(v.Int64()), nil
}

// Uint64 ...
func Uint64() (uint64, error) {
	var t [8]byte
	if _, err := rand.Read(t[:]); err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint64(t[:]), nil
}

// MustUint64 ...
func MustUint64() uint64 {
	v, err := Uint64()
	if err != nil {
		panic(err)
	}
	return v
}
