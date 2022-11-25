// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package replication

import (
	"testing"

	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/stretchr/testify/assert"
)

func TestEmptyCheckpointMapMinVersionIsNotNil(t *testing.T) {
	m := newCheckpointMap([]*replicationv1.Checkpoint{})
	assert.NotNil(t, m.MinVersion())
}

func TestCheckpointMapMinVersion(t *testing.T) {
	m := newCheckpointMap([]*replicationv1.Checkpoint{
		{
			Id: 0,
			Version: &daov1.VersionVector{
				Value: map[uint64]uint64{
					0: 1,
				},
			},
		},
		{
			Id: 1,
			Version: &daov1.VersionVector{
				Value: map[uint64]uint64{
					0: 2,
				},
			},
		},
	})

	assert.EqualValues(
		t,
		&daov1.VersionVector{
			Value: map[uint64]uint64{
				0: 1,
			},
		},
		m.MinVersion(),
	)
}
