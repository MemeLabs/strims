package kvtest

import (
	"fmt"
	"math"
	"math/big"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/petar/GoLLRB/llrb"
)

type storeItem struct {
	key   string
	value []byte
}

func (i storeItem) Less(o llrb.Item) bool {
	return i.key < o.(storeItem).key
}

// NewMemStore ...
func NewMemStore() kv.BlobStore {
	db := make(map[string]*llrb.LLRB)
	return &Store{store: db}
}

// Store ...
type Store struct {
	mu    sync.Mutex
	store map[string]*llrb.LLRB
}

// Close ...
func (s *Store) Close() error {
	return nil
}

// Dump ...
func (s *Store) Dump() map[string]map[string][]byte {
	s.mu.Lock()
	store := map[string]map[string][]byte{}
	for k, v := range s.store {
		store[k] = map[string][]byte{}
		v.AscendLessThan(llrb.Inf(1), func(it llrb.Item) bool {
			store[k][it.(storeItem).key] = it.(storeItem).value
			return true
		})
	}
	s.mu.Unlock()
	return store
}

// CreateStoreIfNotExists ...
func (s *Store) CreateStoreIfNotExists(table string) error {
	s.mu.Lock()
	if _, ok := s.store[table]; !ok {
		s.store[table] = llrb.New()
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
	b *llrb.LLRB
}

// Put ...
func (t Tx) Put(key string, value []byte) error {
	t.s.mu.Lock()
	t.b.ReplaceOrInsert(storeItem{key, value})
	t.s.mu.Unlock()
	return nil
}

// Delete ...
func (t Tx) Delete(key string) error {
	t.s.mu.Lock()
	t.b.Delete(storeItem{key: key})
	t.s.mu.Unlock()
	return nil
}

// Get ...
func (t Tx) Get(key string) (value []byte, err error) {
	t.s.mu.Lock()
	it := t.b.Get(storeItem{key: key})
	t.s.mu.Unlock()
	if it == nil {
		return nil, kv.ErrRecordNotFound
	}
	return it.(storeItem).value, nil
}

// ScanPrefix ...
func (t Tx) ScanPrefix(prefix string) (values [][]byte, err error) {
	return t.ScanCursor(kv.Cursor{After: prefix, Before: prefix + "\uffff"})
}

var big1 = big.NewInt(1)

// ScanCursor ...
func (t Tx) ScanCursor(cursor kv.Cursor) (values [][]byte, err error) {
	t.s.mu.Lock()
	defer t.s.mu.Unlock()

	var limit int
	var boundFunc func(string) bool
	if cursor.Last == 0 {
		limit = cursor.First
		boundFunc = func(k string) bool { return k > cursor.Before }
	} else {
		limit = cursor.Last
		boundFunc = func(k string) bool { return k < cursor.After }
	}

	if limit == 0 {
		limit = math.MaxInt
	}

	iter := func(it llrb.Item) bool {
		if boundFunc(it.(storeItem).key) {
			return false
		}
		values = append(values, it.(storeItem).value)
		return len(values) < limit
	}

	if cursor.Last == 0 {
		t.b.AscendGreaterOrEqual(storeItem{key: cursor.After + "\u0000"}, iter)
	} else {
		before := big.NewInt(0).SetBytes([]byte(cursor.Before))
		before.Sub(before, big1)
		t.b.DescendLessOrEqual(storeItem{key: string(before.Bytes())}, iter)
	}

	return values, nil
}
