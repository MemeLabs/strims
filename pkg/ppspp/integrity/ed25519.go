package integrity

import (
	"crypto/ed25519"
	"encoding/binary"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pool"
)

func NewED25519Signer(key ed25519.PrivateKey) *ED25519Signer {
	return &ED25519Signer{key: key}
}

type ED25519Signer struct {
	key ed25519.PrivateKey
}

func (s *ED25519Signer) Sign(t time.Time, p []byte) []byte {
	b := pool.Get(uint16(len(p) + 8))
	defer pool.Put(b)
	binary.BigEndian.PutUint64(*b, uint64(t.UnixNano()))
	copy((*b)[8:], p)

	return ed25519.Sign(s.key, *b)
}

func (s *ED25519Signer) Size() int {
	return ed25519.SignatureSize
}

func NewED25519Verifier(key ed25519.PublicKey) *ED25519Verifier {
	return &ED25519Verifier{key: key}
}

type ED25519Verifier struct {
	key ed25519.PublicKey
}

func (s *ED25519Verifier) Verify(t time.Time, p []byte, sig []byte) bool {
	b := pool.Get(uint16(len(p) + 8))
	defer pool.Put(b)
	binary.BigEndian.PutUint64(*b, uint64(t.UnixNano()))
	copy((*b)[8:], p)

	return ed25519.Verify(s.key, *b, sig)
}

func (s *ED25519Verifier) Size() int {
	return ed25519.SignatureSize
}
