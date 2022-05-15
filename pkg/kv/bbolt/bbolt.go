// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bbolt

import (
	"bytes"
	"fmt"
	"math"

	"github.com/MemeLabs/strims/pkg/kv"
	bboltlib "go.etcd.io/bbolt"
)

// NewStore ...
func NewStore(path string) (kv.BlobStore, error) {
	db, err := bboltlib.Open(path, 0600, nil)
	if err != nil {
		return nil, err
	}

	return &Store{db: db}, nil
}

// Store ...
type Store struct {
	db *bboltlib.DB
}

// Close ...
func (s *Store) Close() error {
	return s.db.Close()
}

// CreateStoreIfNotExists ...
func (s *Store) CreateStoreIfNotExists(table string) error {
	return s.db.Update(func(tx *bboltlib.Tx) error {
		_, err := tx.CreateBucketIfNotExists([]byte(table))
		return err
	})
}

// DeleteStore ...
func (s *Store) DeleteStore(table string) error {
	return s.db.Update(func(tx *bboltlib.Tx) error {
		return tx.DeleteBucket([]byte(table))
	})
}

// View ...
func (s *Store) View(table string, fn func(tx kv.BlobTx) error) error {
	return s.db.View(func(tx *bboltlib.Tx) error {
		b := tx.Bucket([]byte(table))
		if b == nil {
			return fmt.Errorf("bucket not found %s", table)
		}
		return fn(Tx{tx, b})
	})
}

// Update ...
func (s *Store) Update(table string, fn func(tx kv.BlobTx) error) error {
	return s.db.Update(func(tx *bboltlib.Tx) error {
		b := tx.Bucket([]byte(table))
		if b == nil {
			return fmt.Errorf("bucket not found %s", table)
		}
		return fn(Tx{tx, b})
	})
}

// Tx ...
type Tx struct {
	tx *bboltlib.Tx
	b  *bboltlib.Bucket
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
	return t.ScanCursor(kv.Cursor{After: prefix, Before: prefix})
}

// ScanCursor ...
func (t Tx) ScanCursor(cursor kv.Cursor) (values [][]byte, err error) {
	c := t.b.Cursor()

	var limit int
	var k, v, pivot []byte
	var continueFunc func(*bboltlib.Cursor) (k, v []byte)
	var boundFunc func([]byte) bool
	if cursor.Last == 0 {
		limit = cursor.First
		pivot = []byte(cursor.After)
		continueFunc = (*bboltlib.Cursor).Next
		boundFunc = func(k []byte) bool { return bytes.Compare(k, []byte(cursor.Before)) > 0 }
	} else {
		limit = cursor.Last
		pivot = []byte(cursor.Before)
		continueFunc = (*bboltlib.Cursor).Prev
		boundFunc = func(k []byte) bool { return bytes.Compare(k, []byte(cursor.After)) < 0 }
	}

	k, v = c.Seek(pivot)
	if bytes.Equal(k, pivot) {
		k, v = continueFunc(c)
	}
	if limit == 0 {
		limit = math.MaxInt
	}

	for ; k != nil && !boundFunc(k); k, v = continueFunc(c) {
		value := make([]byte, len(v))
		copy(value, v)
		values = append(values, value)

		if len(values) >= limit {
			break
		}
	}

	return values, nil
}
