// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"github.com/MemeLabs/strims/pkg/apis/type/swarm"
	"github.com/MemeLabs/strims/pkg/kv"
)

const (
	_ = iota + swarmNS
	swarmCacheMetaNS
	swarmCacheMetaKeyNS
	swarmCacheNS
)

var swarmCacheMetas = NewTable[swarm.CacheMeta](swarmCacheMetaNS, nil)

func FormatSwarmCacheMetaKey(id, salt []byte) []byte {
	b := make([]byte, len(id)+len(salt))
	copy(b, id)
	copy(b[len(id):], salt)
	return b
}

var swarmCacheMetasByKey = NewUniqueIndex(
	swarmCacheMetaKeyNS,
	swarmCacheMetas,
	func(m *swarm.CacheMeta) []byte {
		return FormatSwarmCacheMetaKey(m.SwarmId, m.SwarmSalt)
	},
	nil,
)

func SetSwarmCache(s *ProfileStore, id, salt []byte, c *swarm.Cache) error {
	m, err := swarmCacheMetasByKey.Get(s, FormatSwarmCacheMetaKey(id, salt))
	if err != nil {
		if err != kv.ErrRecordNotFound {
			return err
		}
		m = &swarm.CacheMeta{
			SwarmId:   id,
			SwarmSalt: salt,
		}
		m.Id, err = s.GenerateID()
		if err != nil {
			return err
		}
	}

	checksum := CRC32Message(c)
	if m.Checksum == checksum {
		return nil
	}
	m.Checksum = checksum

	return s.Update(func(tx kv.RWTx) error {
		if err := swarmCacheMetas.Upsert(tx, m); err != nil {
			return err
		}

		return tx.Put(swarmCacheNS.Format(m.Id), &swarm.Cache{
			Id:        m.Id,
			Uri:       c.Uri,
			Epoch:     c.Epoch,
			Integrity: c.Integrity,
			Data:      c.Data,
		})
	})
}

func GetSwarmCache(s *ProfileStore, id, salt []byte) (*swarm.Cache, error) {
	m, err := swarmCacheMetasByKey.Get(s, FormatSwarmCacheMetaKey(id, salt))
	if err != nil {
		return nil, err
	}

	var c swarm.Cache
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(swarmCacheNS.Format(m.Id), &c)
	})
	return &c, err
}

func DeleteSwarmCache(s *ProfileStore, id, salt []byte) error {
	m, err := swarmCacheMetasByKey.Get(s, FormatSwarmCacheMetaKey(id, salt))
	if err != nil {
		return err
	}

	return s.Update(func(tx kv.RWTx) error {
		if err := swarmCacheMetas.Delete(tx, m.Id); err != nil {
			return err
		}
		return tx.Delete(swarmCacheNS.Format(m.Id))
	})
}
