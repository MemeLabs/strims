package kvtest

import (
	"fmt"
	"strings"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

// NewMemStore ...
func NewMemStore() kv.BlobStore {
	db := make(map[string]map[string][]byte)
	return &Store{store: db}
}

// Store ...
type Store struct {
	mu    sync.Mutex
	store map[string]map[string][]byte
}

// Close ...
func (s *Store) Close() error {
	return nil
}

// Dump ...
func (s *Store) Dump() map[string]map[string][]byte {
	return s.store
}

// CreateStoreIfNotExists ...
func (s *Store) CreateStoreIfNotExists(table string) error {
	s.mu.Lock()
	if _, ok := s.store[table]; !ok {
		s.store[table] = make(map[string][]byte)
	}
	s.mu.Unlock()
	return nil
}

// DeleteStore ...
func (s *Store) DeleteStore(table string) error {
	s.mu.Lock()
	delete(s.store, table)
	s.mu.Unlock()
	return nil
}

// View ...
func (s *Store) View(table string, fn func(tx kv.BlobTx) error) error {
	s.mu.Lock()
	b, ok := s.store[table]
	s.mu.Unlock()
	if !ok {
		return fmt.Errorf("bucket not found %s", table)
	}
	return fn(Tx{s, b})
}

// Update ...
func (s *Store) Update(table string, fn func(tx kv.BlobTx) error) error {
	s.mu.Lock()
	b, ok := s.store[table]
	s.mu.Unlock()
	if !ok {
		return fmt.Errorf("bucket not found %s", table)
	}
	return fn(Tx{s, b})
}

// Tx ...
type Tx struct {
	s *Store
	b map[string][]byte
}

// Put ...
func (t Tx) Put(key string, value []byte) error {
	t.s.mu.Lock()
	t.b[key] = value
	t.s.mu.Unlock()
	return nil
}

// Delete ...
func (t Tx) Delete(key string) error {
	t.s.mu.Lock()
	delete(t.b, key)
	t.s.mu.Unlock()
	return nil
}

// Get ...
func (t Tx) Get(key string) (value []byte, err error) {
	t.s.mu.Lock()
	val, ok := t.b[key]
	t.s.mu.Unlock()
	if !ok {
		return nil, kv.ErrRecordNotFound
	}
	return val, nil
}

// ScanPrefix ...
func (t Tx) ScanPrefix(prefix string) (values [][]byte, err error) {
	t.s.mu.Lock()
	for key, v := range t.b {
		if strings.HasPrefix(key, prefix) {
			values = append(values, append([]byte{}, v...))
		}
	}
	t.s.mu.Unlock()
	return values, nil
}
