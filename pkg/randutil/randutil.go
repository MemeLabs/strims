// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

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

func Int63n(max int64) (int64, error) {
	v, err := rand.Int(rand.Reader, big.NewInt(max))
	if err != nil {
		return 0, err
	}
	return v.Int64(), nil
}

// Uint64 ...
func Uint64() (uint64, error) {
	var t [8]byte
	if _, err := rand.Read(t[:]); err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint64(t[:]), nil
}
