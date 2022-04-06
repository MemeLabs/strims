package dao

import (
	"errors"
	"fmt"
	"sync"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"google.golang.org/protobuf/proto"
)

const profileIDReservationSize = 100

type ProfileStoreOptions struct {
	EventEmitter EventEmitter
}

// NewProfileStore ...
func NewProfileStore(profileID uint64, key *StorageKey, store kv.BlobStore, opt *ProfileStoreOptions) *ProfileStore {
	if opt == nil {
		opt = &ProfileStoreOptions{}
	}

	return &ProfileStore{
		store: store,
		key:   key,
		opt:   opt,
		name:  fmt.Sprintf("profile:%d", profileID),
	}
}

// ProfileStore ...
type ProfileStore struct {
	store kv.BlobStore
	key   *StorageKey
	opt   *ProfileStoreOptions
	name  string

	idLock         sync.Mutex
	nextID         uint64
	lastReservedID uint64
}

// Init ...
func (s *ProfileStore) Init() error {
	return s.store.CreateStoreIfNotExists(s.name)
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
	var ptx *profileStoreTx

	err := s.store.Update(s.name, func(tx kv.BlobTx) error {
		ptx = &profileStoreTx{
			tx: tx,
			sk: s.key,
		}
		return fn(ptx)
	})
	if err != nil {
		return err
	}

	if s.opt.EventEmitter != nil {
		for _, e := range ptx.events {
			s.opt.EventEmitter.Emit(e)
		}
	}

	return nil
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

	res, err := profileID.Transform(s, func(v *profilev1.ProfileID) error {
		v.NextId += profileIDReservationSize
		return nil
	})
	if err != nil {
		return 0, err
	}

	nextID := res.NextId - profileIDReservationSize

	s.nextID = nextID + 1
	s.lastReservedID = res.NextId

	return nextID, nil
}

type profileStoreTx struct {
	tx       kv.BlobTx
	sk       *StorageKey
	readOnly bool
	events   []proto.Message
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

func (t *profileStoreTx) ScanPrefix(prefix string, messages any) error {
	return scanPrefix(t.tx, t.sk, prefix, messages)
}

func (t *profileStoreTx) ScanCursor(cursor kv.Cursor, messages any) error {
	return scanCursor(t.tx, t.sk, cursor, messages)
}

func (t *profileStoreTx) Salt() []byte {
	return t.sk.Key()
}

func (t *profileStoreTx) Emit(m proto.Message) {
	t.events = append(t.events, m)
}
