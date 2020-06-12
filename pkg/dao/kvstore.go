package dao

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
func (s *KVStore) View(table string, fn func(tx Tx) error) error {
	b := s.store[table]
	return fn(KVTx{b})
}

// Update ...
func (s *KVStore) Update(table string, fn func(tx Tx) error) error {
	b := s.store[table]
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
	return t.b[key], nil
}

// ScanPrefix ...
func (t KVTx) ScanPrefix(prefix string) (values [][]byte, err error) {
	return nil, nil
}
