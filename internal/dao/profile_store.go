// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"errors"
	"fmt"
	"reflect"
	"sync"

	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/options"
	"google.golang.org/protobuf/proto"
)

const profileIDReservationSize = 100

type ProfileStoreOptions struct {
	EventEmitter EventEmitter
}

// NewProfileStore ...
func NewProfileStore(profileID uint64, key *StorageKey, store kv.BlobStore, opt *ProfileStoreOptions) *ProfileStore {
	return &ProfileStore{
		store: store,
		key:   key,
		opt:   options.New(opt),
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

// BlobStore ...
func (s *ProfileStore) BlobStore() kv.BlobStore {
	return s.store
}

// Init ...
func (s *ProfileStore) Init() error {
	if err := s.store.CreateStoreIfNotExists(s.name); err != nil {
		return err
	}
	return storeVersion.Set(s, &daov1.StoreVersion{Version: CurrentVersion})
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
	b, err := proto.Marshal(m)
	if err != nil {
		return err
	}
	b, err = t.sk.Seal(b)
	if err != nil {
		return err
	}
	return t.tx.Put(key, b)
}

func (t *profileStoreTx) Get(key string, m proto.Message) error {
	b, err := t.tx.Get(key)
	if err != nil {
		return err
	}
	b, err = t.sk.Open(b)
	if err != nil {
		return err
	}
	return proto.Unmarshal(b, m)
}

func (t *profileStoreTx) Delete(key string) error {
	return t.tx.Delete(key)
}

func (t *profileStoreTx) ScanPrefix(prefix string, messages any) error {
	return t.ScanCursor(kv.Cursor{After: prefix, Prefix: prefix}, messages)
}

func (t *profileStoreTx) ScanCursor(cursor kv.Cursor, messages any) error {
	bs, err := t.tx.ScanCursor(cursor)
	if err != nil {
		return err
	}

	mv := reflect.ValueOf(messages).Elem()
	messages = mv.Interface()

	for _, b := range bs {
		b, err = t.sk.Open(b)
		if err != nil {
			return err
		}
		messages, err = t.appendUnmarshalled(messages, b)
		if err != nil {
			return err
		}
	}

	mv.Set(reflect.ValueOf(messages))

	return nil
}

func (t *profileStoreTx) appendUnmarshalled(messages any, bufs ...[]byte) (any, error) {
	mt := reflect.TypeOf(messages).Elem().Elem()
	mv := reflect.ValueOf(messages)
	for _, b := range bufs {
		m := reflect.New(mt)
		if err := proto.Unmarshal(b, m.Interface().(proto.Message)); err != nil {
			return nil, err
		}
		mv = reflect.Append(mv, m)
	}

	return mv.Interface(), nil
}

func (t *profileStoreTx) Salt() []byte {
	return t.sk.Key()
}

func (t *profileStoreTx) Emit(m proto.Message) {
	t.events = append(t.events, m)
}
