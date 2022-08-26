// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vnic

import (
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"errors"
	"fmt"
	"sync"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/pool"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"golang.org/x/crypto/curve25519"
)

func handshakeAESLink(l Link, k *key.Key, peerKey []byte) (Link, error) {
	var iv [16]byte
	if _, err := rand.Read(iv[:]); err != nil {
		return nil, fmt.Errorf("reading iv failed: %w", err)
	}

	err := protoutil.WriteStream(l, &vnicv1.AESLinkInit{
		ProtocolVersion: 1,
		Key:             k.Public,
		Iv:              iv[:],
	})
	if err != nil {
		return nil, fmt.Errorf("writing aes link init failed: %w", err)
	}

	var init vnicv1.AESLinkInit
	if err = protoutil.ReadStream(l, &init); err != nil {
		return nil, fmt.Errorf("reading aes link init failed: %w", err)
	}

	if peerKey != nil && !bytes.Equal(init.Key, peerKey) {
		return nil, errors.New("peer key mismatch")
	}

	key, err := curve25519.X25519(k.Private, init.Key)
	if err != nil {
		return nil, err
	}

	return newAESLink(l, key, iv[:], init.Iv)
}

func newAESLink(l Link, key, wiv, riv []byte) (Link, error) {
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}

	link := &aesLink{
		writeStream: cipher.NewCFBEncrypter(block, wiv),
		readStream:  cipher.NewCFBDecrypter(block, riv),
		Link:        l,
	}
	return link, nil
}

type aesLink struct {
	readStream  cipher.Stream
	writeLock   sync.Mutex
	writeStream cipher.Stream
	Link
}

func (c *aesLink) Read(p []byte) (int, error) {
	n, err := c.Link.Read(p)
	if err != nil {
		return 0, err
	}
	c.readStream.XORKeyStream(p[:n], p[:n])
	return n, nil
}

func (c *aesLink) Write(p []byte) (int, error) {
	b := pool.Get(len(p))
	defer pool.Put(b)

	c.writeLock.Lock()
	defer c.writeLock.Unlock()
	c.writeStream.XORKeyStream(*b, p)

	return c.Link.Write(*b)
}
