package dao

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha256"
	"errors"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"golang.org/x/crypto/pbkdf2"
	"google.golang.org/protobuf/proto"
)

// ErrUnsupportedKDFType ...
var ErrUnsupportedKDFType = errors.New("unsupported key derivation function")

const saltSize = 16
const pbkdf2Iterations = 100000
const keySize = 32

// NewStorageKey ...
func NewStorageKey(password string) (*StorageKey, error) {
	options := &profilev1.StorageKey_PBKDF2Options{
		Iterations: pbkdf2Iterations,
		KeySize:    keySize,
		Salt:       make([]byte, saltSize),
	}
	if _, err := rand.Read(options.Salt); err != nil {
		return nil, err
	}

	k := &StorageKey{
		record: profilev1.StorageKey{
			KdfType:    profilev1.KDFType_KDF_TYPE_PBKDF2_SHA256,
			KdfOptions: &profilev1.StorageKey_Pbkdf2Options{Pbkdf2Options: options},
		},
		key: pbkdf2.Key([]byte(password),
			options.Salt,
			int(options.Iterations),
			int(options.KeySize),
			sha256.New,
		),
	}
	return k, nil
}

// NewStorageKeyFromBytes ...
func NewStorageKeyFromBytes(key []byte) *StorageKey {
	return &StorageKey{key: key}
}

// UnmarshalStorageKey ...
func UnmarshalStorageKey(b []byte, password string) (*StorageKey, error) {
	k := &StorageKey{}
	if err := proto.Unmarshal(b, &k.record); err != nil {
		return nil, err
	}

	switch k.record.KdfType {
	case profilev1.KDFType_KDF_TYPE_PBKDF2_SHA256:
		options := k.record.GetPbkdf2Options()
		k.key = pbkdf2.Key([]byte(password),
			options.Salt,
			int(options.Iterations),
			int(options.KeySize),
			sha256.New,
		)
	default:
		return nil, ErrUnsupportedKDFType
	}

	return k, nil
}

// MarshalStorageKey ...
func MarshalStorageKey(k *StorageKey) ([]byte, error) {
	return proto.Marshal(&k.record)
}

// StorageKey ...
type StorageKey struct {
	record profilev1.StorageKey
	key    []byte
}

// Key ...
func (k *StorageKey) Key() []byte {
	return k.key
}

// Seal ...
func (k *StorageKey) Seal(p []byte) ([]byte, error) {
	block, err := aes.NewCipher(k.key[:])
	if err != nil {
		return nil, err
	}
	aesgcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	nonce := make([]byte, aesgcm.NonceSize())
	if _, err := rand.Read(nonce[:]); err != nil {
		return nil, err
	}

	b := make([]byte, 0, len(nonce)+len(p)+aesgcm.Overhead())
	b = append(b, nonce...)
	b = aesgcm.Seal(b, nonce[:], p, nil)

	return b, nil
}

// Open ...
func (k *StorageKey) Open(b []byte) ([]byte, error) {
	block, err := aes.NewCipher(k.key[:])
	if err != nil {
		return nil, err
	}
	aesgcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}

	nonce := make([]byte, aesgcm.NonceSize())
	n := copy(nonce, b)

	b, err = aesgcm.Open(nil, nonce, b[n:], nil)
	if err != nil {
		return nil, err
	}

	return b, nil
}
