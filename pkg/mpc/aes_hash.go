package mpc

import (
	"crypto/aes"
	"crypto/cipher"
)

var fixedKeyAESHash = NewAESHash(Block{0x52, 0xf2, 0x71, 0x98, 0xf2, 0xea, 0xf3, 0x4f, 0x3c, 0x93, 0xbe, 0xc1, 0x17, 0x21, 0x91, 0x3b})

// NewAESHash ...
func NewAESHash(key Block) *AESHash {
	c, _ := aes.NewCipher(key[:])
	return &AESHash{c}
}

// AESHash ...
type AESHash struct {
	c cipher.Block
}

// CRHash ...
func (a *AESHash) CRHash(x Block) (r Block) {
	a.c.Encrypt(r[:], x[:])
	xorBytes(r[:], r[:], x[:])
	return
}

// TCCRHash ...
func (a *AESHash) TCCRHash(i, x Block) (r Block) {
	var t, y, z Block
	a.c.Encrypt(y[:], x[:])
	xorBytes(t[:], y[:], i[:])
	a.c.Encrypt(z[:], t[:])
	xorBytes(r[:], y[:], z[:])
	return
}
