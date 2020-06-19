package dao

import (
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"google.golang.org/protobuf/proto"
)

// NewProfileStore ...
func NewProfileStore(profileID uint64, store BlobStore, key *StorageKey) *ProfileStore {
	return &ProfileStore{
		store: store,
		key:   key,
		name:  prefixProfileKey(profileID),
	}
}

// ProfileStore ...
type ProfileStore struct {
	store BlobStore
	key   *StorageKey
	name  string
}

// Init ...
func (s *ProfileStore) Init(profile *pb.Profile) error {
	if err := s.store.CreateStoreIfNotExists(s.name); err != nil {
		return err
	}
	return s.store.Update(s.name, func(tx BlobTx) error {
		b, err := MarshalStorageKey(s.key)
		if err != nil {
			return err
		}
		if err := tx.Put("key", b); err != nil {
			return err
		}

		if err := put(tx, s.key, "profile", profile); err != nil {
			return err
		}
		return nil
	})
}

// Delete ...
func (s *ProfileStore) Delete() error {
	return s.store.DeleteStore(s.name)
}

// Key ...
func (s *ProfileStore) Key() *StorageKey {
	return s.key
}

// View ...
func (s *ProfileStore) View(fn func(tx Tx) error) error {
	return s.store.View(s.name, func(tx BlobTx) error {
		return fn(&profileStoreTx{
			tx:       tx,
			sk:       s.key,
			readOnly: true,
		})
	})
}

// Update ...
func (s *ProfileStore) Update(fn func(tx Tx) error) error {
	return s.store.Update(s.name, func(tx BlobTx) error {
		return fn(&profileStoreTx{
			tx: tx,
			sk: s.key,
		})
	})
}

type profileStoreTx struct {
	tx       BlobTx
	sk       *StorageKey
	readOnly bool
}

func (t *profileStoreTx) View(fn func(tx Tx) error) error {
	return fn(t)
}

func (t *profileStoreTx) Update(fn func(tx Tx) error) error {
	if t.readOnly {
		return errors.New("cannot create read/write transaction from read only transaction")
	}
	return fn(t)
}

func (t *profileStoreTx) Put(key string, m proto.Message) error {
	return put(t.tx, t.sk, key, m)
}

func (t *profileStoreTx) Get(key string, m proto.Message) error {
	return get(t.tx, t.sk, key, m)
}

func (t *profileStoreTx) Delete(key string) error {
	return t.tx.Delete(key)
}

func (t *profileStoreTx) ScanPrefix(prefix string, messages interface{}) error {
	return scanPrefix(t.tx, t.sk, prefix, messages)
}
