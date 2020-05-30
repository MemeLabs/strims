package vpn

import (
	"crypto/rand"
	"encoding/binary"
)

func randUint16(max uint16) (uint16, error) {
	var t [2]byte
	if _, err := rand.Read(t[:]); err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint16(t[:]) % max, nil
}

func randUint64() (uint64, error) {
	var t [8]byte
	if _, err := rand.Read(t[:]); err != nil {
		return 0, err
	}
	return binary.LittleEndian.Uint64(t[:]), nil
}
