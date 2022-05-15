// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vpn

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"errors"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/ed25519util"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/pool"
	"github.com/klauspost/compress/s2"
	"golang.org/x/crypto/curve25519"
)

func compressMessage(n *Network, m *Message, next networkMessageHandler) error {
	p := pool.Get(s2.MaxEncodedLen(len(m.Body)))
	defer pool.Put(p)
	b := s2.EncodeBest((*p)[:0], m.Body)

	if len(m.Body) > len(b) {
		m.Header.Length = uint16(len(b))
		m.Body = b
	} else {
		m.Header.Flags &^= Mcompress
	}

	return next(n, m)
}

func uncompressMessage(n *Network, m *Message, next networkMessageHandler) error {
	l, err := s2.DecodedLen(m.Body)
	if err != nil {
		return err
	}

	p := pool.Get(l)
	defer pool.Put(p)
	b, err := s2.Decode((*p)[:0], m.Body)
	if err != nil {
		return err
	}

	m.Header.Length = uint16(len(b))
	m.Body = b

	return next(n, m)
}

var errSignatureMismatch = errors.New("message signature mismatch")

func encryptMessage(n *Network, m *Message, next networkMessageHandler) error {
	c, err := newMessageCipher(n.VNIC().Key(), m.Header.DstID)
	if err != nil {
		return err
	}

	p := pool.Get(len(m.Body) + c.Overhead())
	defer pool.Put(p)
	b, err := c.Seal((*p)[:0], m.Body)
	if err != nil {
		return err
	}

	m.Header.Length = uint16(len(b))
	m.Body = b

	return next(n, m)
}

func decryptMessage(n *Network, m *Message, next networkMessageHandler) error {
	c, err := newMessageCipher(n.VNIC().Key(), m.Trailer.Entries[0].HostID)
	if err != nil {
		return err
	}
	p := pool.Get(len(m.Body))
	defer pool.Put(p)
	b, err := c.Open((*p)[:0], m.Body)
	if err != nil {
		return err
	}

	m.Header.Length = uint16(len(b))
	m.Body = b

	return next(n, m)
}

func newMessageCipher(k *key.Key, hostID kademlia.ID) (*messageCipher, error) {
	var ed25519Private [64]byte
	var ed25519Public [32]byte
	hostID.Bytes(ed25519Public[:])
	copy(ed25519Private[:], k.Private)

	var curve25519Private, curve25519Public [32]byte
	ed25519util.PrivateKeyToCurve25519(&curve25519Private, &ed25519Private)
	ed25519util.PublicKeyToCurve25519(&curve25519Public, &ed25519Public)

	secret, err := curve25519.X25519(curve25519Private[:], curve25519Public[:])
	if err != nil {
		return nil, err
	}

	block, err := aes.NewCipher(secret)
	if err != nil {
		return nil, err
	}
	aesgcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}

	return &messageCipher{
		cipher: aesgcm,
	}, nil
}

type messageCipher struct {
	cipher cipher.AEAD
}

func (t *messageCipher) Overhead() int {
	return t.cipher.NonceSize() + t.cipher.Overhead()
}

func (t *messageCipher) Seal(b, p []byte) ([]byte, error) {
	n := t.cipher.NonceSize()
	if _, err := rand.Read(b[:n]); err != nil {
		return nil, err
	}

	return t.cipher.Seal(b[:n], b[:n], p, nil), nil
}

func (t *messageCipher) Open(b, p []byte) ([]byte, error) {
	n := t.cipher.NonceSize()
	return t.cipher.Open(b, p[:n], p[n:], nil)
}
