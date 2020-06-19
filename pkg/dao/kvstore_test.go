package dao

import (
	"fmt"
	"strings"
)

// NewKVStore ...
func NewKVStore(path string) (*KVStore, error) {
	db := make(map[string]map[string][]byte)
	return &KVStore{store: db}, nil
}

// KVStore ...
type KVStore struct {
	store map[string]map[string][]byte
}

// CreateStoreIfNotExists ...
func (s *KVStore) CreateStoreIfNotExists(table string) error {
	if _, ok := s.store[table]; !ok {
		s.store[table] = make(map[string][]byte)
	}
	return nil
}

// DeleteStore ...
func (s *KVStore) DeleteStore(table string) error {
	delete(s.store, table)
	return nil
}

// View ...
func (s *KVStore) View(table string, fn func(tx BlobTx) error) error {
	b, ok := s.store[table]
	if !ok {
		return fmt.Errorf("bucket not found %s", table)
	}
	return fn(KVTx{b})
}

// Update ...
func (s *KVStore) Update(table string, fn func(tx BlobTx) error) error {
	b, ok := s.store[table]
	if !ok {
		return fmt.Errorf("bucket not found %s", table)
	}
	return fn(KVTx{b})
}

// KVTx ...
type KVTx struct {
	b map[string][]byte
}

// Put ...
func (t KVTx) Put(key string, value []byte) error {
	t.b[key] = value
	return nil
}

// Delete ...
func (t KVTx) Delete(key string) error {
	delete(t.b, key)
	return nil
}

// Get ...
func (t KVTx) Get(key string) (value []byte, err error) {
	val, ok := t.b[key]
	if !ok {
		return nil, ErrRecordNotFound
	}
	return val, nil
}

// ScanPrefix ...
func (t KVTx) ScanPrefix(prefix string) (values [][]byte, err error) {

	for key, v := range t.b {
		if strings.HasPrefix(key, prefix) {
			values = append(values, append([]byte{}, v...))
		}
	}

	return values, nil
}
