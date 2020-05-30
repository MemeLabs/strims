package main

import (
	"bytes"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"go.etcd.io/bbolt"
)

// NewKVStore ...
func NewKVStore(path string) (*KVStore, error) {
	db, err := bbolt.Open(path, 0600, nil)
	if err != nil {
		return nil, err
	}

	return &KVStore{db: db}, nil
}

// KVStore ...
type KVStore struct {
	db *bbolt.DB
}

// CreateStoreIfNotExists ...
func (s *KVStore) CreateStoreIfNotExists(table string) error {
	return s.db.Update(func(tx *bbolt.Tx) error {
		_, err := tx.CreateBucketIfNotExists([]byte(table))
		return err
	})
}

// DeleteStore ...
func (s *KVStore) DeleteStore(table string) error {
	return s.db.Update(func(tx *bbolt.Tx) error {
		return tx.DeleteBucket([]byte(table))
	})
}

// View ...
func (s *KVStore) View(table string, fn func(tx dao.Tx) error) error {
	return s.db.View(func(tx *bbolt.Tx) error {
		b := tx.Bucket([]byte(table))
		if b == nil {
			return fmt.Errorf("bucket not found %s", table)
		}
		return fn(KVTx{tx, b})
	})
}

// Update ...
func (s *KVStore) Update(table string, fn func(tx dao.Tx) error) error {
	return s.db.Update(func(tx *bbolt.Tx) error {
		b := tx.Bucket([]byte(table))
		if b == nil {
			return fmt.Errorf("bucket not found %s", table)
		}
		return fn(KVTx{tx, b})
	})
}

// KVTx ...
type KVTx struct {
	tx *bbolt.Tx
	b  *bbolt.Bucket
}

// Put ...
func (t KVTx) Put(key string, value []byte) error {
	return t.b.Put([]byte(key), value)
}

// Delete ...
func (t KVTx) Delete(key string) error {
	return t.b.Delete([]byte(key))
}

// Get ...
func (t KVTx) Get(key string) (value []byte, err error) {
	value = t.b.Get([]byte(key))
	if value == nil {
		return nil, dao.ErrRecordNotFound
	}
	return value, nil
}

// ScanPrefix ...
func (t KVTx) ScanPrefix(prefix string) (values [][]byte, err error) {
	c := t.b.Cursor()

	p := []byte(prefix)
	for k, v := c.Seek(p); k != nil && bytes.HasPrefix(k, p); k, v = c.Next() {
		values = append(values, append([]byte{}, v...))
	}

	return values, nil
}
