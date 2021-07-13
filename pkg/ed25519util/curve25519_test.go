package ed25519util

import (
	"crypto/ed25519"
	"crypto/rand"
	"testing"

	"github.com/stretchr/testify/assert"
	"golang.org/x/crypto/curve25519"
)

func TestEd25519ToCurve25519(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(rand.Reader)

	var edwards25519Private [64]byte
	var edwards25519Public [32]byte
	copy(edwards25519Private[:], priv)
	copy(edwards25519Public[:], pub)

	var curve25519Public, curve25519Private [32]byte
	PrivateKeyToCurve25519(&curve25519Private, &edwards25519Private)
	PublicKeyToCurve25519(&curve25519Public, &edwards25519Public)

	var curve25519Public0 [32]byte
	curve25519.ScalarBaseMult(&curve25519Public0, &curve25519Private)
	assert.Equal(t, curve25519Public0, curve25519Public)
}
