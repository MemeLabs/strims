package kvtest

import (
	"fmt"
	"strings"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

// NewMemStore ...
func NewMemStore(path string) (kv.BlobStore, error) {
	db := make(map[string]map[string][]byte)
	return &Store{store: db}, nil
}

// Store ...
type Store struct {
	store map[string]map[string][]byte
}

// CreateStoreIfNotExists ...
func (s *Store) CreateStoreIfNotExists(table string) error {
	if _, ok := s.store[table]; !ok {
		s.store[table] = make(map[string][]byte)
	}
	return nil
}

// DeleteStore ...
func (s *Store) DeleteStore(table string) error {
	delete(s.store, table)
	return nil
}

// View ...
func (s *Store) View(table string, fn func(tx kv.BlobTx) error) error {
	b, ok := s.store[table]
	if !ok {
		return fmt.Errorf("bucket not found %s", table)
	}
	return fn(Tx{b})
}

// Update ...
func (s *Store) Update(table string, fn func(tx kv.BlobTx) error) error {
	b, ok := s.store[table]
	if !ok {
		return fmt.Errorf("bucket not found %s", table)
	}
	return fn(Tx{b})
}

// Tx ...
type Tx struct {
	b map[string][]byte
}

// Put ...
func (t Tx) Put(key string, value []byte) error {
	t.b[key] = value
	return nil
}

// Delete ...
func (t Tx) Delete(key string) error {
	delete(t.b, key)
	return nil
}

// Get ...
func (t Tx) Get(key string) (value []byte, err error) {
	val, ok := t.b[key]
	if !ok {
		return nil, kv.ErrRecordNotFound
	}
	return val, nil
}

// ScanPrefix ...
func (t Tx) ScanPrefix(prefix string) (values [][]byte, err error) {

	for key, v := range t.b {
		if strings.HasPrefix(key, prefix) {
			values = append(values, append([]byte{}, v...))
		}
	}

	return values, nil
}
