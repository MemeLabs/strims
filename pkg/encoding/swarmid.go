package encoding

import (
	"encoding/base64"
)

type SwarmID struct {
	PublicKey []byte
}

func NewSwarmID(b []byte) *SwarmID {
	return &SwarmID{PublicKey: b}
}

func (s *SwarmID) LiveSignatureByteLength() int {
	// ECDSAP256SHA256
	return 64
}

func (s *SwarmID) String() string {
	return base64.URLEncoding.EncodeToString(s.PublicKey)
}

func (s *SwarmID) Binary() []byte {
	return s.PublicKey
}
