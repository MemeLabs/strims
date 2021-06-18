package integrity

import (
	"crypto/ed25519"
	"encoding/binary"

	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

// NewED25519Signer ...
func NewED25519Signer(key ed25519.PrivateKey) *ED25519Signer {
	return &ED25519Signer{key: key}
}

// ED25519Signer ...
type ED25519Signer struct {
	key ed25519.PrivateKey
}

// Sign ...
func (s *ED25519Signer) Sign(t timeutil.Time, p []byte) []byte {
	b := pool.Get(len(p) + 8)
	defer pool.Put(b)
	binary.BigEndian.PutUint64(*b, uint64(t.UnixNano()))
	copy((*b)[8:], p)

	return ed25519.Sign(s.key, *b)
}

// Size ...
func (s *ED25519Signer) Size() int {
	return ed25519.SignatureSize
}

// NewED25519Verifier ...
func NewED25519Verifier(key ed25519.PublicKey) *ED25519Verifier {
	return &ED25519Verifier{key: key}
}

// ED25519Verifier ...
type ED25519Verifier struct {
	key ed25519.PublicKey
}

// Verify ...
func (s *ED25519Verifier) Verify(t timeutil.Time, p []byte, sig []byte) bool {
	b := pool.Get(len(p) + 8)
	defer pool.Put(b)
	binary.BigEndian.PutUint64(*b, uint64(t.UnixNano()))
	copy((*b)[8:], p)

	return ed25519.Verify(s.key, *b, sig)
}

// Size ...
func (s *ED25519Verifier) Size() int {
	return ed25519.SignatureSize
}
