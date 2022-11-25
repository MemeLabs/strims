// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"sync"

	"github.com/MemeLabs/strims/internal/dao/versionvector"
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
)

func newCheckpointMap(cs []*replicationv1.Checkpoint) *checkpointMap {
	m := &checkpointMap{
		m: map[uint64]*replicationv1.Checkpoint{},
	}
	for _, c := range cs {
		m.Set(c)
	}
	return m
}

type checkpointMap struct {
	mu sync.Mutex
	m  map[uint64]*replicationv1.Checkpoint
}

func (m *checkpointMap) Set(c *replicationv1.Checkpoint) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if c.Deleted {
		delete(m.m, c.Id)
	} else {
		m.m[c.Id] = c
	}
}

func (m *checkpointMap) MinVersion() *daov1.VersionVector {
	m.mu.Lock()
	defer m.mu.Unlock()
	var v *daov1.VersionVector
	for _, c := range m.m {
		if v == nil {
			v = versionvector.Clone(c.Version)
		} else {
			versionvector.Downgrade(v, c.Version)
		}
	}
	if v == nil {
		return versionvector.New()
	}
	return v
}
