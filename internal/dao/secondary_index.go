package dao

import (
	"bytes"
	"encoding/base64"

	daov1 "github.com/MemeLabs/go-ppspp/pkg/apis/dao/v1"
	"github.com/MemeLabs/go-ppspp/pkg/errutil"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"golang.org/x/crypto/blake2b"
)

const secondaryIndexKeyHashSize = 8

// Salter ...
type Salter interface {
	Salt() []byte
}

func hashSecondaryIndexKey(key []byte, s any) string {
	var salt []byte
	if s, ok := s.(Salter); ok {
		salt = s.Salt()
		if len(salt) > blake2b.Size {
			salt = salt[:blake2b.Size]
		}
	}

	h := errutil.Must(blake2b.New(secondaryIndexKeyHashSize, salt))

	h.Write(key)
	return base64.RawStdEncoding.EncodeToString(h.Sum(nil))
}

// SetSecondaryIndex ...
func SetSecondaryIndex(s kv.RWStore, ns namespace, key []byte, id uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Put(ns.Format(hashSecondaryIndexKey(key, s), id), &daov1.SecondaryIndexKey{Key: key, Id: id})
	})
}

// DeleteSecondaryIndex ...
func DeleteSecondaryIndex(s kv.RWStore, ns namespace, key []byte, id uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Delete(ns.Format(hashSecondaryIndexKey(key, s), id))
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
	return scanSecondaryIndexWithKey(s, key, ns.FormatPrefix(hashSecondaryIndexKey(key, s)))
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
