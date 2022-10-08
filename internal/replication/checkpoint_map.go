// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"sync"

	"github.com/MemeLabs/strims/internal/dao/versionvector"
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"golang.org/x/exp/maps"
	"google.golang.org/protobuf/proto"
)

func newCheckpointMap(cs ...*replicationv1.Checkpoint) *checkpointMap {
	m := &checkpointMap{
		m: map[uint64]*replicationv1.Checkpoint{},
	}
	for _, c := range cs {
		m.m[c.Id] = c
	}
	return m
}

type checkpointMap struct {
	mu sync.Mutex
	m  map[uint64]*replicationv1.Checkpoint
}

func (m *checkpointMap) Get(replicaID uint64) *replicationv1.Checkpoint {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.m[replicaID]
}

func (m *checkpointMap) GetAll() []*replicationv1.Checkpoint {
	m.mu.Lock()
	defer m.mu.Unlock()
	return maps.Values(m.m)
}

func (m *checkpointMap) merge(c *replicationv1.Checkpoint) *replicationv1.Checkpoint {
	p, ok := m.m[c.Id]
	if ok {
		if proto.Equal(p, c) {
			return p
		}
	} else {
		p = c
	}

	p = proto.Clone(p).(*replicationv1.Checkpoint)
	versionvector.Upgrade(p.Version, c.Version)
	m.m[c.Id] = p
	return p
}

func (m *checkpointMap) Merge(c *replicationv1.Checkpoint) *replicationv1.Checkpoint {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.merge(c)
}

func (m *checkpointMap) MergeValue(id uint64, v *daov1.VersionVector) *replicationv1.Checkpoint {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.merge(&replicationv1.Checkpoint{
		Id:      id,
		Version: v,
	})
}

func (m *checkpointMap) MergeAll(cs []*replicationv1.Checkpoint) []*replicationv1.Checkpoint {
	m.mu.Lock()
	defer m.mu.Unlock()
	ps := make([]*replicationv1.Checkpoint, len(cs))
	for i, c := range cs {
		ps[i] = m.merge(c)
	}
	return ps
}
