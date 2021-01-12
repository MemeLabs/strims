package dao

import (
	"bytes"
	"encoding/base64"
	"errors"
	"strconv"
	"strings"

	daov1 "github.com/MemeLabs/go-ppspp/pkg/apis/dao/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"golang.org/x/crypto/blake2b"
)

// SetSecondaryIndex ...
func SetSecondaryIndex(s kv.RWStore, prefix string, key []byte, id uint64) error {
	var kb secondaryIndexKeyBuilder
	if err := kb.WritePrefix(prefix); err != nil {
		return err
	}
	if err := kb.WriteKey(key, s); err != nil {
		return err
	}
	if err := kb.WriteID(id); err != nil {
		return err
	}

	return s.Update(func(tx kv.RWTx) error {
		return tx.Put(kb.String(), &daov1.SecondaryIndexKey{Key: key, Id: id})
	})
}

// SetUniqueSecondaryIndex ...
func SetUniqueSecondaryIndex(s kv.RWStore, prefix string, key []byte, id uint64) error {
	var kb secondaryIndexKeyBuilder
	if err := kb.WritePrefix(prefix); err != nil {
		return err
	}
	if err := kb.WriteKey(key, s); err != nil {
		return err
	}

	return s.Update(func(tx kv.RWTx) error {
		keys, err := scanSecondaryIndexWithKey(s, key, kb.String())
		if err != nil {
			return err
		}
		if len(keys) == 1 {
			if keys[0].Id != id {
				return errors.New("duplicate key value violates unique constraint")
			}
			return nil
		}

		if err := kb.WriteID(id); err != nil {
			return err
		}
		return tx.Put(kb.String(), &daov1.SecondaryIndexKey{Key: key, Id: id})
	})
}

// DeleteSecondaryIndex ...
func DeleteSecondaryIndex(s kv.RWStore, prefix string, key []byte, id uint64) error {
	var kb secondaryIndexKeyBuilder
	if err := kb.WritePrefix(prefix); err != nil {
		return err
	}
	if err := kb.WriteKey(key, s); err != nil {
		return err
	}
	if err := kb.WriteID(id); err != nil {
		return err
	}

	return s.Update(func(tx kv.RWTx) error {
		return tx.Delete(kb.String())
	})
}

// GetUniqueSecondaryIndex ...
func GetUniqueSecondaryIndex(s kv.Store, prefix string, key []byte) (uint64, error) {
	keys, err := scanSecondaryIndex(s, prefix, key)
	if err != nil {
		return 0, err
	}
	if len(keys) == 0 {
		return 0, kv.ErrRecordNotFound
	}
	return keys[0].Id, nil
}

// ScanSecondaryIndex ...
func ScanSecondaryIndex(s kv.Store, prefix string, key []byte) ([]uint64, error) {
	keys, err := scanSecondaryIndex(s, prefix, key)
	if err != nil {
		return nil, err
	}

	ids := make([]uint64, 0, len(keys))
	for _, k := range keys {
		ids = append(ids, k.Id)
	}
	return ids, nil
}

func scanSecondaryIndex(s kv.Store, prefix string, key []byte) ([]*daov1.SecondaryIndexKey, error) {
	var kb secondaryIndexKeyBuilder
	if err := kb.WritePrefix(prefix); err != nil {
		return nil, err
	}
	if err := kb.WriteKey(key, s); err != nil {
		return nil, err
	}

	return scanSecondaryIndexWithKey(s, key, kb.String())
}

func scanSecondaryIndexWithKey(s kv.Store, key []byte, indexKeyPrefix string) ([]*daov1.SecondaryIndexKey, error) {
	candidates := []*daov1.SecondaryIndexKey{}
	err := s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(indexKeyPrefix, &candidates)
	})
	if err != nil {
		return nil, err
	}

	n := len(candidates)
	for i := 0; i < n; {
		if !bytes.Equal(key, candidates[i].Key) {
			if i+1 < n {
				candidates[i] = candidates[n-1]
			}
			n--
			continue
		}
		i++
	}
	return candidates[:n], nil
}

// Salter ...
type Salter interface {
	Salt() []byte
}

type secondaryIndexKeyBuilder struct {
	sb strings.Builder
}

func (b *secondaryIndexKeyBuilder) WritePrefix(prefix string) error {
	_, err := b.sb.WriteString(prefix)
	return err
}

func (b *secondaryIndexKeyBuilder) WriteKey(key []byte, s interface{}) error {
	var salt []byte
	if s, ok := s.(Salter); ok {
		salt = s.Salt()
		if len(salt) > blake2b.Size {
			salt = salt[:blake2b.Size]
		}
	}

	h, err := blake2b.New(blake2b.Size256, salt)
	if err != nil {
		return err
	}
	if _, err := h.Write(key); err != nil {
		return err
	}

	if _, err := b.sb.WriteString(base64.RawStdEncoding.EncodeToString(h.Sum(nil))); err != nil {
		return err
	}
	if _, err := b.sb.WriteRune(':'); err != nil {
		return err
	}
	return nil
}

func (b *secondaryIndexKeyBuilder) WriteID(id uint64) error {
	_, err := b.sb.WriteString(strconv.FormatUint(id, 10))
	return err
}

func (b *secondaryIndexKeyBuilder) String() string {
	return b.sb.String()
}
