package encoding

import (
	"bytes"
	"encoding/base64"
	"io/ioutil"
	"log"
)

// SwarmID ...
type SwarmID struct {
	PublicKey []byte
}

// NewSwarmID ...
func NewSwarmID(key []byte) *SwarmID {
	b := make([]byte, len(key))
	copy(b, key)
	return &SwarmID{PublicKey: b}
}

// LiveSignatureByteLength ...
func (s *SwarmID) LiveSignatureByteLength() int {
	// ECDSAP256SHA256
	return 64
}

// String ...
func (s *SwarmID) String() string {
	return base64.URLEncoding.EncodeToString(s.PublicKey)
}

// Binary ...
func (s *SwarmID) Binary() []byte {
	return s.PublicKey
}

// Compare ...
func (s *SwarmID) Compare(id *SwarmID) bool {
	return bytes.Compare(s.PublicKey, id.PublicKey) == 0
}

var idStash = &idStasher{}

type idStasher struct{}

// Store ...
func (i *idStasher) Store(id *SwarmID) error {
	log.Println("stashed id", id.String())
	return ioutil.WriteFile("/tmp/id-stash.bin", id.Binary(), 0644)
}

// Retrieve ...
func (i *idStasher) Retrieve() (*SwarmID, error) {
	d, err := ioutil.ReadFile("/tmp/id-stash.bin")
	if err != nil {
		return nil, err
	}

	id := NewSwarmID(d)
	log.Println("retrieved id", id.String())

	return id, nil
}
