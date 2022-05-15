// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"bytes"
	"encoding/base32"
)

// SwarmID ...
type SwarmID []byte

var idEncoding = base32.StdEncoding.WithPadding(base32.NoPadding)

// DecodeSwarmID ...
func DecodeSwarmID(key string) (SwarmID, error) {
	s, err := idEncoding.DecodeString(key)
	return SwarmID(s), err
}

// NewSwarmID ...
func NewSwarmID(key []byte) SwarmID {
	b := make([]byte, len(key))
	copy(b, key)
	return SwarmID(b)
}

// String ...
func (s SwarmID) String() string {
	return idEncoding.EncodeToString(s)
}

// Equals ...
func (s SwarmID) Equals(o SwarmID) bool {
	return s.Compare(o) == 0
}

// Compare ...
func (s SwarmID) Compare(o SwarmID) int {
	return bytes.Compare([]byte(s), []byte(o))
}
