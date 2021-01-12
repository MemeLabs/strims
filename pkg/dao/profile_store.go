package dao

import (
	"errors"
	"sync"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"google.golang.org/protobuf/proto"
)

const profileIDReservationSize = 100
const profileIDKey = "id"

// NewProfileStore ...
func NewProfileStore(profileID uint64, store kv.BlobStore, key *StorageKey) *ProfileStore {
	return &ProfileStore{
		store: store,
		key:   key,
		name:  prefixProfileKey(profileID),
	}
}

// ProfileStore ...
type ProfileStore struct {
	store kv.BlobStore
	key   *StorageKey
	name  string

	idLock         sync.Mutex
	nextID         uint64
	lastReservedID uint64
}

// Init ...
func (s *ProfileStore) Init(profile *profilev1.Profile) error {
	if err := s.store.CreateStoreIfNotExists(s.name); err != nil {
		return err
	}
	return s.store.Update(s.name, func(tx kv.BlobTx) error {
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
func (s *ProfileStore) View(fn func(tx kv.Tx) error) error {
	return s.store.View(s.name, func(tx kv.BlobTx) error {
		return fn(&profileStoreTx{
			tx:       tx,
			sk:       s.key,
			readOnly: true,
		})
	})
}

// Update ...
func (s *ProfileStore) Update(fn func(tx kv.RWTx) error) error {
	return s.store.Update(s.name, func(tx kv.BlobTx) error {
		return fn(&profileStoreTx{
			tx: tx,
			sk: s.key,
		})
	})
}

// Salt ...
func (s *ProfileStore) Salt() []byte {
	return s.key.Key()
}

// GenerateID ...
func (s *ProfileStore) GenerateID() (uint64, error) {
	s.idLock.Lock()
	defer s.idLock.Unlock()

	if s.nextID < s.lastReservedID {
		id := s.nextID
		s.nextID++
		return id, nil
	}

	res := &profilev1.ProfileID{NextId: 1}
	err := s.Update(func(tx kv.RWTx) error {
		if err := tx.Get(profileIDKey, res); err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return err
		}
		return tx.Put(profileIDKey, &profilev1.ProfileID{NextId: res.NextId + profileIDReservationSize})
	})
	if err != nil {
		return 0, err
	}

	s.nextID = res.NextId + 1
	s.lastReservedID = res.NextId + profileIDReservationSize

	return res.NextId, nil
}

type profileStoreTx struct {
	tx       kv.BlobTx
	sk       *StorageKey
	readOnly bool
}

func (t *profileStoreTx) View(fn func(tx kv.Tx) error) error {
	return fn(t)
}

func (t *profileStoreTx) Update(fn func(tx kv.RWTx) error) error {
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

func (t *profileStoreTx) Salt() []byte {
	return t.sk.Key()
}
