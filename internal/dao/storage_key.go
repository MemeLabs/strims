// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha256"
	"errors"

	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"golang.org/x/crypto/pbkdf2"
	"google.golang.org/protobuf/proto"
)

// ErrUnsupportedKDFType ...
var ErrUnsupportedKDFType = errors.New("unsupported key derivation function")

const saltSize = 16
const pbkdf2Iterations = 100000
const KeySize = 32

// NewStorageKey ...
func NewStorageKey(password string) (*StorageKey, error) {
	options := &profilev1.StorageKey_PBKDF2Options{
		Iterations: pbkdf2Iterations,
		KeySize:    KeySize,
		Salt:       make([]byte, saltSize),
	}
	if _, err := rand.Read(options.Salt); err != nil {
		return nil, err
	}

	k := &profilev1.StorageKey{
		KdfType:    profilev1.KDFType_KDF_TYPE_PBKDF2_SHA256,
		KdfOptions: &profilev1.StorageKey_Pbkdf2Options{Pbkdf2Options: options},
	}
	return NewStorageKeyFromPassword(password, k)
}

// UnmarshalStorageKey ...
func UnmarshalStorageKey(b []byte, password string) (*StorageKey, error) {
	k := &profilev1.StorageKey{}
	if err := proto.Unmarshal(b, k); err != nil {
		return nil, err
	}
	return NewStorageKeyFromPassword(password, k)
}

func NewStorageKeyFromPassword(password string, k *profilev1.StorageKey) (*StorageKey, error) {
	switch k.KdfType {
	case profilev1.KDFType_KDF_TYPE_PBKDF2_SHA256:
		options := k.GetPbkdf2Options()
		key := pbkdf2.Key(
			[]byte(password),
			options.Salt,
			int(options.Iterations),
			int(options.KeySize),
			sha256.New,
		)
		return NewStorageKeyFromBytes(key, k)
	default:
		return nil, ErrUnsupportedKDFType
	}
}

// NewStorageKeyFromBytes ...
func NewStorageKeyFromBytes(key []byte, k *profilev1.StorageKey) (*StorageKey, error) {
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	cipher, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}

	return &StorageKey{
		record: k,
		key:    key,
		cipher: cipher,
	}, nil
}

// MarshalStorageKey ...
func MarshalStorageKey(k *StorageKey) ([]byte, error) {
	return proto.Marshal(k.record)
}

// StorageKey ...
type StorageKey struct {
	record *profilev1.StorageKey
	key    []byte
	cipher cipher.AEAD
}

// Key ...
func (k *StorageKey) Key() []byte {
	return k.key
}

// Seal ...
func (k *StorageKey) Seal(p []byte) ([]byte, error) {
	n := k.cipher.NonceSize()
	b := make([]byte, n, n+len(p)+k.cipher.Overhead())
	if _, err := rand.Read(b); err != nil {
		return nil, err
	}
	return k.cipher.Seal(b, b, p, nil), nil
}

// Open ...
func (k *StorageKey) Open(b []byte) ([]byte, error) {
	n := k.cipher.NonceSize()
	return k.cipher.Open(nil, b[:n], b[n:], nil)
}
