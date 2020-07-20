package mpc

import (
	"crypto/aes"
	"crypto/cipher"
)

// NewPseudorandomCode ...
func NewPseudorandomCode(k0, k1, k2, k3 Block) (*PseudorandomCode, error) {
	cipher0, err := aes.NewCipher(k0[:])
	if err != nil {
		return nil, err
	}
	cipher1, err := aes.NewCipher(k1[:])
	if err != nil {
		return nil, err
	}
	cipher2, err := aes.NewCipher(k2[:])
	if err != nil {
		return nil, err
	}
	cipher3, err := aes.NewCipher(k3[:])
	if err != nil {
		return nil, err
	}
	return &PseudorandomCode{cipher0, cipher1, cipher2, cipher3}, nil
}

// PseudorandomCode ...
type PseudorandomCode struct {
	cipher0, cipher1, cipher2, cipher3 cipher.Block
}

// Encode ...
func (p *PseudorandomCode) Encode(dst *Block512, b Block) {
	p.cipher0.Encrypt(dst[0:16], b[:])
	p.cipher1.Encrypt(dst[16:32], b[:])
	p.cipher2.Encrypt(dst[32:48], b[:])
	p.cipher3.Encrypt(dst[48:64], b[:])
}
