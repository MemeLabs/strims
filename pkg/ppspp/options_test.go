// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"testing"

	"github.com/MemeLabs/strims/pkg/options"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
	"github.com/stretchr/testify/assert"
)

func TestSwarmOptions(t *testing.T) {
	tcs := map[string]struct {
		req      SwarmOptions
		expected SwarmOptions
	}{
		"default options": {
			req:      SwarmOptions{},
			expected: NewDefaultSwarmOptions(),
		},
		"custom options": {
			req: SwarmOptions{
				Label:              "test",
				ChunkSize:          2048,
				ChunksPerSignature: 16,
				StreamCount:        16,
				LiveWindow:         1 << 12,
				SchedulingMethod:   PeerSchedulingMethod,
				Integrity: integrity.VerifierOptions{
					ProtectionMethod:       integrity.ProtectionMethodSignAll,
					MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionSHA256,
					LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
				},
				DeliveryMode: MandatoryDeliveryMode,
				BufferLayout: store.CircularBufferLayout,
			},
			expected: SwarmOptions{
				Label:              "test",
				ChunkSize:          2048,
				ChunksPerSignature: 16,
				StreamCount:        16,
				LiveWindow:         1 << 12,
				SchedulingMethod:   PeerSchedulingMethod,
				Integrity: integrity.VerifierOptions{
					ProtectionMethod:       integrity.ProtectionMethodSignAll,
					MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionSHA256,
					LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
				},
				DeliveryMode: MandatoryDeliveryMode,
				BufferLayout: store.CircularBufferLayout,
			},
		},
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			assert.New(t).Equal(tc.expected, options.AssignDefaults(tc.req, NewDefaultSwarmOptions()))
		})
	}
}
