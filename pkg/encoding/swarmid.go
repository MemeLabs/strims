package encoding

import (
	"bytes"
	"encoding/base32"
)

// SwarmID ...
type SwarmID []byte

// DecodeSwarmID ...
func DecodeSwarmID(key string) (SwarmID, error) {
	s, err := base32.StdEncoding.DecodeString(key)
	return SwarmID(s), err
}

// NewSwarmID ...
func NewSwarmID(key []byte) SwarmID {
	b := make([]byte, len(key))
	copy(b, key)
	return SwarmID(b)
}

// LiveSignatureByteLength ...
func (s SwarmID) LiveSignatureByteLength() int {
	// ECDSAP256SHA256
	return 64
}

// String ...
func (s SwarmID) String() string {
	return base32.StdEncoding.WithPadding(base32.NoPadding).EncodeToString(s)
}

// Binary ...
func (s SwarmID) Binary() []byte {
	return s
}

// Equals ...
func (s SwarmID) Equals(o SwarmID) bool {
	return s.Compare(o) == 0
}

// Compare ...
func (s SwarmID) Compare(o SwarmID) int {
	return bytes.Compare([]byte(s), []byte(o))
}
