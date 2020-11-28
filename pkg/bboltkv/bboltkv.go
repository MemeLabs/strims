package bboltkv

import (
	"bytes"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"go.etcd.io/bbolt"
)

// NewStore ...
func NewStore(path string) (*Store, error) {
	db, err := bbolt.Open(path, 0600, nil)
	if err != nil {
		return nil, err
	}

	return &Store{db: db}, nil
}

// Store ...
type Store struct {
	db *bbolt.DB
}

// CreateStoreIfNotExists ...
func (s *Store) CreateStoreIfNotExists(table string) error {
	return s.db.Update(func(tx *bbolt.Tx) error {
		_, err := tx.CreateBucketIfNotExists([]byte(table))
		return err
	})
}

// DeleteStore ...
func (s *Store) DeleteStore(table string) error {
	return s.db.Update(func(tx *bbolt.Tx) error {
		return tx.DeleteBucket([]byte(table))
	})
}

// View ...
func (s *Store) View(table string, fn func(tx kv.BlobTx) error) error {
	return s.db.View(func(tx *bbolt.Tx) error {
		b := tx.Bucket([]byte(table))
		if b == nil {
			return fmt.Errorf("bucket not found %s", table)
		}
		return fn(Tx{tx, b})
	})
}

// Update ...
func (s *Store) Update(table string, fn func(tx kv.BlobTx) error) error {
	return s.db.Update(func(tx *bbolt.Tx) error {
		b := tx.Bucket([]byte(table))
		if b == nil {
			return fmt.Errorf("bucket not found %s", table)
		}
		return fn(Tx{tx, b})
	})
}

// Tx ...
type Tx struct {
	tx *bbolt.Tx
	b  *bbolt.Bucket
}

// Put ...
func (t Tx) Put(key string, value []byte) error {
	return t.b.Put([]byte(key), value)
}

// Delete ...
func (t Tx) Delete(key string) error {
	return t.b.Delete([]byte(key))
}

// Get ...
func (t Tx) Get(key string) (value []byte, err error) {
	value = t.b.Get([]byte(key))
	if value == nil {
		return nil, kv.ErrRecordNotFound
	}
	return value, nil
}

// ScanPrefix ...
func (t Tx) ScanPrefix(prefix string) (values [][]byte, err error) {
	c := t.b.Cursor()

	p := []byte(prefix)
	for k, v := c.Seek(p); k != nil && bytes.HasPrefix(k, p); k, v = c.Next() {
		values = append(values, append(make([]byte, 0, len(v)), v...))
	}

	return values, nil
}
