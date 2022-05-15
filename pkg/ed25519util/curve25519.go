// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ed25519util

import (
	"crypto/sha512"

	"github.com/bwesterb/go-ristretto/edwards25519"
)

// PrivateKeyToCurve25519 converts an ed25519 private key to a curve25519
// private key
func PrivateKeyToCurve25519(curve25519Private *[32]byte, privateKey *[64]byte) {
	h := sha512.New()
	h.Write(privateKey[:32])
	digest := h.Sum(nil)

	digest[0] &= 248
	digest[31] &= 127
	digest[31] |= 64

	copy(curve25519Private[:], digest)
}

// PublicKeyToCurve25519 converts an ed25519 public key to a curve25519 public
// key according to rfc 7748 section 4.1
func PublicKeyToCurve25519(curve25519Public, publicKey *[32]byte) {
	var t0, t1, x, y, z edwards25519.FieldElement
	y.SetBytes(publicKey)
	z.SetOne()

	t0.Add(&z, &y)
	t1.Sub(&z, &y)
	t1.Inverse(&t1)
	x.Mul(&t0, &t1)

	x.BytesInto(curve25519Public)
}
