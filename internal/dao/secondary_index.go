package dao

import (
	"bytes"
	"encoding/base64"
	"strconv"
	"strings"

	daov1 "github.com/MemeLabs/go-ppspp/pkg/apis/dao/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"golang.org/x/crypto/blake2b"
)

// SetSecondaryIndex ...
func SetSecondaryIndex(s kv.RWStore, ns namespace, key []byte, id uint64) error {
	var kb secondaryIndexKeyBuilder
	if err := kb.WriteNS(ns); err != nil {
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

// DeleteSecondaryIndex ...
func DeleteSecondaryIndex(s kv.RWStore, ns namespace, key []byte, id uint64) error {
	var kb secondaryIndexKeyBuilder
	if err := kb.WriteNS(ns); err != nil {
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

// ScanSecondaryIndex ...
func ScanSecondaryIndex(s kv.Store, ns namespace, key []byte) ([]uint64, error) {
	keys, err := scanSecondaryIndex(s, ns, key)
	if err != nil {
		return nil, err
	}

	ids := make([]uint64, 0, len(keys))
	for _, k := range keys {
		ids = append(ids, k.Id)
	}
	return ids, nil
}

func scanSecondaryIndex(s kv.Store, ns namespace, key []byte) ([]*daov1.SecondaryIndexKey, error) {
	var kb secondaryIndexKeyBuilder
	if err := kb.WriteNS(ns); err != nil {
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

func (b *secondaryIndexKeyBuilder) WriteNS(ns namespace) error {
	_, err := b.sb.WriteString(ns.String())
	if err != nil {
		return err
	}
	_, err = b.sb.WriteRune(':')
	if err != nil {
		return err
	}
	return err
}

func (b *secondaryIndexKeyBuilder) WriteKey(key []byte, s interface{}) error {
	h, err := hashSecondaryIndexKey(key, s)
	if err != nil {
		return err
	}

	if _, err := b.sb.WriteString(h); err != nil {
		return err
	}
	if _, err := b.sb.WriteRune(':'); err != nil {
		return err
	}
	return nil
}

func (b *secondaryIndexKeyBuilder) WriteID(id uint64) error {
	_, err := b.sb.WriteString(strconv.FormatUint(id, 36))
	return err
}

func (b *secondaryIndexKeyBuilder) String() string {
	return b.sb.String()
}

const secondaryIndexKeyHashSize = 8

func hashSecondaryIndexKey(key []byte, s interface{}) (string, error) {
	var salt []byte
	if s, ok := s.(Salter); ok {
		salt = s.Salt()
		if len(salt) > blake2b.Size {
			salt = salt[:blake2b.Size]
		}
	}

	h, err := blake2b.New(blake2b.Size256, salt)
	if err != nil {
		return "", err
	}
	if _, err := h.Write(key); err != nil {
		return "", err
	}

	b := h.Sum(nil)[:secondaryIndexKeyHashSize]
	return base64.RawStdEncoding.EncodeToString(b), nil
}

type secondaryKey struct {
	s   interface{}
	key []byte
}

func (f secondaryKey) String() string {
	h, _ := hashSecondaryIndexKey(f.key, f.s)
	return h
}
